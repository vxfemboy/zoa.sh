---
title: PostgreSQL Failover: Hunting the Dual-Writer Split-Brain Window
short_title: Postgres Split-Brain Hunt
subtitle: Surviving Container IP Aliasing and Standby Promotion
date: 2026-08-18
slug: postgresql-failover-hunting-dual-writer-split-brain
tags: postgres, database, incus, high-availability, sysadmin
---

# PostgreSQL Failover: Hunting the Dual-Writer Split-Brain Window

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

During an automated maintenance failover between physical bare-metal nodes, an unexpected container networking race condition opened an eighty-three-second dual-writer window across our primary and standby database instances. Writes continued landing on the supposedly demoted primary while the newly promoted replica minted a divergent timeline, plunging our core PostgreSQL cluster into a textbook split-brain crisis.

In our autonomous system architecture, stateful services run inside lightweight Incus system containers spread across physical hosts in Kansas City (`pve-kc-01`) and New York (`pve-ny-01`). The database cluster powers our custom API backends, subscriber billing ledgers, and authoritative DNS record state. Application services communicate with the database via a floating overlay service IP (`10.99.0.50`) exposed through container IP aliasing and routed across our internal WireGuard mesh.

On August 18th, we scheduled a rolling kernel upgrade on the Kansas City host. The plan called for an automated failover: promote the New York standby container (`db-standby-ny`), reassign the overlay service IP, and then reboot `pve-kc-01`.

The failover automation triggered as expected:
1. The New York standby issued `pg_ctl promote`.
2. BIRD on `pve-ny-01` announced the overlay service IP `10.99.0.50/32` across the internal iBGP mesh.
3. The host reboot command was issued to `pve-kc-01`.

However, the shutdown sequence on `pve-kc-01` encountered a delayed systemd unmount on a separate storage pool. During this delayed shutdown, the old primary container (`db-primary-kc`) remained alive and accessible to local backend workers running on the same host. Because those workers communicated over a local bridge interface (`incusbr0`) rather than traversing the external BGP mesh, they continued executing `INSERT` and `UPDATE` transactions directly against `db-primary-kc`.

Simultaneously, edge API proxies in Frankfurt and New York updated their routing tables to point at the newly promoted `db-standby-ny`, successfully executing write queries against it.

For eighty-three agonizing seconds, two PostgreSQL instances acted as writable primaries simultaneously. When `pve-kc-01` finally completed its reboot cycle and attempted to reconnect `db-primary-kc` as a standby to New York, PostgreSQL aborted with a catastrophic error:

```text
2026-08-18T04:12:31.402Z [41829] FATAL: timeline 2 is not a child of timeline 1
2026-08-18T04:12:31.403Z [41829] LOG: could not receive data from WAL stream: FATAL: requested starting point 0/1F900000 is on different timeline
2026-08-18T04:12:31.404Z [41829] LOG: startup process exited with exit code 1
2026-08-18T04:12:31.405Z [41828] LOG: aborting startup due to startup process failure
```

The database had branched into two conflicting universes.

---

## The Deep-Dive / Root Cause Analysis

To resolve the divergence without losing customer transactions, we had to dissect PostgreSQL's write-ahead log (WAL) timeline architecture and understand how container network namespaces masked the split-brain.

### The Anatomy of a Timeline Fork

PostgreSQL enforces data integrity using **Timelines**. Whenever a standby is promoted to a read-write primary, it increments its timeline counter (from Timeline 1 to Timeline 2) and records the Log Sequence Number (LSN) where the fork occurred in a history file (`00000002.history`):

```text
       Primary KC (Timeline 1)
       ... ---> [LSN: 0/1F8A9200] ----> [LSN: 0/1F8BA410] ----> [LSN: 0/1F900000] (14 Local Writes)
                      |
                      | Standby Promoted (pg_ctl promote)
                      v
       Standby NY (Timeline 2)
                [LSN: 0/1F8A9200] ----> [LSN: 0/1F8CA000] ----> [LSN: 0/1F954000] (312 Edge Writes)
```

At LSN `0/1F8A9200`, New York severed replication and became Timeline 2. But Kansas City was never fenced! During the 83-second shutdown delay:
- Kansas City generated **14 transactions** on Timeline 1 past the fork point.
- New York generated **312 transactions** on Timeline 2 past the fork point.

Because Timeline 1 had written data past `0/1F8A9200`, Kansas City could not simply rewind its clock or become a streaming replica. PostgreSQL's storage engine guarantees write atomicity by refusing to overwrite pages if the local timeline contains uncommitted or conflicting WAL records.

### Inspecting Divergent Records with `pg_waldump`

We dumped the WAL segments on Kansas City starting from the branch point to discover what transactions were committed in isolation:

```bash
pg_waldump -p /var/lib/postgresql/16/main/pg_wal \
  -s 0/1F8A9200 -e 0/1F900000 00000001000000000000001F
```

The output revealed the exact nature of the 14 orphaned transactions:

