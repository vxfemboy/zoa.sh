---
title: The Crypto Transaction That Got Killed by xfinitywifi
date: 2026-08-17
slug: crypto-transaction-killed-by-xfinitywifi
tags: networking, linux, wifi, networkmanager, debugging
---

# The Crypto Transaction That Got Killed by xfinitywifi

## The Problem

I was in the middle of broadcasting a time-sensitive crypto transaction from my laptop running Void Linux when `wlan0` completely vanished. The transaction failed. 

This wasn't the first time `wlan0` had flapped, but when a dropped connection costs you real money, you stop tolerating it as "just Linux Wi-Fi things" and sit down to find the exact root cause. The initial suspects were obvious:
1. Leftover `dhcpcd` or `wpa_supplicant` conflicts fighting the NetworkManager daemon.
2. A custom WireGuard runit service re-configuring routing tables or flushing links.
3. NetworkManager dispatcher hook scripts acting up on link transitions.

## The Journey

Following a strict rule of investigating root causes before throwing random config tweaks around, we pulled the system logs (`/var/log/socklog/daemon/current`) around the failure timestamp `17:46:56`.

Right away, the log showed NetworkManager executing dispatcher events:
```text
daemon.warn: nm-dispatcher: req:1 'dhcp4-change' [wlan0]: script /etc/NetworkManager/dispatcher.d/90-mount-castletcp failed (not executable by owner)
```

The dispatcher script was throwing a harmless warning (permissions weren't `+x`), but digging deeper into the state transitions preceding it revealed the actual smoking gun between `17:41:38` and `17:41:43`:
```text
<info>  [17:41:38] device (wlan0): supplicant interface state: completed -> disconnected
<info>  [17:41:39] device (wlan0): Activation: starting connection 'xfinitywifi'
<info>  [17:41:40] device (wlan0): state change: config -> ip-config
<info>  [17:41:42] device (wlan0): state change: ip-config -> activated
<info>  [17:41:43] device (wlan0): deactivating connection 'xfinitywifi'
<info>  [17:41:43] device (wlan0): Activation: starting connection 'ItBurnsWhenIP'
```

My Wi-Fi hadn't dropped out due to bad signal or hardware faults. **NetworkManager was intentionally jumping off my home AP and roaming to a neighbor's open public hotspot**, realizing it couldn't route properly, and jumping back three seconds later.

Why would NetworkManager abandon a 5GHz WPA2 connection with great RSSI to hop onto an unauthenticated open network?

We checked the connection profiles and autoconnect priorities:
```bash
nmcli -f NAME,TYPE,AUTOCONNECT,AUTOCONNECT-PRIORITY connection show
```

The output was damning:
- `xfinitywifi`: `autoconnect=yes`, `autoconnect-priority=6`
- `ItBurnsWhenIP` (home SSID): `autoconnect=yes`, `autoconnect-priority=0`

By default, NetworkManager defaults newly created connections to priority `0`. At some point in the past, connecting to an open `xfinitywifi` network had registered it with priority `6` (or higher precedence from a captive portal template). Whenever the 5GHz signal from the home router experienced transient interference, NetworkManager scanned, saw `xfinitywifi` in the beacon list with priority `6 > 0`, and immediately roamed away.

## The Solution

The fix required three decisive steps:

1. **Lock the home connection priority way higher**:
```bash
nmcli connection modify "ItBurnsWhenIP" connection.autoconnect-priority 100
```

2. **Disable autoconnect for predatory captive portals**:
```bash
nmcli connection modify "xfinitywifi" connection.autoconnect no
nmcli connection modify "xfinitywifi" connection.autoconnect-priority -100
```

3. **Pin the home profile directly to the trusted BSSID**:
Prevent rogue roaming by binding the profile directly to the access point's MAC address:
```bash
nmcli connection modify "ItBurnsWhenIP" 802-11-wireless.bssid "84:78:48:C8:95:55"
```

Additionally, the lingering dispatcher script warning was fixed with `chmod +x /etc/NetworkManager/dispatcher.d/90-mount-castletcp`.

## Lessons Learned

- **Check connection priorities before blaming drivers**: When Wi-Fi seems to drop randomly for 3–5 seconds, check `nmcli connection show` for rogue networks with higher `autoconnect-priority`.
- **Open networks are sticky traps**: Never leave open networks set to auto-connect on mobile workstations. NetworkManager treats high-priority open SSIDs as preferred egress paths over lower-priority secured networks.
- **BSSID pinning prevents unwanted AP roaming**: If you have a stationary desk or reliable AP, pinning your wireless profile to the specific BSSID eliminates edge-of-cell roaming jitter completely.

## References

- [NetworkManager.conf Documentation](https://networkmanager.dev/docs/api/latest/NetworkManager.conf.html)
- [nmcli Connection Settings](https://networkmanager.dev/docs/api/latest/nm-settings-nmcli.html)
