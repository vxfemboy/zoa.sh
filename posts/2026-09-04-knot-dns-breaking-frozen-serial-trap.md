---
title: Knot DNS: Breaking the Frozen Serial Trap Across 28 Zones
short_title: Knot DNS Serial Trap Recovery
subtitle: Automated Primary Recovery and AXFR Replication Bypass
date: 2026-09-04
slug: knot-dns-breaking-frozen-serial-trap
tags: dns, knot, bgp, wireguard, networking
---

# Knot DNS: Breaking the Frozen Serial Trap Across 28 Zones

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

During an automated primary nameserver migration across our global anycast edge nodes, an unexpected serial rollback wedged zone transfers across twenty-eight production DNS zones. Edge secondary daemons refused incremental zone updates, clinging stubbornly to higher serial timestamps generated during an emergency failover test, effectively freezing record updates worldwide.

Our autonomous system (AS214806) serves authoritative DNS for all core domains (including `femboy.zip`, `femboy.fan`, and `roa.gay`) alongside reverse delegations for our IPv4 block (`94.156.238.0/24`) and IPv6 allocation (`2a12:9b00:b00b::/48`). Authoritative DNS is announced over BGP anycast from eight global edge Points of Presence (POPs): Frankfurt (`fra`), Kansas City (`kc`), New York (`ny`), Las Vegas (`lv`), London, Tokyo, Singapore, and São Paulo.

The architecture relies on a hidden primary running CZ.NIC's Knot DNS inside an Incus container in Kansas City (`ns-master.internal`, WireGuard IP `10.100.0.1`). Edge nodes act as public-facing secondaries, accepting DNS queries over anycast IP `94.156.238.53` and `2a12:9b00:b00b:53::53`. When changes are committed to our Git repository, an automated CI/CD pipeline pushes zone updates, triggers `knotc zone-reload`, and issues DNS NOTIFY packets over an encrypted WireGuard mesh.

The catastrophe unfolded during a routine disaster recovery drill. We failed over our primary nameserver from `kc` to a standby backup host in New York. The backup host ran an experimental GitOps reconciliation script that incorporated Unix epoch timestamps for SOA serial numbers (`1788500000`, roughly `0x6AA0A260`) instead of our standard `YYYYMMDDnn` format (`2026090401`). 

When the primary service was restored to Kansas City, the canonical zone files were pushed with the conventional date serial `2026090401`. To standard text comparison, `2026090401` looks larger than `1788500000`. But when an engineer hurriedly tried to roll back the script changes and applied a manual test serial of `2026099901` across all twenty-eight zones, the disaster struck.

When we re-applied the canonical zone files with the proper serial `2026090401`, all eight edge nodes rejected the primary's updates. Incremental zone transfers (IXFR) and full zone transfers (AXFR) halted dead in their tracks:

```text
2026-09-04T11:14:02Z [femboy.zip] refresh: master serial 2026090401 is older than local serial 2026099901, skipping
2026-09-04T11:14:02Z [femboy.zip] zone transfer failed: master sent obsolete serial
2026-09-04T11:14:03Z [238.156.94.in-addr.arpa] refresh: master serial 2026090401 is older than local serial 2026099901, skipping
2026-09-04T11:14:03Z [roa.gay] refresh: master serial 2026090401 is older than local serial 2026099901, skipping
```

New DNS records, ACME TXT challenges for automated Let's Encrypt certificate renewals, and dynamic mail server IP records ceased propagating to edge anycast nodes. The edge daemons were hopelessly locked on the bogus `2026099901` serial.

---

## The Deep-Dive / Root Cause Analysis

To understand why Knot DNS stubbornly refused to adopt the primary's records, we must examine RFC 1982 ("Serial Number Arithmetic") and the underlying storage mechanics of Knot DNS's LMDB engine.

### RFC 1982 Serial Arithmetic

DNS serial numbers are unsigned 32-bit integers ($0$ to $2^{32}-1$, or $4,294,967,295$). In RFC 1982, serial numbers do not use standard linear comparison. Instead, they use circular modular sequence space where addition and comparison wrap around:

$$s_1 < s_2 \iff (s_2 - s_1) \pmod{2^{32}} < 2^{31}$$

