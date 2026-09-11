---
title: The Sockpuppet Cutover Drill: Chaos Testing Anycast Edge Redundancy
short_title: Sockpuppet Anycast Drill
subtitle: What Happens When You Pull the Plug on a Primary Gateway
date: 2026-08-20
slug: sockpuppet-cutover-drill-chaos-testing-anycast
tags: bgp, anycast, networking, chaos-engineering, bird
---

# The Sockpuppet Cutover Drill: Chaos Testing Anycast Edge Redundancy

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Building a resilient BGP anycast edge network looks bulletproof on paper: announce your prefixes from multiple data centers worldwide, configure health-checking daemons, and let the internet's routing table automatically shift packets to the nearest healthy Point of Presence (POP) when a node fails. In practice, unverified failover assumptions almost always collapse into silent routing loops, asymmetric return path drops, and agonizing BGP hold-timer delays.

Our autonomous system (AS214806) announces our primary service prefixes (`94.156.238.0/24` and `2a12:9b00:b00b::/48`) across Tier-1 transit providers (including Cogent, Arelion, and Hurricane Electric) from edge POPs in Kansas City (`kc`), New York (`ny`), and Frankfurt (`fra`). Origin services run in Kansas City, connected to edge nodes via encrypted WireGuard tunnels and internal iBGP routes managed by BIRD.

To validate that our network could withstand an abrupt primary transit failure without human intervention, we scheduled the "Sockpuppet Cutover Drill": an unannounced chaos test where we forcibly pulled the virtual plug on our primary Kansas City gateway (`gw-kc-01`). The drill simulated an ungraceful catastrophic failure: an abrupt hypervisor kernel panic, severed fiber uplinks, and immediate loss of BGP keepalives.

Within seconds of killing the gateway, our synthetic monitoring probes erupted in alarms:
- **TCP Connection Reset Storms**: Existing HTTPS connections traversing transatlantic routes experienced 100% packet loss and timed out after 90 seconds.
- **The Black Hole Window**: Upstream transit providers continued pumping inbound packets toward Kansas City for nearly three minutes despite the dead link.
- **Asymmetric Flapping**: When Frankfurt attempted to absorb European and East Coast traffic, internal return packets took asymmetric routes over stale WireGuard tunnels, colliding with Linux Reverse Path Filtering (`rp_filter`) rules and getting summarily discarded at the kernel boundary.

The failure proved that passive BGP timeout defaults are completely inadequate for real-time edge failover.

---

## The Deep-Dive / Root Cause Analysis

Dissecting the failure required analyzing BGP timer mechanics, Bidirectional Forwarding Detection (BFD), and Linux kernel socket state machines under asymmetric routing.

### The Default BGP Hold-Timer Trap

Standard BGP session parameters are designed for router stability, not sub-second disaster recovery:
```text
BGP Keepalive Timer: 60 seconds
BGP Hold Timer:      180 seconds (3x keepalive)
```

When `gw-kc-01` experienced an abrupt power cut, no TCP `FIN` or `RST` packets were transmitted to upstream transit peers. Upstream BGP routers in Chicago and Denver kept the Kansas City route active in their Forwarding Information Base (FIB) until the 180-second hold timer expired:

```text
Time    Kansas City Edge (Dead)                Upstream Tier-1 Peer (Arelion)
 |
 t=0s   [POWER LOSS: Hypervisor dies]
 t=1s                                          Routes traffic to KC FIB entry
 t=10s                                         Packets dropped into void (Silent Black Hole)
 t=60s                                         Missed BGP Keepalive 1
 t=120s                                        Missed BGP Keepalive 2
 t=180s                                        HOLD TIMER EXPIRED: Flush KC Prefix!
 t=181s                                        Reconverge to Frankfurt / New York
```

For three full minutes, any client whose BGP path favored Kansas City experienced total service outage.

### BFD (Bidirectional Forwarding Detection) Absence

Without BFD, BGP has no sub-second mechanism to detect link liveness across Layer-2 switching fabrics. If an intermediate switch port stays up while the daemon behind it halts, BGP remains completely oblivious until long hold timers tick away.

### Kernel Reverse Path Filtering (`rp_filter = 1`)

When transit finally converged and routed packets to New York, return packets hit an unexpected kernel barrier. Our origin application containers responded using source IP `94.156.238.50`. 

On the New York edge router, the ingress interface for the client request was `eth0` (transit), but the routing table for return packets indicated the best path back to the client was via an alternate internal gateway (`wg-core`). Because Linux `net.ipv4.conf.all.rp_filter` was set to `1` (strict mode):
$$\text{If } \text{FIB}(\text{source\_ip}) \ne \text{ingress\_interface} \implies \text{DROP}$$

The kernel suspected IP spoofing and silently dropped every outbound SYN-ACK packet:
```text
kernel: [ChaosDrill] IPv4: Martian source 94.156.238.50 from 198.51.100.42, on dev eth0
```

