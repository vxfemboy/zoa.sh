---
title: High-Reliability Anycast: What Happens When an Entire Edge POP Vanishes?
short_title: High-Reliability Anycast
subtitle: What Happens When an Entire Edge POP Vanishes?
date: 2026-08-16
slug: anycast-resilience-when-an-edge-pop-vanishes
tags: networking, bgp, anycast, chaos-engineering, sysadmin
---

# High-Reliability Anycast: What Happens When an Entire Edge POP Vanishes?

## Table of Contents
1. [The Theory vs. Reality of Anycast Failover](#the-theory-vs-reality-of-anycast-failover)
2. [Simulating a Catastrophic Node Loss](#simulating-a-catastrophic-node-loss)
3. [The BGP Convergence Timeline](#the-bgp-convergence-timeline)
4. [Route Flap Damping and Tier-1 Upstream Latencies](#route-flap-damping-and-tier-1-upstream-latencies)
5. [Hardening our BGP Health-Check Timers](#hardening-our-bgp-health-check-timers)
6. [Conclusion](#conclusion)
7. [References](#references)

---

## The Theory vs. Reality of Anycast Failover

The foundational selling point of BGP anycast is seamless high availability:
> *"If an edge POP dies, upstream routers automatically withdraw the route and send traffic to the next closest POP with zero intervention!"*

In theory, yes. In production on the public Internet? **BGP convergence is rarely instantaneous.** Upstream transit providers have route timers, caching layers, and flap dampening algorithms designed to prevent routing storms.

We wanted to know our real-world failure metrics:
- If our New York (`ny`) edge router loses power, how many seconds does it take for traffic to re-converge onto Kansas City or Frankfurt?
- Do existing TCP connections reset cleanly, or do they hang in half-open purgatory?
- Does withdrawal of our `/24` prefix trigger upstream route flap dampening?

We scheduled a controlled chaos engineering experiment on AS214806 to find out.

---

## Simulating a Catastrophic Node Loss

To test worst-case failure, we didn't perform a graceful BGP shutdown (`BGP Cease` / `ADMINISTRATIVE_SHUTDOWN`). A graceful shutdown allows routers to propagate clean withdrawals in sub-second time.

Instead, we simulated a sudden power-loss event: we dropped the transit uplinks on our New York node abruptly via iptables hardware emulation, forcing the upstream transit providers (Cogent and Hurricane Electric) to detect the outage strictly through BGP **BGP Hold Timer expiration**:

```text
[Global Internet Traffic]
            |
            v
   [Tier-1 Transit Provider (HE/Cogent)]
      | (BGP Session active: HoldTimer = 90s)
      |
      x <--- LINK CUT (Simulated Blackhole)
      |
   [Edge POP: New York (ny)] (Dead)
```

At the same time, we ran continuous probe clients across 12 worldwide monitoring nodes, recording ICMP latency, TCP handshake success rates, and HTTP request timings every 200 milliseconds.

---

## The BGP Convergence Timeline

Here is what the real packet traces looked like:

```text
T+0.00s : Uplink on New York dropped.
T+0.20s : Probe traffic sent to New York continues flying into dead link. 
          Packet loss on North American East Coast spikes to 100%.
T+15.0s : BGP BFD (Bidirectional Forwarding Detection) fires on Cogent session.
          Cogent withdraws 94.156.238.0/24 from New York.
          Traffic from Cogent immediately re-routes to Kansas City (kc).
          Packet loss drops to 0% for Cogent-routed clients!
T+45.0s : Hurricane Electric BGP KeepAlive timeout expires on secondary peer.
          HE withdraws route. European traffic re-routes to Frankfurt (de).
T+48.2s : Full global BGP convergence achieved. 100% of global traffic restored.
```

The experiment exposed an immediate vulnerability:
While Cogent supported fast **BFD (Bidirectional Forwarding Detection)** with 300ms heartbeat intervals, our secondary upstream peer was relying on standard BGP timers (Hold Time = 90 seconds, KeepAlive = 30 seconds).

For 45 seconds, any client whose ISP peered with Hurricane Electric was blackholed into New York!

---

## Route Flap Damping and Tier-1 Upstream Latencies

When New York disappeared, our Kansas City node saw an immediate 65% surge in incoming connection rates.

Because our backend architecture follows the **Island Survivability** rule (independent local Incus containers on ZFS), Kansas City absorbed the diverted load without breaking a sweat. Memory utilization increased by only 8%, and CPU usage on the dual Xeon E5 chips remained under 18%.

However, when we restored New York's network connection 10 minutes later, we observed another classic BGP pitfall: **Route Flap Damping (RFC 2439)**.

Because New York had withdrawn and then re-announced the `/24` within a short interval, several European Internet exchanges placed the announcement in penalty status for 15 minutes to suppress routing churn.

---

## Hardening our BGP Health-Check Timers

The chaos test yielded three crucial improvements to our Bird BGP configurations:

### 1. Mandatory BFD on All eBGP Peering Sessions
We enforced BFD across all upstream sessions in `/etc/bird/bird.conf`:

```bird
protocol bfd {
    interface "eth0" {
        min rx interval 300 ms;
        min tx interval 300 ms;
        idle tx interval 1000 ms;
        multiplier 3;
    };
}

protocol bgp upstream_transit {
    # ...
    bfd on;
    hold time 90;
    keepalive time 30;
}
```
With BFD active, link failures are detected in $3 \times 300\text{ms} = 900\text{ms}$, slashing failover time from 45 seconds to under 1 second.

### 2. Graceful Maintenance BGP Community Signaling
For planned reboots, we codified standard BGP communities to notify transit providers to deprioritize a node (graceful shutdown) before taking interfaces down:
```bird
# Announce GRACEFUL_SHUTDOWN community (RFC 8326)
bgp_community.add((65535, 0));
```

---

## Conclusion

Anycast is not magic; it is simply automated policy routing. Understanding the physical reality of BGP hold timers, BFD negotiation, and transit failover intervals is what separates brittle setups from truly bulletproof infrastructure.

---

## References
- [RFC 5880: Bidirectional Forwarding Detection (BFD)](https://datatracker.ietf.org/doc/rfc5880/)
- [RFC 8326: BGP Graceful Shutdown](https://datatracker.ietf.org/doc/rfc8326/)
- [RFC 2439: BGP Route Flap Damping](https://datatracker.ietf.org/doc/rfc2439/)
