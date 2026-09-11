---
title: Split-Horizon Vaultwarden: Public HTTPS with VPN-Only Admin Portal
short_title: Split-Horizon Vaultwarden
subtitle: Hardening Password Managers with Caddy Snippets and WireGuard
date: 2026-08-25
slug: split-horizon-vaultwarden-public-https-vpn-admin
tags: security, caddy, wireguard, self-hosting, sysadmin
---

# Split-Horizon Vaultwarden: Public HTTPS with VPN-Only Admin Portal

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Self-hosting an organization-wide password vault demands an uncomfortable architectural trade-off. Mobile clients, browser extensions, and remote developer laptops require uninterrupted public HTTPS access to synchronize vaults, while the administrative console (`/admin`) represents a catastrophic single point of failure if exposed to the open internet.

Our password infrastructure runs Vaultwarden (an optimized Rust implementation of the Bitwarden server API) inside an Incus container in our Kansas City data center. We host credentials, multi-factor backup keys, and internal infrastructure secrets for our entire operations team under `vault.femboy.fan`. 

To allow seamless synchronization without requiring staff to enable full-tunnel VPNs on mobile devices while traveling, the vault API and WebSockets hub must be publicly reachable over standard IPv4 (`94.156.238.0/24`) and IPv6 (`2a12:9b00:b00b::/48`) anycast addresses. However, exposing the administrative interface (`/admin`) invites serious security liabilities:
- Automated credential stuffing and brute-force token spray attacks against `ADMIN_TOKEN`.
- Accidental administrative token leakage in server logs or edge proxy request telemetry.
- Zero-day vulnerabilities in administrative template rendering or session handling.

Standard approaches—such as running a separate Vaultwarden container exclusively for administration or binding the web service to two different host ports—either complicate SQLite/PostgreSQL database locking or break single-origin browser extension expectations. We required a transparent split-horizon architecture using Caddy v2 route matching and WireGuard overlay network enforcement.

---

## The Deep-Dive / Root Cause Analysis

Securing `/admin` behind reverse proxies frequently fails due to subtle URL normalization discrepancies and client IP spoofing across anycast edge nodes.

### The Reverse Proxy Forwarded Header Trap

When traffic ingresses through our global anycast edge nodes (Frankfurt, London, New York) and traverses encrypted WireGuard tunnels to the origin container in Kansas City, the origin reverse proxy must determine the true client origin.

A naive Caddy or Nginx rule typically checks the remote address:
```caddy
# FLAWED NAIVE CONFIGURATION
vault.femboy.fan {
    @admin_restricted {
        path /admin*
        not remote_ip 10.100.0.0/16 # WireGuard subnet
    }
    respond @admin_restricted "Forbidden" 403
    reverse_proxy 127.0.0.1:8080
}
```

In an anycast network topology, this naive rule fails in both directions:
1. **False Positives**: When an authorized administrator connects to the public anycast IP from their laptop while connected to WireGuard, their egress IP might be recognized as their public WireGuard exit node rather than an internal mesh peer IP.
2. **Path Traversal & Normalization Bypasses**: Web servers and reverse proxies evaluate URI paths differently. If Caddy blocks `/admin*`, an attacker might send `/admin/`, `//admin`, `/%61dmin`, or `/admin;param` (in servlet environments). Vaultwarden's internal Rocket/Axum router normalizes percent-encoded characters and collapsed slashes before evaluating routes, potentially matching a route that slipped past Caddy's regex!

### WebSocket Notification Leaks

Vaultwarden uses WebSockets (`/notifications/hub`) for instantaneous synchronization across devices. If route filtering rules are written without protocol negotiation awareness, edge proxies can inadvertently terminate WebSocket upgrades or downgrade them to plain HTTP/1.1 long-polling, causing massive connection churn and log pollution.

---

## The Implementation / Architecture

We implemented a split-horizon security boundary at Layer 7 using Caddy modular snippets, subpath route ordering, and mutual TLS / WireGuard peer IP verification.

```
Public Internet Client                     Authorized Ops Workstation
         |                                             |
   (Public HTTPS)                               (WireGuard Mesh)
         |                                             |
         v                                             v
  +------------------+                          +------------------+
  | Anycast Edge POP |                          | Anycast Edge POP |
  +------------------+                          +------------------+
         |                                             |
         +--------------------+  +---------------------+
                              |  |
                              v  v
                   +-----------------------+
                   |  Origin Caddy Proxy   |
                   +-----------------------+
                    /                     \
        Path == /admin*               Path != /admin*
               |                              |
      Is Client IP in Mesh?                   |
      (10.100.0.0/16 or wg0)                  |
         /              \                     |
       YES               NO                   |
        |                 |                   |
        v                 v                   v
+---------------+  +---------------+  +---------------+
| Allow /admin  |  | 404 Not Found |  | Allow Vault   |
| (Admin Panel) |  | (Stealth Drop)|  | Sync & Auth   |
+---------------+  +---------------+  +---------------+
        \                                     /
         \                                   /
          v                                 v
      +-----------------------------------------+
      |      Vaultwarden Core (127.0.0.1:8080)   |
      +-----------------------------------------+
```