---

## The Implementation / Architecture

We redesigned our edge routing topology by deploying Bidirectional Forwarding Detection (BFD) with millisecond timers, BGP graceful shutdown signaling, and loose reverse path filtering (`rp_filter = 2`).

```
                    +---------------------------+
                    |  Upstream Transit Peers   |
                    |  (Arelion, Cogent, HE)    |
                    +---------------------------+
                             /         \
                 BFD (300ms)           BFD (300ms)
                           /             \
                          v               v
            +-------------------+   +-------------------+
            |   Edge POP (KC)   |   |   Edge POP (NY)   |
            |  Primary Anycast  |   |  Backup Anycast   |
            +-------------------+   +-------------------+
                    |         \       /         |
                    |          \     /          |
               WireGuard        WireGuard   WireGuard
                    |            \ /            |
                    +-------------X-------------+
                                  |
                                  v
                    +---------------------------+
                    |    Origin Core Cluster    |
                    |   rp_filter = 2 (Loose)   |
                    |   Deterministic Next-Hop  |
                    +---------------------------+
```

### Hardened BIRD Configuration with BFD

In our updated `/etc/bird/bird.conf`, we paired all external eBGP and internal iBGP sessions with BFD protocols:

```bird
# Global router identification
router id 94.156.238.1;
define OWNAS = 214806;

# 1. BFD Protocol Configuration for Sub-Second Failure Detection
protocol bfd {
    interface "eth0" {
        min rx interval 100 ms;
        min tx interval 100 ms;
        multiplier 3; # Failover in 300 ms!
    };
    interface "wg-*" {
        min rx interval 200 ms;
        min tx interval 200 ms;
        multiplier 3;
    };
}

# 2. Kernel and Device Syncer
protocol device {
    scan time 10;
}

protocol direct {
    ipv4;
    interface "dummy0", "lo";
}

# 3. eBGP Transit Session with BFD and Graceful Teardown
protocol bgp transit_arelion {
    local 94.156.238.1 as OWNAS;
    neighbor 195.12.254.1 as 1299;
    
    bfd on; # Link liveness tied directly to BFD
    
    ipv4 {
        import filter {
            # Accept default route and customer prefixes
            if net = 0.0.0.0/0 then accept;
            reject;
        };
        export filter {
            # Announce anycast service block
            if net = 94.156.238.0/24 then {
                bgp_community.add((OWNAS, 100));
                accept;
            }
            reject;
        };
    };
}

# 4. Internal Mesh iBGP Session with Fast Reroute
protocol bgp ibgp_ny {
    local 10.100.0.1 as OWNAS;
    neighbor 10.100.0.2 as OWNAS;
    
    bfd on;
    next hop self;
    
    ipv4 {
        import all;
        export all;
    };
}
```

### Sysctl Hardening for Asymmetric Anycast Paths

To allow asymmetric multipath routing between edge POPs without dropping valid return traffic, we adjusted the Linux kernel's networking parameters across all edge nodes and origin hypervisors:

```bash
# /etc/sysctl.d/99-anycast-routing.conf

# Enable Loose Reverse Path Filtering (RFC 3704)
# Allows packets if the source is reachable via ANY interface
net.ipv4.conf.all.rp_filter = 2
net.ipv4.conf.default.rp_filter = 2
net.ipv4.conf.eth0.rp_filter = 2
net.ipv4.conf.wg0.rp_filter = 2

# Enable TCP BBR Congestion Control for faster recovery after route flaps
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr

# TCP SYN Retries & Keepalives
net.ipv4.tcp_syn_retries = 2
net.ipv4.tcp_keepalive_time = 30
net.ipv4.tcp_keepalive_intvl = 5
net.ipv4.tcp_keepalive_probes = 3
```

---

## Lessons Learned & Best Practices

1. **Never Deploy Anycast Without BFD**: Standard BGP hold-timers take 180 seconds to detect dead links. BFD reduces failure detection to under 300 milliseconds.
2. **Switch `rp_filter` to Loose Mode (2) on Edge Routers**: Strict reverse path filtering (`rp_filter = 1`) breaks asymmetric routing during anycast cutovers, causing silent kernel packet drops.
3. **Automate Chaos Drills Regularly**: Simulating link and hypervisor failures under controlled conditions uncovers silent routing dependencies that never appear during synthetic tabletop reviews.
4. **Use BBR for Rapid Path Shift Recovery**: CUBIC's exponential backoff struggles when TCP flows shift abruptly across transatlantic links. BBR adapts immediately to the new bandwidth-delay product.

---

## References

- [RFC 5880: Bidirectional Forwarding Detection (BFD)](https://datatracker.ietf.org/doc/rfc5880/)
- [RFC 3704: Ingress Filtering for Multihomed Networks (RP Filtering)](https://datatracker.ietf.org/doc/rfc3704/)
- [BIRD Internet Routing Daemon Documentation](https://bird.network.cz/?get_doc)
