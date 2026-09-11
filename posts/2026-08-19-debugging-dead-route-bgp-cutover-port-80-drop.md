---
title: Debugging the Dead Route: Why BGP Cutover Dropped Port 80 but Kept Port 25
short_title: Debugging the Dead BGP Route
subtitle: Tracing Asymmetric Routing and Dummy Interface Ephemerality
date: 2026-08-19
slug: debugging-dead-route-bgp-cutover-port-80-drop
tags: bgp, bird, networking, caddy, firewall
---

# Debugging the Dead Route: Why BGP Cutover Dropped Port 80 but Kept Port 25

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

During a planned BGP maintenance cutover on our Kansas City edge router, our autonomous system exhibited a bewildering routing pathology: inbound mail traffic on Port 25 connected instantly and processed messages flawlessly, while web traffic on Port 80 and Port 443 died at the TCP SYN packet level. To external monitoring probes, the web server was completely dark, even though both services shared the exact same public BGP-announced prefix.

Our autonomous system (AS214806) multi-homes with two Tier-1 transit providers: Cogent Communications (AS174) on `eth0` and Hurricane Electric (AS6939) on `eth1`. From our edge router (`kc-edge-01`), we announce our public IPv4 allocation `94.156.238.0/24` using the BIRD 2.x Internet Routing Daemon.

Within that prefix, services are addressed logically:
- Mail Server (Stalwart SMTP): `94.156.238.25`
- Public Web Ingress (Caddy reverse proxy): `94.156.238.80`

To cut over to a hardened edge router, we established eBGP peering sessions with both Cogent and Hurricane Electric. BIRD reported both sessions as `Established`. The `/24` prefix was accepted by both upstreams and propagated across the global default-free zone (DFZ).

We began validating external connectivity:

```bash
# Probing SMTP (Port 25)
$ nc -zv -w 3 94.156.238.25 25
Connection to 94.156.238.25 25 port [tcp/smtp] succeeded!

# Probing HTTP (Port 80)
$ curl -4 -v --connect-timeout 5 http://94.156.238.80/
* Trying 94.156.238.80:80...
* Connection timed out after 5001 milliseconds
* Closing connection 0
curl: (28) Failed to connect to 94.156.238.80 port 80: Connection timed out
```

In networking fundamentals, BGP operates strictly at Layer 3 (Network Layer). Routers forward IP packets based on destination prefixes; they have no awareness of Layer 4 TCP port numbers. If the BGP route for `94.156.238.0/24` was functional for Port 25, packets destined for Port 80 had to be reaching the physical machine.

Why was Port 25 thriving while Port 80 was silently dropped into the void?

---

## The Deep-Dive / Root Cause Analysis

To diagnose the outage, we attached packet captures across all physical and virtual interfaces using `tcpdump` simultaneously:

```bash
tcpdump -i any -nn 'port 80 or port 25'
```

The trace immediately revealed where the connection was terminating:

```text
# Port 80 HTTP Trace:
14:22:01.120412 IP 198.51.100.45.49210 > 94.156.238.80.80: Flags [S], seq 3810294812, win 64240, ... In eth0
14:22:02.122105 IP 198.51.100.45.49210 > 94.156.238.80.80: Flags [S], seq 3810294812, win 64240, ... In eth0

# Port 25 SMTP Trace:
14:22:04.550119 IP 198.51.100.45.51230 > 94.156.238.25.25: Flags [S], seq 1928401124, win 64240, ... In eth0
14:22:04.550381 IP 94.156.238.25.25 > 198.51.100.45.51230: Flags [S.], seq 4019284102, ack 1928401125, ... Out eth1
```

Look at the difference in the packet traces:
1. For Port 80, the incoming TCP SYN packet arrived on `eth0` (Cogent), but the Linux kernel **never emitted a SYN-ACK reply**. The packet disappeared inside the networking stack.
2. For Port 25, the incoming TCP SYN arrived on `eth0` (Cogent), and the kernel immediately transmitted a SYN-ACK out of `eth1` (Hurricane Electric). The connection was established via **asymmetric routing**.

This pointed directly to a kernel ingress drop mechanism.