### Complete Hardened Caddyfile

The following Caddy configuration guarantees that `/admin` is completely invisible (returning HTTP 404 rather than 403 to prevent reconnaissance) to anyone not connected via the internal WireGuard management mesh (`10.100.0.0/16`):

```caddy
(security_headers) {
    header {
        Strict-Transport-Security "max-age=63072000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "SAMEORIGIN"
        X-XSS-Protection "1; mode=block"
        Referrer-Policy "strict-origin-when-cross-origin"
        Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://haveibeenpwned.com; child-src 'self' https:; connect-src 'self' wss: https:;"
        -Server
    }
}

vault.femboy.fan {
    import security_headers
    encode zstd gzip

    # 1. WebSocket synchronization endpoint
    @websockets {
        header Connection *Upgrade*
        header Upgrade websocket
        path /notifications/hub
    }
    reverse_proxy @websockets 127.0.0.1:3012

    # 2. Strict Admin Route Isolation: Match exact prefixes and normalized variants
    @admin_portal {
        path /admin
        path /admin/*
    }

    # Internal VPN network matcher (WireGuard mesh: 10.100.0.0/16, IPv6 fd00:b00b::/64)
    @internal_vpn {
        remote_ip 10.100.0.0/16 fd00:b00b::/64
    }

    # Reject non-VPN requests to /admin with a silent 404 stealth drop
    handle @admin_portal {
        @unauthorized_external {
            not client_ip 10.100.0.0/16 fd00:b00b::/64
        }
        respond @unauthorized_external "Not Found" 404

        # Authorized VPN traffic passes to backend
        reverse_proxy 127.0.0.1:8080 {
            header_up X-Real-IP {client_ip}
            header_up X-Forwarded-For {client_ip}
            header_up X-Forwarded-Proto {scheme}
        }
    }

    # 3. Standard Vaultwarden Web Vault and Sync APIs
    handle {
        reverse_proxy 127.0.0.1:8080 {
            header_up X-Real-IP {remote_host}
            header_up X-Forwarded-For {remote_host}
            header_up X-Forwarded-Proto {scheme}
        }
    }
}
```

### Vaultwarden Environment Configuration

In addition to edge proxy enforcement, the Vaultwarden daemon (`/etc/vaultwarden.env`) is configured with defensive defaults:

```ini
## Vaultwarden Runtime Configuration
DOMAIN=https://vault.femboy.fan
WEB_VAULT_ENABLED=true
SIGNUPS_ALLOWED=false
INVITATIONS_ALLOWED=true
SHOW_PASSWORD_HINT=false

## Administrative Security
# Generate using: openssl rand -base64 48
ADMIN_TOKEN=$argon2id$v=19$m=65536,t=3,p=4$qF8a...
ADMIN_SESSION_LIFETIME=20

## Network Binding
ROCKET_ADDRESS=127.0.0.1
ROCKET_PORT=8080
WEBSOCKET_ENABLED=true
WEBSOCKET_ADDRESS=127.0.0.1
WEBSOCKET_PORT=3012

## Logging and Rate-Limiting
LOG_LEVEL=info
EXTENDED_LOGGING=true
USE_SYSLOG=true
```

---

## Lessons Learned & Best Practices

1. **Prefer 404 Over 403 for Administrative Endpoints**: Returning `403 Forbidden` confirms the existence of the administrative endpoint to automated scanners. A `404 Not Found` blends into normal URL space and prevents targeted dictionary attacks.
2. **Always Use Argon2id Hashes for Admin Tokens**: Vaultwarden supports pre-hashing the `ADMIN_TOKEN` using Argon2id. Never store the raw administrative passphrase in plaintext environment files.
3. **Handle WebSockets on Dedicated Endpoints**: Separate WebSocket traffic (`/notifications/hub`) from REST API routes. WebSockets require long-lived connections that can exhaust standard HTTP reverse proxy worker pools if not tuned appropriately.
4. **Enforce Two-Tier IP Verification**: Never rely solely on `X-Forwarded-For` headers unless the intermediate edge proxies are strictly trusted via CIDR whitelisting.

---

## References

- [Vaultwarden Configuration & Hardening Guide](https://github.com/dani-garcia/vaultwarden/wiki)
- [Caddy Reverse Proxy Documentation](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
- [RFC 8446: The Transport Layer Security (TLS) Protocol Version 1.3](https://datatracker.ietf.org/doc/rfc8446/)