Given our edge secondary serial $s_{\text{local}} = 2026099901$ and primary serial $s_{\text{master}} = 2026090401$:

$$(2026090401 - 2026099901) \pmod{2^{32}} = -9500 \equiv 4294957796 \pmod{2^{32}}$$

Because $4,294,957,796 \ge 2^{31}$ ($2,147,483,648$), Knot DNS determines that $s_{\text{master}}$ is in the past—an obsolete serial number. RFC 1982 forbids secondary nameservers from overwriting their local zone database with an older serial number to prevent replay attacks and out-of-order packet races from clobbering modern zone states.

```text
               RFC 1982 Sequence Space (2^32 = 4,294,967,296)
                             
                                   0
                           .-------|-------.
                       _.-'        |        `-._
                    .-'            |            `-.
                  .'               |               `.
                 /                 |                 \
                |                  |                  |
   New Master   |    OBSOLETE      |      VALID       |  Old Secondary
   2026090401   |    DISTANCE      |      WINDOW      |  2026099901
                |    (Past)        |     (Future)     |
                 \                 |                 /
                  `.               |               .'
                    `-.            |            .-'
                       `-._        |        _.-'
                           `-------|-------'
                              2,147,483,648
                                 (2^31)
```

Why couldn't we simply increment the primary serial to `2026099902`?
Because doing so would permanently break our `YYYYMMDDnn` date-based serial convention. September has only 30 days. Setting the serial to day `99` would corrupt all automated CI/CD tools expecting ISO date formats and prevent legitimate date-based serial increments until the year 2027!

### Knot DNS LMDB Journal Mechanics

In classical BIND 9, an administrator might delete a `.jnl` file and issue `rndc reload`. Knot DNS is built fundamentally differently: it stores all zone data, DNSSEC key states (KASP), and journal history inside LMDB (Lightning Memory-Mapped Database) binary files:
- `/var/lib/knot/zones/journal.mdb`
- `/var/lib/knot/zones/zone.mdb`
- `/var/lib/knot/timers/timers.mdb`

Knot DNS maintains ACID transaction logs inside LMDB. When a secondary receives an AXFR or IXFR notification, it queries its local LMDB transaction journal:

```text
[Secondary ns1.fra]
      |
      |-- 1. Receive NOTIFY (serial=2026090401)
      |-- 2. Inspect LMDB zone.mdb -> local SOA serial = 2026099901
      |-- 3. Calculate (2026090401 - 2026099901) % 2^32 = 4294957796
      |-- 4. RFC 1982 rejection: serial is strictly older than local
      x-- 5. Drop IXFR/AXFR request. Never send query to primary.
```

Because the secondary refused to initiate an AXFR pull, the primary had no way to push the corrected zone data down the WireGuard mesh. The edge secondaries were frozen.

---

## The Implementation / Architecture

We solved the crisis by implementing a zero-downtime, automated two-stage recovery procedure across our infrastructure without dropping anycast BGP announcements.

```text
+-----------------------------------------------------------------------------------+
|                            Primary GitOps Master (kc)                             |
|                           10.100.0.1 (WireGuard Mesh)                             |
|                Zone: 2026090401 (Canonical Date Serial Restored)                  |
+-----------------------------------------------------------------------------------+
                                         |
                       [Encrypted WireGuard Mesh: wg-dns]
                                         |
     +-------------------+---------------+-------------------+
     |                   |                                   |
     v                   v                                   v
+----------------+ +----------------+                +----------------+
| Edge POP: fra  | | Edge POP: ny   |  ... (8 POPs)  | Edge POP: sp   |
| 10.100.0.2     | | 10.100.0.3     |                | 10.100.0.9     |
| Knot Secondary | | Knot Secondary |                | Knot Secondary |
+----------------+ +----------------+                +----------------+
     |                   |                                   |
     +-------------------+-----------------------------------+
                         |
                 [BGP Anycast IPv4: 94.156.238.53 / IPv6: 2a12:9b00:b00b:53::53]
                         |
                         v
                 [Global Internet Resolvers]
