---
title: The Great Spam House Mail Recovery, Part 1: The Anycast Black Hole
short_title: Spam House Mail Recovery, Pt 1
subtitle: Diagnosing the Anycast Black Hole
date: 2026-09-05
slug: spam-house-part-1-the-anycast-black-hole
tags: networking, bgp, email, stalwart, anycast, postmortem
series: Spam House Mail Recovery
series_order: 1
---

# The Great Spam House Mail Recovery, Part 1: The Anycast Black Hole

*Part 1 of the Spam House Mail Saga: **Part 1: The Anycast Black Hole** | [Part 2: The Reverse Proxy SPA War →](/blog/spam-house-part-2-the-reverse-proxy-war)*

## Table of Contents
1. [The Setup: Running AS214806](#the-setup-running-as214806)
2. [The Incident: When Decommissioning a Server Murders Your Mail](#the-incident-when-decommissioning-a-server-murders-your-mail)
3. [The Investigation: Is Stalwart Even Alive?](#the-investigation-is-stalwart-even-alive)
4. [Tracing the Packets: The Missing Host Pin](#tracing-the-packets-the-missing-host-pin)
5. [The Bird BGP Fix](#the-bird-bgp-fix)
6. [Verification: The First Ping from Gmail](#verification-the-first-ping-from-gmail)
7. [Lessons Learned](#lessons-learned)
8. [References](#references)

---

## The Setup: Running AS214806

I run [AS214806](https://bgp.tools/as/214806) (femboy cyber networks llc). We announce our own IPv4 (`94.156.238.0/24`) and IPv6 (`2a12:9b00:b00b::/48`) address space from eight edge Points of Presence (POPs) dotted around North America and Europe: Kansas City (`kc`), New York (`ny`), Las Vegas (`lv`), Frankfurt (`de`), Zurich (`ch`), and a few others.

```text
               +----------------------------------------+
               |            Public Internet             |
               +----------------------------------------+
                  /          |              |          \
                 /           |              |           \
           BGP /24       BGP /24        BGP /24       BGP /24
               v             v              v             v
          +---------+   +---------+    +---------+   +---------+
          | Edge NY |   | Edge LV |    | Edge DE |   | Edge KC |
          +---------+   +---------+    +---------+   +---------+
               \             |              |            /
                \            |              |           /
                 +---> WireGuard Full-Mesh iBGP <------+
                                    |
                                    v
                         [Incus Container on KC]
                         Stalwart Mail + Storage
```

Every single POP announces the exact same `/24` prefix over eBGP to upstream transit providers. That's standard BGP anycast: when a user in Europe queries our authoritative DNS or loads a public frontend, BGP routes them to Frankfurt or Zurich; when someone in California connects, they land on Las Vegas.

Anycast is fantastic for stateless UDP protocols like DNS, or idempotent HTTP reverse proxies. But **stateful email storage is not anycast**. You cannot comfortably split an incoming SMTP TCP session across eight global POPs unless they share a synchronized distributed mail spool (which is a recipe for distributed lock hell).

So, for our mail cluster (`spam.house`), we use a **host pin**. We carved out a specific IP from our allocation—`94.156.238.25`—and designated it as the sole MX target for all our hosted mailboxes. Inside Kansas City (`kc`), an Incus container runs [Stalwart Mail](https://stalw.art/), listening for inbound SMTP traffic and managing the storage database.

---

## The Incident: When Decommissioning a Server Murders Your Mail

A few days earlier, we completed a hardware upgrade and decommissioned an old physical server nicknamed `sockpuppet`. It had lived under a desk, served its purpose, and was cleanly powered down, wiped, and unplugged. Everything else seemed fine. Web traffic flowed, DNS queries resolved across all POPs, and SSH connectivity was intact.

Then the reports started rolling in:
> *"Hey, emails sent to admin@femboy.zip and admin@spam.house aren't delivering. They're bouncing with connection timeouts."*

I sent a test email from a personal Gmail account. Nothing. Ten minutes later, still nothing. Another test from a Fastmail account timed out at the `RCPT TO` phase. But bizarrely, an email sent from a server physically co-located in the midwest went through instantly.

The symptom was infuriating: **inbound email delivery was completely non-deterministic**. Roughly 85% of incoming mail vanished into thin air, while 15% landed in the inbox as if nothing was wrong.

---

## The Investigation: Is Stalwart Even Alive?

When an email server starts dropping messages intermittently, the first instinct is to suspect software rot:
- Is Stalwart's RocksDB database locked or corrupted?
- Did an automatic update break SMTP listener bindings?
- Is the internal queue worker deadlocked on DKIM verification or SpamAssassin milters?

I jumped into the Kansas City host and stepped into the mail container:
```bash
incus exec mail -- bash
```

Stalwart doesn't log every routine event to syslog by default; it maintains its internal state and diagnostics in its embedded key-value store. Rather than blindly restarting the daemon (which destroys forensic state), we checked the internal table row counts directly:

```text
Table 's_email': 1751 rows
Table 'p':       5876 rows
Table 'q':          7 rows
```

The queue table `q` had 7 rows sitting in it. Were they stuck, or was the queue processing normally?
To test without needing administrator authentication credentials, we crafted a raw SMTP injection directly against `localhost:25`:

```bash
swaks --to admin@femboy.zip --from test@internal.local --server 127.0.0.1:25
```

We immediately re-checked the store counters:
```text
Table 's_email': 1751 -> 1752
Table 'p':       5876 -> 5880
```

The message store incremented immediately. The message filed into `admin@femboy.zip`'s mailbox without a hitch. 

**Stalwart was completely innocent.** Inbound SMTP acceptance, parser pipeline, and local disk writes were 100% operational. The issue wasn't the mail daemon. Traffic was simply never reaching Kansas City from the outside world.

---

## Tracing the Packets: The Missing Host Pin

If the mail server is alive and accepting connections on `kc`, why were external MTAs timing out?

Let's revisit how packet routing works on an anycast network. When Gmail wants to deliver a message to `admin@femboy.zip`, it does an MX lookup:
```sh
$ dig +short MX femboy.zip
10 mail.spam.house.

$ dig +short A mail.spam.house
94.156.238.25
```

Google's edge router in Virginia looks at its global BGP routing table for `94.156.238.25`.
- What does it see? It sees `94.156.238.0/24` announced by our New York (`ny`) POP.
- So Google sends the TCP SYN packet to New York!

Now, the packet arrives at our New York edge router on `eth0`.
New York needs to know: *"Where do I forward packets addressed to 94.156.238.25?"*

We checked the routing table in New York:
```bash
ip route get 94.156.238.25
```

And there was the smoking gun:
```text
94.156.238.25 dev dummy0 proto kernel scope link src 94.156.238.1
```

**New York had no specific route for `94.156.238.25`!** Because New York originates the entire `/24` aggregate on its local `dummy0` interface to announce it via BGP, its kernel routing table assumed that *any* address in `94.156.238.0/24` was local. It tried to ARP for `94.156.238.25` on its local dummy interface, got no reply, and dropped the packet on the floor.

How had this ever worked in the first place?

We checked our historical Git repository for our Bird BGP configurations. And there it was:
Historically, our retired bare-metal machine **`sockpuppet`** had been configured to originate a `/32` host route (`94.156.238.25/32`) and broadcast it across our internal iBGP WireGuard mesh to all other POPs.

Under BGP and IP routing rules, **longest prefix match always wins**:
- A `/32` route beats a `/24` aggregate every single time.
- When `sockpuppet` was announcing `94.156.238.25/32`, New York, Frankfurt, and Las Vegas saw the `/32` via iBGP pointing across the WireGuard tunnel to Kansas City, and forwarded the packet appropriately.
- When we pulled the plug on `sockpuppet`, the `/32` route evaporated from the mesh.
- Every edge POP fell back to its local `/24` aggregate and blackholed inbound mail!

The only reason 15% of mail had still been delivering was because senders whose BGP transit happened to hand packets directly to Kansas City hit the machine where the container actually lived.

---

## The Bird BGP Fix

The fix was straightforward, but it had to be made rock-solid so that no single node decommissioning could ever cause this again. **Kansas City itself had to originate the host pin.**

On `kc`, we opened `/etc/bird/bird.conf` and added explicit static route definitions for both IPv4 and IPv6 mail host pins:

```bird
# Define dedicated static host pins for unicast services
protocol static mail_host_pin {
    ipv4 { preference 200; };
    route 94.156.238.25/32 reject;
}

protocol static mail_host_pin_v6 {
    ipv6 { preference 200; };
    route 2a12:9b00:b00b:25::/128 reject;
}
```

Next, we ensured that these host pins are exported into our internal iBGP mesh filter:

```bird
filter ibgp_mesh_export {
    # Export our unicast service pins across the internal mesh
    if proto = "mail_host_pin" then accept;
    if proto = "mail_host_pin_v6" then accept;

    # Do not leak the full /24 across internal mesh unless required
    if net = 94.156.238.0/24 then reject;

    # Standard internal mesh export policies
    if source = RTS_STATIC then accept;
    if source = RTS_BGP then accept;
    reject;
}
```

We told Bird to reload:
```bash
birdc configure
```

And verified the export:
```bash
birdc show route export ibgp_ny
```

```text
BIRD 2.15.1 ready.
94.156.238.25/32     unicast [mail_host_pin 03:42:10] * (200)
                     reject
```

Over on the New York node (`ny`), we re-checked `ip route get`:
```bash
ip route get 94.156.238.25
```

```text
94.156.238.25 via 10.222.1.1 dev wg-mesh src 10.222.2.1 uid 0
    cache
```

Instant relief. New York now knew that any packet landing on its edge for `94.156.238.25` must be encapsulated over `wg-mesh` and delivered straight to Kansas City.

---

## Verification: The First Ping from Gmail

With the BGP mesh converged, we triggered an external test run from three distinct geographic locations:
1. A Gmail probe from North America (Ashburn, VA).
2. A Fastmail probe from Europe (Amsterdam).
3. A local CLI injection via an external VPS in Frankfurt.

Within 4 seconds, the logs inside the Stalwart container erupted:
```text
2026-09-05T03:44:12.102Z [INFO]  [SMTP:inbound] Connection accepted from 209.85.216.178 (mail-pj1-f178.google.com) to 94.156.238.25:25
2026-09-05T03:44:12.450Z [INFO]  [SMTP:inbound] Received message from <sender@gmail.com> to <admin@femboy.zip>
2026-09-05T03:44:12.610Z [INFO]  [STORE] Message successfully written to s_email: row 1753
```

The store counters climbed:
```text
Table 's_email': 1752 -> 1753
Table 'p':       5880 -> 5884
```

External mail delivery was completely restored.

---

## Lessons Learned

1. **Anycast Aggregates Will Swallow Unicast IPs**: If an edge router announces an aggregate prefix (like `/24`) to the world, its local kernel thinks it owns every IP in that range unless a more specific `/32` route explicitly overrides it.
2. **Never Let Decommissioned Hardware Hold Orphaned Routes**: If a host pin was originated on a machine slated for teardown, the route origination must be migrated and tested *before* uncoupling the server from the rack.
3. **Black-Box Database Counters Save Time**: When debugging complex distributed applications, finding a way to observe persistent state change (like table row increments) proves the application layer is healthy in seconds, sparing you hours of pointless config tweaking.

---

*Continue to **[Part 2: The Reverse Proxy SPA War →](/blog/spam-house-part-2-the-reverse-proxy-war)** to read how fixing inbound mail uncovered a hilarious battle between Caddy, Single-Page App OAuth redirections, and SMTP EHLO compliance.*

## References
- [Bird 2.x Documentation](https://bird.network.cz/?get_doc&v=20&f=bird.html)
- [RFC 4786: Operation of Anycast Services](https://datatracker.ietf.org/doc/rfc4786/)
- [Stalwart Mail Server Architecture](https://stalw.art/docs/)
- [AS214806 BGP Routing Profile](https://bgp.tools/as/214806)
