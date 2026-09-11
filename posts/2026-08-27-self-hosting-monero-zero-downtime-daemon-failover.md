---
title: Self-Hosting Monero: Cakewallet to Hashvault Zero-Downtime Daemon Failover
short_title: Monero Daemon Failover
subtitle: Automating Payment Daemon Health Checks and Failover Routing
date: 2026-08-27
slug: self-hosting-monero-zero-downtime-daemon-failover
tags: crypto, monero, payments, routing, monitoring
---

# Self-Hosting Monero: Cakewallet to Hashvault Zero-Downtime Daemon Failover

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Accepting privacy-preserving cryptocurrency in production requires running dedicated full nodes without tolerating the unreliability inherent to peer-to-peer blockchain daemons. In our payment infrastructure, backend settlement services, internal wallet RPC workers, and mobile clients like Cakewallet communicate through a central Monero daemon endpoint.

Whenever our self-hosted `monerod` instance encountered memory pressure during heavy blockchain re-organizations or paused during disk-intensive LMDB database sync flushes, payment verification ground to an immediate halt. Unconfirmed transactions timed out, customer checkouts failed, and mobile wallets hung indefinitely during block scanning.

Relying entirely on remote third-party public nodes was unacceptable from both a privacy and a reliability perspective: public nodes log connecting IP addresses, rate-limit JSON-RPC queries, and frequently drop offline during network congestion. Conversely, relying strictly on a single self-hosted node created a single point of failure in our financial pipeline.

To solve this, we architected a transparent, zero-downtime failover proxy routing layer:
- **Tier 1 (Local Primary)**: Our dedicated bare-metal full node in Kansas City (`monero-kc-01`).
- **Tier 2 (Internal Standby)**: A secondary pruned node in Frankfurt (`monero-fra-01`) reached across our private WireGuard mesh.
- **Tier 3 (Public Fallback)**: An encrypted fallback to reputable public community nodes (such as Hashvault and Cakewallet) with strict client privacy scrubbing.

The fundamental engineering challenge was not routing the traffic, but detecting when a Monero daemon was actually dead.

---

## The Deep-Dive / Root Cause Analysis

Traditional Layer 4 load balancers and reverse proxies (HAProxy, Nginx, Envoy) evaluate backend health using simple TCP connection handshakes or generic HTTP 200 status codes. For a blockchain daemon like `monerod`, standard health checks are catastrophically misleading.

### The Deceptive TCP Listener

When `monerod` experiences severe LMDB lock contention or runs out of available worker threads during a block synchronization burst:
1. The daemon continues listening on TCP port `18081`.
2. A standard TCP health check (`check port 18081`) connects successfully and marks the node as 100% healthy.
3. However, when a client wallet issues an HTTP POST request to `/json_rpc`, the daemon queues the connection indefinitely or drops it after a ninety-second socket timeout.

```text
    [Client: Cakewallet]
            |
            | HTTP POST /json_rpc {"method": "get_balance"}
            v
    [Standard L4 Load Balancer] ---> (TCP Port 18081 Open? YES! Forwarding...)
            |
            v
    [monerod: Frozen in LMDB Sync]
            |
            x (Socket hangs for 90 seconds. Client times out. Checkout failed.)
```

### The Block-Lag Illusion

An even more insidious failure mode occurs when `monerod` is running smoothly, responding to HTTP requests with status `200 OK`, but has stalled its peer-to-peer gossip loop and lagged several thousand blocks behind the chain tip.

If a payment processing daemon asks an out-of-sync node whether a customer's transaction has been confirmed (`/get_transactions`), the lagging daemon checks its local database, finds no record of the transaction, and returns:

```json
{
  "status": "Failed",
  "untrusted": false,
  "txs_as_hex": []
}
```

The payment processor concludes the customer never sent the funds and cancels the order, despite the transaction already existing in the global mempool!

### Semantic Health Verification

