---
title: Bounding the Crypto Payment Window: Preventing Underpaid Invoice Races
short_title: Crypto Payment Grace Windows
subtitle: State Machine Isolation for High-Volatility Settlement
date: 2026-08-28
slug: bounding-crypto-payment-grace-windows
tags: crypto, payments, architecture, rust, security
---

# Bounding the Crypto Payment Window: Preventing Underpaid Invoice Races

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Accepting unconfirmed cryptocurrency transactions under high market volatility exposes checkout flows to insidious race conditions. When spot exchange rates fluctuate rapidly against fiat invoice targets, late-arriving or fractional mempool deposits frequently clash with invoice expiration timers, stranding payments in non-deterministic settlement states.

In our self-hosted infrastructure billing and domain registration platform, customers settle invoices using on-chain assets including Monero (XMR), Bitcoin (BTC), and USDT. The core payment gateway exposes fixed 15-minute checkout windows during which exchange rates are guaranteed by internal liquidity buffers. A recurring anomaly arose during periods of severe mempool congestion and market volatility: customers would submit transactions with insufficient miner fees right before invoice expiration, or broadcast partial payments across separate wallets minutes apart.

When an invoice timed out after 900 seconds, the checkout frontend marked the transaction expired and cancelled order provisioning. However, background blockchain daemons scanning mempools and newly minted blocks would subsequently detect incoming transactions matching the expired invoice subaddress. Because the naive worker processed incoming deposits against linear relational database rows using simple status flags (`pending`, `paid`, `expired`), edge cases surfaced:

1. **Underpaid-then-Expired**: A user sent 0.045 XMR on a 0.050 XMR invoice. The invoice expired before they sent the difference. Minutes later, the remaining 0.005 XMR arrived. The daemon marked the second deposit as received, but the invoice transition logic raised an uncaught state transition error, leaving customer funds credited on-chain but services unprovisioned.
2. **Expired-then-Overpaid**: A deposit arrived 12 seconds after expiration. The invoice was already marked `expired`, yet the gateway processed the deposit as an overpayment against a cancelled invoice, triggering automated refund sweeps with high transaction fee deductions.
3. **Concurrent Deposit Clashing**: When two inputs landed in the same block, concurrent daemon worker threads attempted simultaneous row updates in PostgreSQL, triggering serialization failures and deadlock rollbacks.

We needed a formally bounded settlement window with strict state machine isolation, delta-tracking accumulators, and idempotent reconciliation in async Rust.

---

## The Deep-Dive / Root Cause Analysis

Tracing the bug down to the database transaction isolation level revealed two architectural design flaws: loose temporal boundaries and mutable status flags without state invariants.

### The Linear Flag Anti-Pattern

Our legacy ledger used a single table column:
```sql
ALTER TABLE invoices ADD COLUMN status VARCHAR(32) DEFAULT 'pending';
```

When an invoice expired, a cron task executed:
```sql
UPDATE invoices SET status = 'expired' WHERE status = 'pending' AND expires_at < NOW();
```

Meanwhile, the blockchain monitor daemon polled wallet RPC endpoints every 5 seconds. Upon detecting a payment on a subaddress:
```sql
SELECT id, total_amount, paid_amount, status FROM invoices WHERE address = $1 FOR UPDATE;
```

The daemon calculated `new_paid_amount = paid_amount + incoming_tx.amount`. If `new_paid_amount >= total_amount`, it updated `status = 'paid'`.

The race condition materialized during the exact tick where the invoice expired while a deposit was in flight:

```text
Time    Payment Monitor Daemon               Cron Expiration Worker
 |
 t0     Detects Mempool TX (0.04 XMR)
 t1                                          Scans invoices: expires_at < NOW()
 t2                                          Locks row: status -> 'expired'
 t3     Locks row (waits for t2 commit)
 t4     Reads status == 'expired'
 t5     ERROR: cannot transition from 'expired' to 'partial_paid'
```

Because `expired` was treated as a terminal sink state, any subsequent deposit caused an unrecoverable exception. Even worse, if the cron job ran slightly late, a partial payment at $t = +2$ seconds could transition an expired invoice to `partially_paid`, extending its life indefinitely without re-quoting the exchange rate.

### Floating-Point Underflow in Satoshis and Piconero

A secondary vulnerability lay in fractional currency conversion. The original settlement code converted fiat amounts to crypto using 64-bit floating-point numbers (`f64`) before casting to atomic blockchain units (satoshis or piconero). Rounding inaccuracies resulted in dust deficits:

$$\text{Required: } 50{,}000{,}000{,}000 \text{ atomic units}$$
$$\text{Received: } 49{,}999{,}999{,}999 \text{ atomic units}$$

The invoice remained unfulfilled due to a 1-piconero shortfall ($10^{-12}$ XMR), triggering false partial-payment states that blocked automated checkout pipelines.

---

## The Implementation / Architecture

We replaced the fragile flag-based model with an explicit, statically typed Rust state machine utilizing Tokio, SQLx, and atomic fixed-point math.

```
                  +-------------------------------------------------+
                  |                                                 |
                  v                                                 |
            +------------+  tx detected   +-----------------+       |
            |   Draft    | -------------> | AwaitingDeposit |       |
            +------------+                +-----------------+       |
                                                   |                |
                       +---------------------------+                |
                       |                           |                |
             grace window expired          deposit received         |
                       |                           |                |
                       v                           v                |
               +---------------+           +---------------+        |
               | HardExpired   |           | PartialCredit | -------+
               +---------------+           +---------------+ (within grace)
                                                   |
                                            amount >= target
                                                   |
                                                   v
                                           +---------------+
                                           | Settled(Paid) |
                                           +---------------+
```

### Statically Typed Invoice State Machine

Using Rust's type system, invalid state transitions are made unrepresentable at compile time:

```rust
use std::time::{Duration, Instant, SystemTime};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicAmount(pub u64); // Fixed atomic units (e.g. satoshis, piconero)

#[derive(Debug, Clone)]
pub struct InvoiceConfig {
    pub window_duration: Duration,      // Primary quote window (e.g., 15 mins)
    pub grace_duration: Duration,       // Grace window for mempool propagation (e.g., 3 mins)
    pub underpayment_tolerance: u64,    // Allowable dust threshold in atomic units
}

#[derive(Debug, Clone)]
pub enum InvoiceState {
    AwaitingPayment {
        target_amount: AtomicAmount,
        received_amount: AtomicAmount,
        quote_expiry: SystemTime,
        grace_expiry: SystemTime,
    },
    PartiallyPaid {
        target_amount: AtomicAmount,
        received_amount: AtomicAmount,
        quote_expiry: SystemTime,
        grace_expiry: SystemTime,
        deposit_txids: Vec<String>,
    },
    Settled {
        target_amount: AtomicAmount,
        final_amount: AtomicAmount,
        settled_at: SystemTime,
        txids: Vec<String>,
    },
    UnderpaidExpired {
        target_amount: AtomicAmount,
        received_amount: AtomicAmount,
        expired_at: SystemTime,
        refundable_txids: Vec<String>,
    },
    HardExpired {
        expired_at: SystemTime,
    },
}

impl InvoiceState {
    pub fn process_deposit(
        self,
        txid: String,
        amount: AtomicAmount,
        now: SystemTime,
        config: &InvoiceConfig,
    ) -> Result<InvoiceState, &'static str> {
        match self {
            InvoiceState::AwaitingPayment {
                target_amount,
                received_amount,
                quote_expiry,
                grace_expiry,
            } => {
                if now > grace_expiry {
                    return Ok(InvoiceState::HardExpired { expired_at: grace_expiry });
                }

                let new_total = AtomicAmount(received_amount.0 + amount.0);
                if new_total.0 + config.underpayment_tolerance >= target_amount.0 {
                    Ok(InvoiceState::Settled {
                        target_amount,
                        final_amount: new_total,
                        settled_at: now,
                        txids: vec![txid],
                    })
                } else {
                    Ok(InvoiceState::PartiallyPaid {
                        target_amount,
                        received_amount: new_total,
                        quote_expiry,
                        grace_expiry,
                        deposit_txids: vec![txid],
                    })
                }
            }
            InvoiceState::PartiallyPaid {
                target_amount,
                received_amount,
                quote_expiry,
                grace_expiry,
                mut deposit_txids,
            } => {
                if now > grace_expiry {
                    return Ok(InvoiceState::UnderpaidExpired {
                        target_amount,
                        received_amount,
                        expired_at: grace_expiry,
                        refundable_txids: deposit_txids,
                    });
                }

                let new_total = AtomicAmount(received_amount.0 + amount.0);
                deposit_txids.push(txid);

                if new_total.0 + config.underpayment_tolerance >= target_amount.0 {
                    Ok(InvoiceState::Settled {
                        target_amount,
                        final_amount: new_total,
                        settled_at: now,
                        txids: deposit_txids,
                    })
                } else {
                    Ok(InvoiceState::PartiallyPaid {
                        target_amount,
                        received_amount: new_total,
                        quote_expiry,
                        grace_expiry,
                        deposit_txids,
                    })
                }
            }
            InvoiceState::Settled { .. } => {
                // Ignore idempotent redelivery or treat surplus as unallocated balance
                Ok(self)
            }
            InvoiceState::UnderpaidExpired { .. } | InvoiceState::HardExpired { .. } => {
                Err("Cannot credit deposits to expired invoice; route to refund pool")
            }
        }
    }
}
```

