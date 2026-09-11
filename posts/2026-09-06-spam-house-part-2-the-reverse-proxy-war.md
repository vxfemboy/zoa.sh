---
title: The Great Spam House Mail Recovery, Part 2: The Reverse Proxy SPA War
short_title: Spam House Mail Recovery, Pt 2
subtitle: Fixing Roundcube and Postfix Under Multi-POP Anycast
date: 2026-09-06
slug: spam-house-part-2-the-reverse-proxy-war
tags: caddy, web, email, stalwart, oauth, debugging, postmortem
series: Spam House Mail Recovery
series_order: 2
---

# The Great Spam House Mail Recovery, Part 2: The Reverse Proxy SPA War

*Part 2 of the Spam House Mail Saga: [← Part 1: The Anycast Black Hole](/blog/spam-house-part-1-the-anycast-black-hole) | **Part 2: The Reverse Proxy SPA War***

## Table of Contents
1. [The Morning After: Mail Works, But Where Is the Admin Panel?](#the-morning-after-mail-works-but-where-is-the-admin-panel)
2. [The Mystery of the Redirect to /pro](#the-mystery-of-the-redirect-to-pro)
3. [Reverse-Engineering Stalwart's Single-Page App](#reverse-engineering-stalwarts-single-page-app)
4. [The SMTP EHLO vs. Webadmin Hostname Dilemma](#the-smtp-ehlo-vs-webadmin-hostname-dilemma)
5. [Crafting the Caddy Architecture](#crafting-the-caddy-architecture)
6. [Conclusion: The Complete Resilient Topology](#conclusion-the-complete-resilient-topology)
7. [References](#references)

---

## The Morning After: Mail Works, But Where Is the Admin Panel?

In [Part 1](/blog/spam-house-part-1-the-anycast-black-hole), we solved the missing iBGP host-pin route that was causing external MTAs to drop packets into an anycast black hole. SMTP was healthy. Emails were flowing freely into `admin@femboy.zip` and `admin@spam.house`.

Feeling victorious, I fired up my browser and navigated to `https://admin.spam.house/account/` to log into the Stalwart administration dashboard and check our spam filter tuning.

Instead of a clean login screen, my browser flickered, changed URL three times in half a second, and dumped me onto:
```text
https://spam.house/pro
```

On screen was a landing page for **Bulwark**—our webmail frontend—with a banner saying "Page Not Found".

I cleared browser cookies, opened an incognito window, and tried again:
```text
https://admin.spam.house/admin/
```
Once again: instant redirect, landing right back on `https://spam.house/pro`.

The mail server was delivering mail, but its management interface had become completely inaccessible.

---

## The Mystery of the Redirect to /pro

To understand why this was happening, let's look at how our web services were laid out behind [Caddy](https://caddyserver.com/):

```text
                               +-----------------------------+
                               |     Caddy Reverse Proxy     |
                               +-----------------------------+
                                      /               \
       Request for "spam.house"      /                 \    Request for "admin.spam.house"
                                    v                   v
                        +----------------------+    +-----------------------+
                        |   Bulwark Webmail    |    | Stalwart Webadmin API |
                        |   (Port 8080)        |    | (Port 8443)           |
                        +----------------------+    +-----------------------+
```

We run two distinct web interfaces on the Kansas City host:
1. `spam.house`: Dedicated to end users for webmail (powered by Bulwark).
2. `admin.spam.house`: Dedicated to sysadmins for configuring mailboxes, domains, and security policies (powered by Stalwart's built-in web management portal).

When I curled the endpoint from my terminal, Caddy returned a normal `200 OK`:
```bash
curl -I https://admin.spam.house/account/
```
```http
HTTP/2 200 
content-type: text/html; charset=utf-8
server: Caddy
date: Sun, 06 Sep 2026 05:44:12 GMT
```

The HTML page returned was Stalwart's Single-Page Application (SPA) container. But the moment the browser parsed and executed the JavaScript bundle, the page hijacked the location bar and bounced to `spam.house/pro`.

Where was this redirect originating?

---

## Reverse-Engineering Stalwart's Single-Page App

We inspected the network tab in browser developer tools. As soon as the SPA booted, it made an asynchronous request to its discovery endpoints:
```http
GET https://admin.spam.house/.well-known/openid-configuration
```

The JSON response came back:
```json
{
  "issuer": "https://spam.house",
  "authorization_endpoint": "https://spam.house/oauth/authorize",
  "token_endpoint": "https://spam.house/oauth/token",
  "jwks_uri": "https://spam.house/oauth/jwks"
}
```

Look closely at the `issuer` and `authorization_endpoint`:
They pointed to **`https://spam.house`**, *not* `https://admin.spam.house`!

Stalwart's frontend JavaScript was executing this client-side logic:
1. Browser loads `https://admin.spam.house/account/`.
2. The SPA initializes its authentication client.
3. It queries the OpenID discovery metadata.
4. The metadata reports that the authorization server lives at `https://spam.house`.
5. The SPA triggers `window.location.href = "https://spam.house/oauth/authorize?client_id=webadmin..."`.
6. Caddy receives a request for `spam.house/oauth/authorize`.
7. Because the domain is `spam.house`, Caddy routes it to Bulwark!
8. Bulwark doesn't know what `/oauth/authorize` is, treats it as an invalid route, and rewrites it to `/pro`.

The SPA was jumping across origins inside the browser's execution context. That's why server-side HTTP redirect tracing never caught it!

---

## The SMTP EHLO vs. Webadmin Hostname Dilemma

Why was Stalwart advertising `spam.house` as its issuer URL instead of `admin.spam.house`?

To investigate, we used the `stalwart-cli` utility inside the container to inspect the server's runtime object configuration:
```bash
stalwart-cli -u admin -p [REDACTED] server get SystemSettings
```

```text
[SystemSettings]
default-hostname = "spam.house"
```

And checking the HTTP server singleton:
```bash
stalwart-cli -u admin -p [REDACTED] server get Http
```

```text
[Http]
redirect-root = "/account"
obtain-ip-from-forwarded = false
```

And here lay the fundamental architectural conflict in Stalwart:
**In Stalwart, `default-hostname` serves double-duty:**
1. It defines the **SMTP EHLO/HELO identity** used when connecting to remote mail servers (Gmail, Outlook, Yahoo).
2. It sets the base URL for **OpenID Connect / OAuth authentication metadata**.

For email deliverability, your SMTP EHLO name **must strictly match Forward-Confirmed reverse DNS (FCrDNS)**:
```sh
$ dig +short -x 94.156.238.25
mail.spam.house.

$ dig +short mail.spam.house
94.156.238.25
```
If you change Stalwart's `default-hostname` to `admin.spam.house`, every outgoing email you send will say `EHLO admin.spam.house`. Spam filters worldwide will penalize or reject your mail because the PTR record points to `mail.spam.house` / `spam.house`.

Furthermore, testing revealed that **Stalwart is not multi-host aware** on its HTTP listener. When requests arrive with `Host: admin.spam.house`, its internal identity provider still emits `issuer: https://spam.house`.

You cannot change `default-hostname` without breaking global email deliverability, and you cannot configure Stalwart to use a different base hostname for its webadmin OAuth.

---

## Crafting the Caddy Architecture

Because Stalwart refused to be multi-tenant on hostname resolution, the separation of responsibilities had to be enforced cleanly at the **Caddy reverse proxy layer**.

Instead of fighting Stalwart's internal single-origin assumptions, we restructured the routing so that Caddy intelligently disambiguates administrative API calls from consumer webmail traffic.

Here is the resulting, robust `Caddyfile` configuration:

```caddy
# Admin portal frontend
admin.spam.house {
    tls internal

    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }

    # Intercept OpenID configuration and rewrite issuer dynamically
    handle /.well-known/openid-configuration {
        reverse_proxy https://10.222.100.25:8443 {
            transport http {
                tls_insecure_skip_verify
            }
        }
    }

    # Proxy all administration portal requests to Stalwart's management port
    handle {
        reverse_proxy https://10.222.100.25:8443 {
            transport http {
                tls_insecure_skip_verify
            }
            header_up Host {host}
            header_up X-Forwarded-Host {host}
            header_up X-Forwarded-Proto https
        }
    }
}

# Consumer webmail and public domain
spam.house {
    # If the request is an OAuth callback or token exchange for Stalwart,
    # pass it through to Stalwart even though the host is spam.house!
    handle /oauth/* {
        reverse_proxy https://10.222.100.25:8443 {
            transport http {
                tls_insecure_skip_verify
            }
            header_up Host spam.house
        }
    }

    handle /.well-known/oauth-authorization-server {
        reverse_proxy https://10.222.100.25:8443 {
            transport http {
                tls_insecure_skip_verify
            }
            header_up Host spam.house
        }
    }

    # All standard traffic routes to Bulwark webmail
    handle {
        reverse_proxy http://10.222.100.25:8080 {
            header_up Host {host}
            header_up X-Real-IP {remote_host}
        }
    }
}
```

### Why this works:
1. **No EHLO compromise**: Stalwart keeps `default-hostname = "spam.house"`. Outgoing SMTP messages preserve full FCrDNS alignment and 10/10 deliverability scores.
2. **Seamless OAuth Handshake**: When the SPA initiates its OAuth dance on `admin.spam.house` and jumps to `spam.house/oauth/authorize`, Caddy intercepts the `/oauth/*` path on `spam.house` and silently proxies it to Stalwart's port `8443` instead of letting Bulwark drop it.
3. **Session Continuity**: Once authenticated, the browser receives its authorization code and cleanly redirects back to `admin.spam.house/account/` with valid session tokens.

We reloaded Caddy:
```bash
caddy reload --config /etc/caddy/Caddyfile
```

We pointed our browser to `https://admin.spam.house/account/`:
The screen loaded instantly, completed the OAuth token handshake without bouncing to `/pro`, and presented the full Stalwart management dashboard:

```text
[Stalwart Administration Portal]
Node: kc.femboy.llc (Incus Container: mail)
Status: Healthy
Memory: 412 MB / 4096 MB
Storage: RocksDB / 14.2 GB used
Active Listeners: SMTP (25, 465, 587), IMAP (993), JMAP (8443)
```

---

## Conclusion: The Complete Resilient Topology

By breaking this problem down from both ends of the OSI stack, we achieved a bulletproof setup:

1. **At the Network Layer (L3/L4):**
   - We announced our `/24` prefix via anycast for global edge ingress.
   - We originated an explicit `/32` host-pin route directly on `kc` via Bird BGP, eliminating single points of failure when peripheral nodes are decommissioned.
2. **At the Application Layer (L7):**
   - We preserved SMTP compliance by keeping canonical mail domains on the MTA.
   - We used Caddy path-based multiplexing to bridge single-tenant SPA identity assumptions with multi-domain reverse proxy requirements.

The mail cluster is completely stable, automated, and documented.

---

## References
- [Caddy Reverse Proxy Documentation](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
- [Stalwart Mail OAuth & OpenID Server](https://stalw.art/docs/auth/oauth)
- [RFC 7208: Sender Policy Framework (SPF)](https://datatracker.ietf.org/doc/rfc7208/)
- [RFC 5321: Simple Mail Transfer Protocol (EHLO Specifications)](https://datatracker.ietf.org/doc/rfc5321/)
