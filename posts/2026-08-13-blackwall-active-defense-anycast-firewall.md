---
title: Blackwall: Building an Automated Active-Defense Firewall on Anycast Edge Nodes
short_title: Blackwall Edge Firewall
subtitle: Building an Automated Active-Defense Firewall on Anycast Edge Nodes
date: 2026-08-13
slug: blackwall-active-defense-anycast-firewall
tags: security, networking, anycast, linux, firewall, ebpf
---

# Blackwall: Building an Automated Active-Defense Firewall on Anycast Edge Nodes

## Table of Contents
1. [The Internet Is a Hostile Neighborhood](#the-internet-is-a-hostile-neighborhood)
2. [Why Off-the-Shelf Tools Failed Us](#why-off-the-shelf-tools-failed-us)
3. [The Architecture of Blackwall](#the-architecture-of-blackwall)
4. [Tarpits, Honeypots, and Kernel Blackholes](#tarpits-honeypots-and-kernel-blackholes)
5. [Anycast Mesh Synchronization](#anycast-mesh-synchronization)
6. [Field Results: What Scanners Actually Hit Us With](#field-results-what-scanners-actually-hit-us-with)
7. [References](#references)

---

## The Internet Is a Hostile Neighborhood

When you announce your own `/24` IPv4 block (`94.156.238.0/24`) to the global Internet routing table, you don't just get access to 256 IP addresses. You get an uninterrupted, round-the-clock deluge of automated port scans, brute-force bots, opportunistic credential stuffers, and Mirai variants probing every single IP from `94.156.238.1` to `94.156.238.254` within 45 seconds of your BGP announcement going live.

On an anycast network, this assault is distributed across all your edge Points of Presence (POPs). While anycast absorbs DDoS volume naturally, passive firewall rules (like standard `iptables -A INPUT -j DROP`) are boring and give scanners clean data: a dropped packet tells the attacker there is a firewall; a closed port (`RST`) tells them the host is active but unlistening.

We wanted something active, dynamic, and merciless. We built **Blackwall**.

---

## Why Off-the-Shelf Tools Failed Us

Before writing custom automation, we evaluated existing open-source defense tools like `ghostport`, `masscanned`, `endlessh`, and standard `fail2ban`:

1. **Masscan / Ghostport Quirks:** Many existing tarpits operate as high-level user-space python or node daemons that bind thousands of sockets. Under high scan velocity (100,000 packets/sec), file descriptor limits exhaust system memory and CPU scheduling collapses.
2. **Lack of Anycast Awareness:** If a bot scans `94.156.238.45:22` and hits our Frankfurt node, banning that IP only in Frankfurt is useless. The scanner's next probe for `94.156.238.46:22` might be routed by upstream transit to New York. The threat intel had to propagate across the entire autonomous system in milliseconds.
3. **Passive Dropping Leaks Topology:** Modern vulnerability scanners measure response latencies across IP ranges to map subnets and identify NAT gateways.

---

## The Architecture of Blackwall

Blackwall splits security enforcement into three layers:

```text
+-------------------------------------------------------------+
|                  Internet Edge Traffic Ingress              |
+-------------------------------------------------------------+
                              |
                              v
                [L3/L4 eBPF / XDP Ingress Filter]
                  - High-rate packet inspection
                  - Drops confirmed malicious CIDRs in kernel
                              |
                              v
                  [Blackwall Tarpit Daemon]
                  - Intercepts unauthorized SYN packets
                  - Emulates open ports with fake handshakes
                  - Feeds slow, infinite entropy payloads
                              |
                              v
                  [Mesh Sync Engine (Redis/WireGuard)]
                  - Broadcasts offender IPs to all 8 POPs
```

---

## Tarpits, Honeypots, and Kernel Blackholes

Instead of rejecting connections on unassigned ports, Blackwall implements a multi-stage trap:

### 1. The SYN Trap (TCP Tarpit)
When an unknown remote IP sends a TCP SYN packet to an unrouted IP in our `/24`, Blackwall responds with a valid `SYN-ACK` with an artificially throttled TCP window size of `1` or `2` bytes:

```text
Attacker                     Blackwall Edge
   |                               |
   |--- TCP SYN (Port 2222) ------>|
   |<-- TCP SYN-ACK (Win=1) -------|
   |--- TCP ACK ------------------>|
   |                               |
   |<-- 1 Byte SSH Banner (10s) ---|
   |                               | (Attacker connection hangs for hours)
```

By tricking the remote scanner into maintaining thousands of half-open TCP states, we turn the computational cost of the scan back against the attacker.

### 2. High-Speed Kernel Blackholing via nftables sets
Once an IP exceeds our threshold (e.g. hitting more than 4 unrouted ports within 30 seconds), Blackwall dynamically promotes the IP into an atomic `nftables` set evaluated in kernel space before connection tracking allocates any memory:

```bash
nft add element inet blackwall banned_ips { 198.51.100.42 timeout 24h }
```

---

## Anycast Mesh Synchronization

When New York bans an abusive IP, every other POP (`kc`, `lv`, `de`, `ch`) must adopt the ban immediately.

We implemented a lightweight mesh synchronizer over our existing internal WireGuard backbone using Redis Pub/Sub:
- When a node bans an IP, it publishes:
  `{"ip": "198.51.100.42", "reason": "ssh-scan", "ttl": 86400}`
- Subscribed daemons on every edge POP receive the message and insert the IP into their local `nftables` set in under 45 milliseconds.

---

## Field Results: What Scanners Actually Hit Us With

Over our first 30 days running Blackwall on AS214806:
- **Total Unique Scanner IPs Trapped:** 41,208
- **Top Targeted Ports:**
  - `22` / `2222` (SSH brute-forcers) — 38%
  - `23` / `2323` (Mirai / Telnet IoT bots) — 24%
  - `80` / `443` / `8080` (Spring Boot & phpMyAdmin vulnerability scrapers) — 19%
  - `5060` (SIP VoIP exploiters) — 11%
- **Average Attacker Connection Hold Time:** 4.2 hours per tarpit session.

The result is a clean, quiet autonomous system where malicious noise is neutralized before it ever reaches our application containers.

---

## References
- [nftables Sets and Maps Documentation](https://wiki.nftables.org/wiki-nftables/index.php/Sets)
- [eBPF and XDP Reference Guide](https://docs.cilium.io/en/stable/bpf/)
- [Endlessh: An SSH Tarpit](https://github.com/skeeto/endlessh)