### Two-Tier Window Architecture

We partitioned the settlement timeline into two discrete phases:

1. **Quote Window (0–900s)**: The rate quote is locked. The frontend displays an active countdown. Users must broadcast their transaction before $t = 900$.
2. **Propagation Grace Window (900s–1080s)**: The frontend displays "Verifying Network Broadcast...". No new quotes can be accepted, but transactions detected in the mempool that bear timestamps or locktimes prior to $t = 900$ are permitted to settle without penalty.
3. **Hard Boundary ($t > 1080s$)**: The subaddress is unlinked from the active listener. Any late transaction is routed directly to an isolated cold refund pool with zero interaction with the order pipeline.

```sql
-- Atomic ledger insert with row-level advisory lock
CREATE OR REPLACE FUNCTION record_invoice_deposit(
    p_invoice_id UUID,
    p_txid TEXT,
    p_amount BIGINT,
    p_now TIMESTAMPTZ
) RETURNS JSONB AS $$
DECLARE
    v_invoice RECORD;
    v_new_paid BIGINT;
BEGIN
    -- Acquire exclusive advisory lock keyed on invoice UUID hash
    PERFORM pg_advisory_xact_lock(hashtext(p_invoice_id::text));

    SELECT * INTO v_invoice FROM crypto_invoices WHERE id = p_invoice_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Invoice % not found', p_invoice_id;
    END IF;

    IF v_invoice.state = 'SETTLED' THEN
        RETURN jsonb_build_object('status', 'ALREADY_SETTLED');
    END IF;

    IF p_now > v_invoice.grace_expires_at THEN
        INSERT INTO orphaned_deposits (invoice_id, txid, amount, received_at)
        VALUES (p_invoice_id, p_txid, p_amount, p_now);
        RETURN jsonb_build_object('status', 'ROUTED_TO_REFUND');
    END IF;

    v_new_paid := v_invoice.amount_paid + p_amount;

    IF v_new_paid + v_invoice.dust_tolerance >= v_invoice.amount_required THEN
        UPDATE crypto_invoices
        SET amount_paid = v_new_paid,
            state = 'SETTLED',
            settled_at = p_now,
            updated_at = p_now
        WHERE id = p_invoice_id;
        RETURN jsonb_build_object('status', 'SETTLED', 'total_paid', v_new_paid);
    ELSE
        UPDATE crypto_invoices
        SET amount_paid = v_new_paid,
            state = 'PARTIALLY_PAID',
            updated_at = p_now
        WHERE id = p_invoice_id;
        RETURN jsonb_build_object('status', 'PARTIALLY_PAID', 'remaining', v_invoice.amount_required - v_new_paid);
    END IF;
END;
$$ LANGUAGE plpgsql;
```

---

## Lessons Learned & Best Practices

1. **Eliminate Floating-Point in Settlement Math**: Never perform fiat-to-crypto rate conversions using `f32` or `f64`. Use fixed-point integer types like `rust_decimal::Decimal` or integer units (satoshis, piconero, wei) exclusively.
2. **Dust Tolerances Prevent Customer Friction**: High gas fees or exchange withdrawal fees frequently shave off 1–10 satoshis. Configure an acceptable underpayment epsilon (e.g., $0.05 USD equivalent) where invoices are marked settled rather than wedging the customer in an underpaid loop.
3. **Separate Quote Expiration from Payment Processing**: A quote may expire in 15 minutes, but block inclusion latency is governed by miners. The grace window isolates market volatility risk from peer-to-peer gossip latency.
4. **Idempotent Refund Queues**: Late deposits must never mutate order records. Route out-of-bounds funds directly into an unallocated deposit table keyed by cryptographic transaction hashes.

---

## References

- [Monero Payment Gateway Architecture & Subaddresses](https://web.getmonero.org/resources/developer-guides/)
- [PostgreSQL Explicit Locking & Advisory Locks](https://www.postgresql.org/docs/current/explicit-locking.html)
- [Rust Decimal: Precision Crate for Financial Calculations](https://docs.rs/rust_decimal/)