```text
rmgr: Heap        len(rec):       78, tg: 0, desc: INSERT ... rel 16384/16385/16410 (billing_events)
rmgr: Transaction len(rec):       34, tg: 0, desc: COMMIT 2026-08-18 04:11:15.112 UTC
rmgr: Heap        len(rec):       82, tg: 0, desc: UPDATE ... rel 16384/16385/16422 (auth_tokens)
rmgr: Transaction len(rec):       34, tg: 0, desc: COMMIT 2026-08-18 04:11:18.420 UTC
```

Twelve of the transactions were ephemeral session token updates, but two were critical automated billing ledger entries. If we had blindly wiped Kansas City's data directory with a fresh `pg_basebackup`, those financial transactions would have vanished without a trace.

### The Container IP Aliasing Trap

Why did Kansas City receive writes after New York took over the service IP?
1. The service IP `10.99.0.50` was configured as a secondary IP on the container's `eth0` via `systemd-networkd`.
2. When BIRD on New York announced `10.99.0.50/32` via BGP, the global mesh updated properly.
3. However, on host `pve-kc-01`, local routing table lookup rules prioritize `local` interface table lookups over external WireGuard interface tables.
4. As long as `db-primary-kc`'s virtual interface was physically up, any service residing on `pve-kc-01` routed packets directly into the local container, completely ignoring the BGP withdrawal!

---

## The Implementation / Architecture

Resolving this incident required a precise surgical procedure: extracting the orphaned transactions from the old primary, rolling its data directory back to the exact point of divergence using `pg_rewind`, replaying the extracted data onto the active primary, and implementing a hard STONITH fencing mechanism.

```text
+---------------------------------------------------------------------------------------+
|                               FENCING & PROMOTION FLOW                                |
+---------------------------------------------------------------------------------------+
  
      [Failover Coordinator]
                 |
                 | 1. Hard Fence (Incus Force Stop + Null Route)
                 v
      +----------------------+
      |   Old Primary (kc)   | ----> Container FROZEN / Network CUT
      +----------------------+
                 |
                 | 2. Verify Zero Traffic & Standby Catchup
                 v
      +----------------------+
      |   New Primary (ny)   | ----> pg_ctl promote (Timeline 2 Minted)
      +----------------------+
                 |
                 | 3. pg_rewind & Re-attach
                 v
      +----------------------+
      | Rebuilt Standby (kc) | <---- Streaming replication on Timeline 2
      +----------------------+
```

### 1. Data Recovery and Timeline Reconciliation

First, we prevented any further corruption by stopping the Kansas City instance:

```bash
incus exec db-primary-kc -- systemctl stop postgresql
```

Next, we brought up Kansas City on an isolated loopback port (5433) in read-only mode to extract the orphaned billing records using custom SQL:

```sql
-- Extracted from Kansas City (Timeline 1)
COPY (
    SELECT id, account_id, amount_cents, currency, event_type, created_at 
    FROM billing_events 
    WHERE created_at >= '2026-08-18 04:11:00'
) TO '/tmp/orphaned_billing_events.csv' WITH CSV HEADER;
```

We verified that the IDs did not collide with any records inserted on New York, and inserted them directly into New York (`db-standby-ny`).

### 2. Rewinding Timeline 1 with `pg_rewind`

Instead of transferring 400 GB of base backup across the transatlantic WAN link, we used PostgreSQL's built-in `pg_rewind` utility. `pg_rewind` finds the divergence point (`0/1F8A9200`), copies only the changed blocks from the source server (New York), and aligns the WAL control data to Timeline 2:

```bash
incus exec db-primary-kc -- sudo -u postgres pg_rewind \
  --target-pgdata=/var/lib/postgresql/16/main \
  --source-server="host=10.200.0.20 port=5432 user=replicator password=REDACTED_SECRET dbname=postgres sslmode=require" \
  --progress
```

The output confirmed clean synchronization:

```text
pg_rewind: connected to server
pg_rewind: servers diverged at WAL location 0/1F8A9200 on timeline 1
pg_rewind: rewinding from last common checkpoint at 0/1F8A9150 on timeline 1
pg_rewind: reading source file list
pg_rewind: reading target file list
pg_rewind: need to copy 84 MB of data
84/84 MB (100%) copied
pg_rewind: creating backup label and setting timeline to 2
pg_rewind: SUCCESS
```

Next, we signaled PostgreSQL to start as a standby by generating the signal file and configuring `primary_conninfo`:

```bash
incus exec db-primary-kc -- bash -c "
  touch /var/lib/postgresql/16/main/standby.signal
  cat <<EOF >> /var/lib/postgresql/16/main/postgresql.auto.conf
primary_conninfo = 'host=10.200.0.20 port=5432 user=replicator password=REDACTED_SECRET application_name=db_kc sslmode=require'
primary_slot_name = 'standby_kc_slot'
EOF
  chown postgres:postgres /var/lib/postgresql/16/main/standby.signal
  systemctl start postgresql
"
```

