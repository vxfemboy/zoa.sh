---
title: When iBGP Breaks Your Mail: The Missing Host Route
date: 2026-09-05
slug: when-ibgp-breaks-your-mail
tags: networking, bgp, email, stalwart, anycast
---

# When iBGP Breaks Your Mail: The Missing Host Route

## The Problem

Inbound mail to `admin@femboy.zip` and domains hosted on our mail cluster suddenly became intermittent and then stopped delivering altogether. Inbound SMTP connections from major providers like Gmail would either time out during handshake or vanish into the void.

Our infrastructure runs across 8 edge Points of Presence (POPs) globally, announcing our IPv4 and IPv6 address space over BGP. The actual mail service—running Stalwart Mail in an Incus container—lives exclusively on the Kansas City (`kc`) host. Why would removing an old retired physical machine (`sockpuppet`) completely break external mail delivery to `kc`?

## The Journey

First, we checked whether Stalwart inside the container on `kc` was even receiving messages or if the local queue was wedged. We inspected the local SQLite/RocksDB backend directly:
```text
Table 's_email': 1751 rows
Table 'p': 5876 rows
```

Sending a manual test probe directly over the internal loopback immediately incremented the store:
```text
s_email: 1751 -> 1752
p:       5876 -> 5880
```
Stalwart was healthy. Inbound acceptance, parser logic, and local mailbox storage functioned perfectly. The issue was strictly network pathing from the public Internet to `kc`.

Next, we looked at how public traffic reaches our infrastructure:
- We announce `94.156.238.0/24` over eBGP from all edge POPs (anycast).
- A specific host-pinned IP (`94.156.238.25/32` and `2a12:9b00:b00b:25::/128`) is configured as the public MX record for our mail exchange.

Tracing the BGP routing table across the mesh revealed the culprit:
The `/32` mail host pin had historically been originated by `sockpuppet` over internal iBGP, pointing traffic across the WireGuard mesh into `kc`. When `sockpuppet` was decommissioned and taken offline, the specific `/32` route disappeared from the iBGP tables across our remaining POPs!

Without the specific `/32` route:
1. An incoming SMTP packet destined for `94.156.238.25` hit whatever edge POP was closest to the sender (e.g. Frankfurt, New York, or Las Vegas) because of the general `/24` anycast announcement.
2. The edge POP looked up `94.156.238.25` in its local routing table. Without the `/32` host route, it matched only the local aggregate `/24` subnet.
3. The edge POP tried to deliver the packet locally or dropped it, because Stalwart wasn't running on that POP.
4. Only packets that coincidentally landed directly on `kc`'s uplink reached the mailbox!

## The Solution

The Kansas City node itself needed to originate the `/32` host pin and announce it across the iBGP mesh so that every POP knows precisely where to forward mail traffic.

We added the host pin routes into `kc`'s Bird routing configuration:

```bird
protocol static mail_pin {
    ipv4 { preference 200; };
    route 94.156.238.25/32 reject;
}

protocol static mail_pin_v6 {
    ipv6 { preference 200; };
    route 2a12:9b00:b00b:25::/128 reject;
}
```

And exported them across the internal mesh:
```bird
filter ibgp_mesh_export {
    if proto = "mail_pin" then accept;
    if proto = "mail_pin_v6" then accept;
    # ... standard mesh exports
}
```

Once Bird reloaded (`birdc configure`), `94.156.238.25/32` was broadcast to all 8 POPs via iBGP. Any POP receiving external SMTP traffic now encapsulates and forwards it directly across the WireGuard tunnel to `kc`.

We re-tested delivery with external test probes from Gmail and verified the store counters:
```text
Table 's_email': 1752 -> 1753
Delivery status: 250 2.0.0 Ok: queued as ...
```

## Lessons Learned

- **Anycast + Unicast Host Pins Require Dedicated Originators**: When announcing an aggregate prefix (like `/24`) via anycast while running stateful unicast services (like SMTP) on an IP within that range, the unicast node must explicitly originate the `/32` host route over iBGP.
- **Decommissioning Nodes Can Drop Silent Dependencies**: When taking an old server offline, audit route originations in Bird/BGP. If a retired box was the designated originator for any host pin, another node must adopt that origination before teardown.
- **Verify Internal Services vs External Transit**: Testing local store increments isolated the problem to network routing in minutes, eliminating hours of unnecessary application debugging.

## References

- [Bird Internet Routing Daemon Documentation](https://bird.network.cz/)
- [Stalwart Mail Server Documentation](https://stalw.art/docs/)
- [RFC 4786: Operation of Anycast Services](https://datatracker.ietf.org/doc/rfc4786/)