A Monero node is only genuinely healthy if all four of the following conditions are simultaneously met:
1. HTTP endpoint `/get_info` returns status `200 OK`.
2. The JSON payload contains `"status": "OK"`.
3. The field `"synchronized"` is strictly `true`.
4. The difference between `"target_height"` and `"height"` is less than or equal to two blocks:

$$\Delta_{\text{height}} = |\text{target\_height} - \text{height}| \le 2$$

Standard proxies cannot parse nested JSON arithmetic out of the box without external helper daemons.

---

## The Implementation / Architecture

We built a resilient high-availability pipeline by coupling HAProxy 3.0 with a lightweight Python sentinel daemon running beside each `monerod` instance, feeding semantic health evaluations into HAProxy's health-check engine.

```text
+---------------------------------------------------------------------------------------------------+
|                                  MONERO HIGH-AVAILABILITY CLUSTER                                 |
+---------------------------------------------------------------------------------------------------+

     [Internal Wallet RPC]    [Payment Processor]    [Mobile Clients (Cakewallet)]
               |                      |                           |
               +----------------------+---------------------------+
                                      |
                                      v
                        [HAProxy Ingress: Port 18081]
                        - Semantic HTTP Health Checks
                        - Automated Failover Routing
                        - Privacy Header Stripping
                                      |
         +----------------------------+----------------------------+
         | (Priority 1: Weight 100)   | (Priority 2: Backup)       | (Priority 3: Emergency)
         v                            v                            v
+------------------+         +------------------+         +----------------------+
| Tier 1: Local KC |         | Tier 2: Standby  |         | Tier 3: Public Remote|
| monero-kc-01     |         | monero-fra-01    |         | nodes.hashvault.pro  |
| 10.200.0.50      |         | 10.200.0.51      |         | (SSL Verified)       |
| + Sentinel :9081 |         | + Sentinel :9081 |         | + Client IP Scrubbed |
+------------------+         +------------------+         +----------------------+
```

### 1. The Sentinel Health-Check Daemon

We deployed `monero-sentinel.py` as a systemd service on each node hosting `monerod`. It queries the local daemon every two seconds and serves an HTTP `200 OK` or `503 Service Unavailable` on port `9081`:

```python
#!/usr/bin/env python3
"""
monero-sentinel.py
Semantic health-check micro-daemon for Monero (monerod)
"""

import http.server
import json
import urllib.request
import sys

MONERO_RPC_URL = "http://127.0.0.1:18081/get_info"
MAX_BLOCK_LAG = 2

class SentinelHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/health":
            self.send_response(404)
            self.end_headers()
            return

        is_healthy, details = self.evaluate_daemon()
        if is_healthy:
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "healthy", "details": details}).encode())
        else:
            self.send_response(503)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "unhealthy", "reason": details}).encode())

    def evaluate_daemon(self):
        try:
            req = urllib.request.Request(MONERO_RPC_URL, headers={"User-Agent": "monero-sentinel"})
            with urllib.request.urlopen(req, timeout=1.5) as resp:
                if resp.status != 200:
                    return False, f"HTTP status {resp.status}"
                data = json.loads(resp.read().decode())

            status = data.get("status")
            synced = data.get("synchronized", False)
            height = data.get("height", 0)
            target = data.get("target_height", 0)

            if status != "OK":
                return False, f"Status reported: {status}"

            # Target height is 0 when node has no peers or is already at tip
            effective_target = target if target > 0 else height
            lag = effective_target - height

            if not synced and lag > MAX_BLOCK_LAG:
                return False, f"Not synced: height={height}, target={effective_target} (lag={lag})"

            return True, {"height": height, "target": effective_target, "lag": lag}
        except Exception as e:
            return False, f"Connection error: {str(e)}"

    def log_message(self, format, *args):
        # Suppress noisy standard request logging
        return

if __name__ == "__main__":
    server = http.server.HTTPServer(("0.0.0.0", 9081), SentinelHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.exit(0)
```

### 2. HAProxy Production Configuration

In `/etc/haproxy/haproxy.cfg`, HAProxy utilizes `monero-sentinel` on port `9081` for active probing while proxying client queries to `monerod` on port `18081`:

```haproxy
global
    log /dev/log local0 info
    maxconn 4096
    user haproxy
    group haproxy

defaults
    log global
    mode http
    option httplog
    option dontlognull
    timeout connect 3000ms
    timeout client  60000ms
    timeout server  60000ms

frontend monero_rpc_front
    bind 0.0.0.0:18081
    bind [::]:18081

    # Security: Limit maximum payload size for JSON-RPC
    http-request deny if { req.body_size gt 1048576 }

    default_backend monero_cluster_back

backend monero_cluster_back
    mode http
    balance roundrobin

    # Use HTTP health checks hitting the sentinel daemon
    option httpchk GET /health
    http-check expect status 200

    # Tier 1: Local Primary Node (Kansas City)
    server monero-kc 10.200.0.50:18081 check port 9081 inter 2000ms fall 2 rise 3 weight 100

    # Tier 2: Remote Standby Node (Frankfurt over WireGuard)
    server monero-fra 10.200.0.51:18081 check port 9081 inter 3000ms fall 2 rise 3 backup weight 80

    # Tier 3: Emergency Public Fallback (Hashvault)
    # Scrub headers before forwarding to external infrastructure
    http-request del-header X-Forwarded-For if { srv_is_up(monero-hashvault) }
    http-request del-header User-Agent if { srv_is_up(monero-hashvault) }
    http-request set-header Host nodes.hashvault.pro if { srv_is_up(monero-hashvault) }

    server monero-hashvault nodes.hashvault.pro:18081 ssl verify required ca-file /etc/ssl/certs/ca-certificates.crt check port 18081 inter 10000ms fall 3 rise 2 backup
```

### 3. Failover Validation in Practice

We performed a stress test by simulating a sudden database stall on the Kansas City primary using `kill -STOP`:

```bash
# Freeze the primary monerod process
$ sudo kill -STOP $(pgrep monerod)
```

The sentinel immediately reported connection timeouts. HAProxy's failover timeline:
1. **0.00s**: `monerod` process suspended.
2. **2.01s**: Sentinel probe 1 fails (`HTTP 503 Connection error`).
3. **4.02s**: Sentinel probe 2 fails. HAProxy marks `monero-kc` as `DOWN`.
4. **4.03s**: Next incoming payment RPC request from Cakewallet routes seamlessly to `monero-fra` in Frankfurt.
5. **Client Impact**: Zero dropped transactions. Handshake completed within 120ms.

```bash
# Resume the primary monerod process
$ sudo kill -CONT $(pgrep monerod)
```

Within six seconds (three successful rising checks), HAProxy smoothly restored `monero-kc` to active primary duty.

---

## Lessons Learned & Best Practices

1. **L4 TCP Health Checks Are Blind to Application State**: Never rely on port open checks for blockchain or database services. A listening port does not imply the engine is capable of servicing requests. Always validate application semantics.
2. **Block Lag Is a Hard Outage**: An out-of-sync node returning `status 200` with stale block heights is more dangerous than an outright crashed node. Stale nodes produce false-negative transaction lookups that corrupt financial state.
3. **Strip Identifying Headers on Third-Party Failover**: When failing over to community or public nodes, scrub `X-Forwarded-For` and client identifying headers in the proxy layer to prevent leaking client IP addresses.
4. **Tune Failover Rise/Fall Windows**: Set aggressive fall timers (`fall 2`, 2-second interval) for rapid failover during crashes, but conservative rise timers (`rise 3`, 6 seconds) to prevent flapping when a recovering node catches up on backlogged blocks.

---

## References

- [Monero Project Daemon RPC Documentation](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html)
- [HAProxy 3.0 Documentation: HTTP Health Checking and Backup Servers](https://docs.haproxy.org/3.0/configuration.html#4.2-option%20httpchk)
- [Cakewallet Mobile Monero Architecture & Node Connectivity](https://cakewallet.com/)
- [Hashvault Public Monero Node Infrastructure](https://nodes.hashvault.pro/)