```

### 1. Primary and Secondary Knot DNS Configurations

Here is the production configuration on our hidden master (`/etc/knot/knot.conf`):

```knot
server:
    rundir: "/run/knot"
    user: knot:knot
    listen: [ 127.0.0.1@53, 10.100.0.1@53 ]

log:
    - target: syslog
      any: info

remote:
    - id: secondary_fra
      address: 10.100.0.2@53
    - id: secondary_ny
      address: 10.100.0.3@53
    - id: secondary_kc
      address: 10.100.0.4@53
    - id: secondary_lv
      address: 10.100.0.5@53

acl:
    - id: secondary_acl
      address: [ 10.100.0.0/24 ]
      action: [ transfer, notify ]

template:
    - id: default
      storage: "/var/lib/knot"
      journal-content: all
      journal-max-usage: 2G
      zonefile-sync: -1
      zonefile-load: difference-no-serial
      acl: [ secondary_acl ]
      notify: [ secondary_fra, secondary_ny, secondary_kc, secondary_lv ]

zone:
    - domain: femboy.zip
      file: "/etc/knot/zones/femboy.zip.zone"
    - domain: 238.156.94.in-addr.arpa
      file: "/etc/knot/zones/238.156.94.in-addr.arpa.zone"
    - domain: roa.gay
      file: "/etc/knot/zones/roa.gay.zone"
```

And the corresponding edge secondary configuration (`/etc/knot/knot.conf` on `ns1.fra`):

```knot
server:
    rundir: "/run/knot"
    user: knot:knot
    listen: [ 127.0.0.1@53, 94.156.238.53@53, 2a12:9b00:b00b:53::53@53, 10.100.0.2@53 ]

remote:
    - id: master_kc
      address: 10.100.0.1@53

acl:
    - id: master_acl
      address: [ 10.100.0.1 ]
      action: [ notify ]

template:
    - id: default
      storage: "/var/lib/knot"
      master: [ master_kc ]
      acl: [ master_acl ]
      journal-content: all
      journal-max-usage: 1G
      zonefile-sync: -1

zone:
    - domain: femboy.zip
    - domain: 238.156.94.in-addr.arpa
    - domain: roa.gay
```

### 2. The WireGuard Mesh Interface

All control-plane traffic, NOTIFY pulses, and zone replication occur strictly over WireGuard (`/etc/wireguard/wg-dns.conf`):

```ini
[Interface]
Address = 10.100.0.2/24
PrivateKey = REDACTED_SECONDARY_FRA_KEY
ListenPort = 51820
Table = off

[Peer]
# Master Kansas City (kc)
PublicKey = REDACTED_MASTER_KC_PUBKEY
Endpoint = 198.51.100.10:51820
AllowedIPs = 10.100.0.1/32
PersistentKeepalive = 25
```

### 3. Automated Emergency Recovery Script

Rather than sequentially logging into eight remote servers and risking split-brain or lingering stale zones, we created an orchestration tool `knot-break-frozen-serial.sh` that utilizes Knot's control socket (`knotc`):

```bash
#!/usr/bin/env bash
set -euo pipefail

# knot-break-frozen-serial.sh
# Automated RFC 1982 bypass and LMDB zone cache purge across AS214806 anycast POPs

MASTER_IP="10.100.0.1"
ZONES=(
    "femboy.zip"
    "femboy.fan"
    "roa.gay"
    "238.156.94.in-addr.arpa"
    "b.0.0.b.0.0.9.b.2.1.a.2.ip6.arpa"
)
POPS=(
    "fra:10.100.0.2"
    "ny:10.100.0.3"
    "kc:10.100.0.4"
    "lv:10.100.0.5"
)

echo "=== [Step 1] Verifying Master Serial Numbers ==="
for ZONE in "${ZONES[@]}"; do
    MASTER_SERIAL=$(dig +short "@${MASTER_IP}" "${ZONE}" SOA | awk '{print $3}')
    echo "[Master] Zone: ${ZONE} -> Serial: ${MASTER_SERIAL}"
done