Within seconds, Kansas City attached to New York, replayed WAL along Timeline 2, and achieved zero replication lag.

### 3. Bulletproof STONITH Fencing Script for Incus

To guarantee that a dual-writer condition can never reoccur, we created a strict fencing and promotion orchestrator (`pg-fence-and-promote.sh`). Standbys are strictly forbidden from promoting until the old primary has been physically cut off:

```bash
#!/usr/bin/env bash
set -euo pipefail

# pg-fence-and-promote.sh
# Hard-fencing STONITH failover for Incus PostgreSQL clusters

OLD_PRIMARY_HOST="pve-kc-01"
OLD_CONTAINER="db-primary-kc"
NEW_PRIMARY_CONTAINER="db-standby-ny"
OVERLAY_IP="10.99.0.50"

echo "=== [Phase 1] Hard Fencing Old Primary ==="
echo "Issuing forced container stop on ${OLD_PRIMARY_HOST}:${OLD_CONTAINER}..."

# Force terminate container execution immediately
ssh -o ConnectTimeout=3 "root@${OLD_PRIMARY_HOST}" \
    "incus stop ${OLD_CONTAINER} --force || true"

# Null-route the container IP on the host bridge as a secondary safety net
ssh -o ConnectTimeout=3 "root@${OLD_PRIMARY_HOST}" \
    "ip route replace blackhole ${OVERLAY_IP}/32 || true"

echo "Fencing verification: confirming container state is STOPPED..."
CONTAINER_STATE=$(ssh "root@${OLD_PRIMARY_HOST}" \
    "incus info ${OLD_CONTAINER} | grep -E '^Status:' | awk '{print \$2}'")

if [[ "${CONTAINER_STATE}" != "STOPPED" ]]; then
    echo "CRITICAL: Fencing failed! Container status is ${CONTAINER_STATE}. Aborting promotion!" >&2
    exit 1
fi
echo "Old primary successfully fenced and offline."

echo ""
echo "=== [Phase 2] Promoting Standby to Read-Write Primary ==="
incus exec "${NEW_PRIMARY_CONTAINER}" -- sudo -u postgres pg_ctl promote -D /var/lib/postgresql/16/main

echo "Verifying read-write status on ${NEW_PRIMARY_CONTAINER}..."
IS_STANDBY=$(incus exec "${NEW_PRIMARY_CONTAINER}" -- sudo -u postgres psql -t -A -c "SELECT pg_is_in_recovery();")

if [[ "${IS_STANDBY}" != "f" ]]; then
    echo "CRITICAL: Promotion failed! Database is still in recovery." >&2
    exit 1
fi
echo "Standby successfully promoted to Primary (pg_is_in_recovery = false)."

echo ""
echo "=== [Phase 3] Re-binding Overlay Service IP ==="
incus exec "${NEW_PRIMARY_CONTAINER}" -- ip addr add "${OVERLAY_IP}/32" dev eth0 || true
birdc configure
echo "BGP route converged. Failover completed safely without split-brain."
```

### 4. Systemd Pre-Start Lease Protection

To protect against inadvertent reboot restarts before fencing is cleared, we added a drop-in override (`/etc/systemd/system/postgresql@.service.d/override.conf`):

```ini
[Service]
# Ensure node cannot start without verifying it is not a stale unassigned primary
ExecStartPre=/usr/local/bin/pg-verify-cluster-lease.sh
```

Where `pg-verify-cluster-lease.sh` validates that if `standby.signal` is absent, the host must possess the active leadership lock on our consensus ledger.

---

## Lessons Learned & Best Practices

1. **Never Promote Without a Hard Fence (STONITH)**: Hope is not an architectural strategy. Standbys must never promote based solely on a lost heartbeat. The coordinator must actively kill the old primary (`incus stop --force`) and verify confirmation before promoting.
2. **Local Kernel Routing Bypasses BGP Withdrawals**: If client applications run on the same physical host as a container, local kernel routing tables take precedence over overlay BGP mesh routes. Withdrawing a BGP route does not stop local intra-host traffic.
3. **Master `pg_rewind` Before Disaster Strikes**: When a split-brain happens, `pg_rewind` is the difference between a 30-second resync and hours of downtime streaming a massive multi-terabyte `pg_basebackup`.
4. **Audit WAL Timelines Immediately**: When divergence occurs, never panic and never discard WAL logs. Use `pg_waldump` to isolate the divergence point and check for orphan committed transactions before re-aligning timelines.

---

## References

- [PostgreSQL Documentation: Continuous Archiving and Point-in-Time Recovery (PITR)](https://www.postgresql.org/docs/16/continuous-archiving.html)
- [PostgreSQL Documentation: pg_rewind Internals](https://www.postgresql.org/docs/16/app-pgrewind.html)
- [Linux Kernel IP Routing & Local Table Lookup Precedence](https://man7.org/linux/man-pages/man8/ip-route.8.html)
- [Incus Documentation: Instance Lifecycle and Systemd Integration](https://linuxcontainers.org/incus/docs/main/)