```text
                             ASYMMETRIC ROUTING & RP_FILTER
                             
       [External Client: 198.51.100.45]
            |                     ^
    Ingress | (Via Cogent)        | Egress (Via Hurricane Electric)
            v                     |
      [eth0: Cogent]        [eth1: HE]
            |                     ^
            v                     |
    +---------------------------------------------------------+
    |                   Linux Kernel Stack                    |
    |                                                         |
    |  Port 80 Ingress:                                       |
    |  1. Ingress on eth0                                     |
    |  2. Kernel checks FIB: best route to 198.51.100.45 is   |
    |     via eth1 (Hurricane Electric)                       |
    |  3. rp_filter = 1 (Strict Mode):                        |
    |     Arrival interface (eth0) != Reverse interface (eth1)|
    |     --> SILENT KERNEL DROP (Martian packet)             |
    |                                                         |
    |  Port 25 Ingress:                                       |
    |  1. Policy routing rule matched: 'from 94.156.238.25'   |
    |  2. Custom route table forces egress via eth0           |
    |  3. Arrival interface (eth0) == Reverse interface (eth0)|
    |     --> PACKET ACCEPTED & PROCESSED                     |
    +---------------------------------------------------------+
```

### Culprit 1: Strict Reverse Path Filtering (`rp_filter = 1`)

Linux implements RFC 3704 Reverse Path Filtering to combat IP address spoofing. The kernel offers three modes via sysctl:
- `0`: No filtering. Any packet is accepted regardless of source.
- `1`: Strict Mode. The incoming packet is dropped if the interface it arrived on is not the best reverse path to the source IP according to the local Forwarding Information Base (FIB).
- `2`: Loose Mode. The incoming packet is accepted as long as the source IP is reachable via *any* interface on the system.

During the cutover, a system package upgrade had restarted `systemd-networkd`, which silently re-asserted the distribution default `net.ipv4.conf.all.rp_filter = 1`.

Because our edge router is multi-homed, global Internet traffic is inherently asymmetric:
- Cogent (`eth0`) offered the lowest latency path from European client `198.51.100.45`, so their SYN arrived on `eth0`.
- However, our edge router's internal BGP best-path calculation selected Hurricane Electric (`eth1`) for outbound packets to `198.51.100.0/24`.

When the Port 80 SYN arrived on `eth0`, the kernel performed a reverse path lookup. Because the return path pointed out `eth1`, Strict Reverse Path Filtering flagged the packet as spoofed and dropped it silently.

### Culprit 2: Ephemeral Dummy Interface Configuration

Why did Port 25 succeed while Port 80 failed under the exact same sysctl setting?
1. The mail server had a dedicated policy routing rule configured:
   `ip rule add from 94.156.238.25 table mail_egress`
   This forced reverse lookups for `94.156.238.25` to evaluate a symmetric routing table.
2. Web traffic (`94.156.238.80`) was designed to bind to an anycast dummy interface (`dummy0`).
3. During the router reboot, `dummy0` had been created via a one-off shell command without persistent systemd-networkd unit files. When the network stack reloaded, `dummy0` vanished!
4. Caddy, unable to bind to the missing `dummy0` IP, fell back to binding to wildcard `0.0.0.0:80`. Without a source-specific policy routing rule, return packets evaluated the main routing table, triggered the strict `rp_filter` discrepancy, and were dropped.

---

## The Implementation / Architecture

We solved the crisis by deploying a three-part configuration: enabling RFC 3704 Loose Reverse Path Filtering across all transit interfaces, creating persistent anycast dummy netdevs in `systemd-networkd`, and standardizing policy routing tables.

```text
+---------------------------------------------------------------------------------------+
|                                EDGE ROUTER ARCHITECTURE                               |
+---------------------------------------------------------------------------------------+

       [Cogent: AS174] (eth0)                   [Hurricane Electric: AS6939] (eth1)
              |                                                 |
              +-----------------------+-------------------------+
                                      |
                         +--------------------------+
                         | BIRD 2.x Routing Daemon  |
                         | - Ingress multi-homing   |
                         | - Announces /24 prefix   |
                         +--------------------------+
                                      |
                         +--------------------------+
                         | Linux Kernel (rp_filter=2|
                         | - Loose Reverse Path     |
                         | - Asymmetric traffic OK  |
                         +--------------------------+
                                      |
                                      v
                         +--------------------------+
                         | systemd-networkd dummy0  |
                         | - IP: 94.156.238.80/32   |
                         | - Persistent after boot  |
                         +--------------------------+
                                      |
                                      v
                         +--------------------------+
                         | Caddy Reverse Proxy (80) |
                         | - Serves web traffic     |
                         +--------------------------+
```

### 1. Hardening Reverse Path Filtering via Sysctl

We deployed `/etc/sysctl.d/60-bgp-anycast.conf` to guarantee Loose Reverse Path Filtering across all network namespaces:

```ini
# /etc/sysctl.d/60-bgp-anycast.conf
# RFC 3704 Loose Mode for multi-homed BGP routing

net.ipv4.ip_forward = 1
net.ipv4.conf.default.rp_filter = 2
net.ipv4.conf.all.rp_filter = 2

# Explicit per-interface overrides to prevent distribution defaults from taking over
net.ipv4.conf.eth0.rp_filter = 2
net.ipv4.conf.eth1.rp_filter = 2
net.ipv4.conf.dummy0.rp_filter = 2

# Log Martian packets to dmesg for rapid debugging
net.ipv4.conf.all.log_martians = 1
```

We applied the changes immediately: `sysctl --system`.

### 2. Persistent Dummy Interface via systemd-networkd

To ensure that anycast IP addresses remain bound across network reloads and daemon restarts, we codified `dummy0` into `systemd-networkd`:

Device definition (`/etc/systemd/network/10-dummy-anycast.netdev`):

```ini
[NetDev]
Name=dummy0
Kind=dummy
```

Network configuration (`/etc/systemd/network/10-dummy-anycast.network`):

```ini
[Match]
Name=dummy0

[Network]
Address=94.156.238.80/32
Address=94.156.238.25/32
Address=2a12:9b00:b00b:80::1/128
Address=2a12:9b00:b00b:25::1/128
```

### 3. Production BIRD 2.x Configuration

Here is the production routing configuration in `/etc/bird/bird.conf` handling multi-homing across Cogent and Hurricane Electric:

```bird
router id 94.156.238.1;
define OWNAS = 214806;

protocol device {
    scan time 5;
}

# Announce our anycast address space
protocol static anycast_routes {
    ipv4;
    route 94.156.238.0/24 reject;
}

# Template for transit providers
template bgp transit {
    local as OWNAS;
    multihop;
    ipv4 {
        import all;
        export filter {
            if proto = "anycast_routes" then {
                # Add BGP communities if necessary
                accept;
            }
            reject;
        };
    };
}

# Cogent Uplink (eth0)
protocol bgp cogent from transit {
    neighbor 198.51.100.1 as 174;
    interface "eth0";
}

# Hurricane Electric Uplink (eth1)
protocol bgp hurricane from transit {
    neighbor 203.0.113.1 as 6939;
    interface "eth1";
}
```

### 4. Caddy Web Server Binding

With the persistent `dummy0` in place, Caddy was configured to bind cleanly to the public anycast address (`/etc/caddy/Caddyfile`):

```caddy
{
    admin off
    auto_https off
}

http://94.156.238.80:80 {
    bind 94.156.238.80

    header {
        X-Edge-POP "kc"
        X-Routing-AS "AS214806"
        Strict-Transport-Security "max-age=31536000;"
    }

    reverse_proxy 10.99.0.80:8080 {
        header_up Host {host}
        header_up X-Real-IP {remote_host}
    }
}
```

Once `rp_filter` was switched to Loose Mode (`2`) and Caddy bound to `dummy0`, `curl http://94.156.238.80/` returned HTTP 200 within 14 milliseconds.

---

## Lessons Learned & Best Practices

1. **BGP Routers Must Run Loose Reverse Path Filtering**: In a multi-homed autonomous system, asymmetric routing is normal and expected. Running `rp_filter = 1` guarantees silent packet drops whenever inbound and outbound transit paths diverge. Always set `rp_filter = 2`.
2. **Beware of `systemd-networkd` Sysctl Resets**: Restarting network services or upgrading system packages often reapplies system defaults, silently reverting `rp_filter` to Strict Mode. Pin sysctl settings explicitly inside `/etc/sysctl.d/`.
3. **Never Rely on Ad-Hoc `ip link` Commands in Production**: Network interfaces that host production IP addresses must be declaratively codified in configuration files (`.netdev` and `.network`). An interface that fails to recreate after a reboot turns a routine maintenance window into a multi-hour outage.
4. **Log Martian Packets During Debugging**: Enabling `net.ipv4.conf.all.log_martians = 1` immediately prints kernel drops to `dmesg`, eliminating the guesswork when packets vanish between the NIC and user-space sockets.

---

## References

- [RFC 3704: Ingress Filtering for Multihomed Networks](https://datatracker.ietf.org/doc/rfc3704/)
- [Linux Kernel Documentation: IP Sysctl and Reverse Path Filtering](https://docs.kernel.org/networking/ip-sysctl.html)
- [BIRD Internet Routing Daemon: User's Guide (BGP Multi-Homing)](https://bird.network.cz/doc/bird.html)
- [systemd-networkd: Network and NetDev Configuration](https://manpages.debian.org/testing/systemd/systemd.network.5.en.html)