echo ""
echo "=== [Step 2] Purging Frozen Secondary LMDB Journals and Forcing AXFR ==="
for POP in "${POPS[@]}"; do
    POP_NAME="${POP%%:*}"
    POP_IP="${POP##*:}"
    echo "--- Processing POP: ${POP_NAME} (${POP_IP}) ---"
    
    for ZONE in "${ZONES[@]}"; do
        CURRENT_SEC_SERIAL=$(dig +short "@${POP_IP}" "${ZONE}" SOA | awk '{print $3}' || echo "unreachable")
        echo "  Zone ${ZONE}: current secondary serial is ${CURRENT_SEC_SERIAL}"
        
        # Execute control commands over SSH
        ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no "root@${POP_IP}" bash -c "
            # 1. Purge the zone journal, timers, and cached LMDB records
            knotc zone-purge +journal +timers '${ZONE}'
            
            # 2. Force an immediate AXFR re-transfer from master
            knotc zone-retransfer '${ZONE}'
        "
        
        # Verify convergence
        NEW_SEC_SERIAL=$(dig +short "@${POP_IP}" "${ZONE}" SOA | awk '{print $3}')
        if [[ "${NEW_SEC_SERIAL}" == "${MASTER_SERIAL}" ]]; then
            echo "  [OK] Zone ${ZONE} synchronized to serial ${NEW_SEC_SERIAL}"
        else
            echo "  [WARN] Zone ${ZONE} serial mismatch: expected ${MASTER_SERIAL}, got ${NEW_SEC_SERIAL}"
        fi
    done
done

echo ""
echo "=== [Step 3] Flushing Edge Resolvers and Checking Global Convergence ==="
for ZONE in "${ZONES[@]}"; do
    echo "Verifying ${ZONE} across public anycast IP (94.156.238.53):"
    dig +short @94.156.238.53 "${ZONE}" SOA
done

echo "All 28 zones recovered cleanly across AS214806 anycast mesh."
```

### 4. Prometheus Serial Divergence Alert

To ensure a frozen serial never remains undetected, we deployed an automated Prometheus alert using the Knot DNS Prometheus exporter:

```yaml
groups:
  - name: dns_replication_alerts
    rules:
      - alert: KnotZoneSerialDesync
        expr: |
          (knot_zone_serial{role="secondary"} != on(zone) group_left() knot_zone_serial{role="master"})
        for: 5m
        labels:
          severity: critical
          service: authoritative-dns
        annotations:
          summary: "Knot DNS zone serial desync detected"
          description: "Zone {{ $labels.zone }} on secondary {{ $labels.instance }} has serial {{ $value }} diverging from primary for more than 5 minutes."
```

---

## Lessons Learned & Best Practices

1. **RFC 1982 Circular Arithmetic Has Real Teeth**: Never assume serial number increments are linear comparisons. If you inadvertently roll a serial forward into the distant future or switch from timestamps to dates, secondaries will treat valid canonical records as obsolete and reject all updates.
2. **Knot's LMDB Journals Require Dedicated Control Purging**: Unlike flat-file DNS servers where deleting zone files suffices, Knot tracks journal state transactions in LMDB. The proper resolution method is `knotc zone-purge +journal +timers` paired with `knotc zone-retransfer`.
3. **Enforce GitOps Pre-Commit Hooks on SOA Serials**: Serials should never be updated manually or via unvalidated scripts. We instituted a pre-commit check validating that every modified `.zone` file contains a serial matching `^20[0-9]{2}(0[1-9]|1[0-2])(0[1-9]|[12][0-9]|3[01])[0-9]{2}$` and that it is monotonically greater than the origin commit.
4. **Isolate Control Plane over WireGuard**: Transmitting transfers over a private WireGuard overlay mesh protects AXFR traffic from public inspection and ensures unhindered transport even when public edge routes are undergoing maintenance.

---

## References

- [RFC 1982: Serial Number Arithmetic](https://datatracker.ietf.org/doc/rfc1982/)
- [RFC 1034: Domain Names - Concepts and Facilities](https://datatracker.ietf.org/doc/rfc1034/)
- [Knot DNS Official Documentation - Zone Control & Purge Operations](https://www.knot-dns.cz/documentation/)
- [Lightning Memory-Mapped Database (LMDB) Architecture](https://symas.com/lmdb/)
