---
title: Zero-Downtime Forgejo Migration: Moving Git Repositories Across Domains
short_title: Zero-Downtime Forgejo Migration
subtitle: PostgreSQL Schema Rewrites and Git SSH Host Key Continuity
date: 2026-08-29
slug: zero-downtime-forgejo-domain-migration
tags: git, forgejo, postgresql, migration, devops
---

# Zero-Downtime Forgejo Migration: Moving Git Repositories Across Domains

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Renaming the primary domain of a self-hosted Git forge sounds straightforward until hundreds of automated CI/CD runners, production webhooks, developer SSH key rings, and LFS blob pointers fracture simultaneously. During an infrastructure consolidation, we migrated our central Forgejo instance between two top-level domains without dropping a single active SSH clone or triggering host key verification warnings.

Our self-hosted developer platform initially operated under `git.filmtek.cloud`. It serves as the single source of truth for our infrastructure-as-code repositories, custom kernel configurations, internal Rust crates, and automated deployment manifests. Over two hundred developer workstations and continuous delivery runners in Kansas City, New York, and Frankfurt interacted with the forge over SSH (`git@git.filmtek.cloud:...`) and HTTPS.

As part of our brand consolidation under our anycast network domain (`git.femboy.fan`), we needed to migrate the entire forge:
1. **Zero Git Remote Breakage**: Workstations and CI runners retaining the old `git.filmtek.cloud` remote URLs had to continue pushing and pulling transparently over SSH and HTTPS.
2. **Host Key Continuity**: Users connecting to the new domain must never encounter `WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!`.
3. **Database Integrity**: Forgejo stores fully qualified domain names and webhook URLs across several PostgreSQL relational tables and Git LFS metadata pointers. If these strings did not match the new domain, internal webhook dispatchers, OAuth callbacks, and package registries would fail silently.
4. **Zero Maintenance Window**: Developers were actively committing changes throughout the global migration window. We could not take the database offline or lock the git repositories in read-only mode.

---

## The Deep-Dive / Root Cause Analysis

A naive migration typically involves changing `app.ini`, restarting the service, and pointing a new CNAME record. Doing so in an active production environment causes immediate failures across three architectural layers.

### SSH Host Key Fingerprint Mismatch

When an SSH client connects to `git.femboy.fan` instead of `git.filmtek.cloud`, OpenSSH validates the host key presented by the server against entries in `~/.ssh/known_hosts`.

```text
# ~/.ssh/known_hosts
git.filmtek.cloud,10.100.0.15 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGx...
```

If the new server provisions a new host key, or if OpenSSH serves separate host keys across internal container boundaries, every automated pipeline immediately halts with:
```text
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
IT IS POSSIBLE THAT SOMEONE IS DOING SOMETHING NASTY!
Someone could be eavesdropping on you right now (man-in-the-middle attack)!
```

To eliminate warnings, the SSH server must serve the identical Ed25519 and RSA private keys under both domain names, and SSH client configurations must be primed or aliased seamlessly.

### PostgreSQL Hardcoded URL Traps

Forgejo does not merely compute URLs dynamically on request. To optimize query performance, several database tables store fully qualified URLs at creation time:
- `webhook`: Webhook payload delivery endpoints often point back to internal forge mirrors or CI webhooks.
- `oauth2_application`: Redirect URIs for single sign-on applications hardcode the old origin.
- `lfs_meta_object`: Large File Storage tracking keys and verification digests.
- `user`: Avatar URLs and external profile links.
- `repository`: Avatar and mirror upstream sync URLs.

A simple search across the database dump revealed over 4,200 occurrences of the literal string `git.filmtek.cloud`:

```sql
SELECT count(*) FROM webhook WHERE target_url LIKE '%git.filmtek.cloud%';
-- returned: 148

SELECT count(*) FROM oauth2_application WHERE redirect_uris LIKE '%git.filmtek.cloud%';
-- returned: 22
```

Simply updating `ROOT_URL` in `/etc/forgejo/app.ini` left existing webhooks attempting to push events to stale destinations, causing webhook queue buildup and database lock contention.

---

## The Implementation / Architecture

We executed the migration using a three-phase operational blueprint: dual-homed SSH configuration, transactional schema rewrite, and Caddy layer-4/layer-7 split routing.

```
Incoming Traffic (Old & New Domains)
            |
            v
     +--------------+
     | Caddy Ingress |
     +--------------+
      /            \
HTTPS:443         SSH:22
    /                \
   v                  v
[HTTP Layer-7]      [SSH Proxy & Passthrough]
308 Redirect         Dual Host Keys (Ed25519)
   or Proxy                 |
   v                        v
+-------------------------------+
|       Forgejo Container       |
|    (git.femboy.fan)           |
|  - app.ini (ROOT_URL updated) |
|  - PostgreSQL (Regex updated) |
+-------------------------------+
```

### PostgreSQL Transactional Schema Rewrite

We authored a PostgreSQL transaction script using regex substitutions to safely rewrite all internal URLs while preserving data types and jsonb structures:

```sql
BEGIN;

-- 1. Update active webhooks pointing to internal mirrors
UPDATE webhook
SET target_url = regexp_replace(target_url, 'https?://git\.filmtek\.cloud', 'https://git.femboy.fan', 'g')
WHERE target_url LIKE '%git.filmtek.cloud%';

-- 2. Update OAuth2 redirect URIs (array of strings serialized as JSON/text)
UPDATE oauth2_application
SET redirect_uris = regexp_replace(redirect_uris, 'git\.filmtek\.cloud', 'git.femboy.fan', 'g')
WHERE redirect_uris LIKE '%git.filmtek.cloud%';

-- 3. Update external avatar and repository description links
UPDATE "user"
SET avatar = regexp_replace(avatar, 'git\.filmtek\.cloud', 'git.femboy.fan', 'g')
WHERE avatar LIKE '%git.filmtek.cloud%';

-- 4. Update repository clone links in wiki mirrors
UPDATE repository
SET description = regexp_replace(description, 'git\.filmtek\.cloud', 'git.femboy.fan', 'g'),
    original_url = regexp_replace(original_url, 'git\.filmtek\.cloud', 'git.femboy.fan', 'g')
WHERE description LIKE '%git.filmtek.cloud%' OR original_url LIKE '%git.filmtek.cloud%';

-- 5. Validate no orphaned strings remain in critical tables
DO $$
DECLARE
    v_remaining_hooks INT;
BEGIN
    SELECT count(*) INTO v_remaining_hooks FROM webhook WHERE target_url LIKE '%git.filmtek.cloud%';
    IF v_remaining_hooks > 0 THEN
        RAISE EXCEPTION 'Migration aborted: % orphaned webhooks detected', v_remaining_hooks;
    END IF;
END $$;

COMMIT;
```

### Dual Host Key SSH Preservation

To guarantee SSH host key continuity, we extracted the active host keys from the old server and mounted them directly into Forgejo's OpenSSH server configuration (`/etc/ssh/`):

```bash
# Verify host key fingerprints match exactly
ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
# 256 SHA256:XkP8n9... root@mail-core (ED25519)
```

In the Forgejo configuration (`/etc/forgejo/app.ini`), we configured Forgejo's built-in SSH server and root URL:

```ini
[server]
PROTOCOL         = http
DOMAIN           = git.femboy.fan
ROOT_URL         = https://git.femboy.fan/
HTTP_ADDR        = 0.0.0.0
HTTP_PORT        = 3000
SSH_DOMAIN       = git.femboy.fan
SSH_PORT         = 22
START_SSH_SERVER = false ; Using OpenSSH with authorized_keys command
SSH_LISTEN_PORT  = 22
DISABLE_SSH      = false

[repository]
DEFAULT_BRANCH   = master
ENABLE_PUSH_CREATE_USER = true
ENABLE_PUSH_CREATE_ORG  = true
```

### Caddy Reverse Proxy & Smart Git Smart-HTTP Redirects

Git Smart-HTTP protocol requires careful handling. A standard `301 Moved Permanently` on an HTTP POST request can cause legacy Git clients to drop request bodies or fail silently. We implemented HTTP `308 Permanent Redirect` (which preserves POST methods and streaming request bodies) alongside smart SSH proxying:

```caddy
# Legacy domain: preserve git smart-http protocol while migrating web browsers
git.filmtek.cloud {
    # Git Smart-HTTP endpoints: redirect with 308 to preserve POST payloads
    @git_http {
        path /info/refs*
        path /*/git-upload-pack*
        path /*/git-receive-pack*
    }
    handle @git_http {
        redir https://git.femboy.fan{uri} 308
    }

    # Browser UI traffic: standard 301
    handle {
        redir https://git.femboy.fan{uri} 301
    }
}

# New canonical domain
git.femboy.fan {
    encode zstd gzip

    # Git LFS chunk upload limit
    request_body {
        max_size 5GB
    }

    reverse_proxy 10.100.0.15:3000 {
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-For {remote_host}
        header_up X-Forwarded-Proto {scheme}
    }
}
```

---

## Lessons Learned & Best Practices

1. **Use HTTP 308 Instead of 301 for API & Git Endpoints**: RFC 7538 defines `308 Permanent Redirect`, which guarantees that HTTP clients must not change the request method from POST to GET. Older Git clients clobber pushes when receiving a 301.
2. **Preserve Exact SSH Host Keys Across Renames**: Never regenerate SSH host keys when renaming a server. Client known_hosts stores both hostnames and IP addresses against host keys. By maintaining the same key, clients connecting via IP or aliases connect transparently.
3. **Audit Embedded URLs Inside Relational Tables**: Forges and content systems frequently cache absolute canonical URLs in database columns. Always run comprehensive regex audits across dumps before declaring a domain migration complete.
4. **Test Webhook Queues Post-Migration**: Webhooks that fail during migration can back up in task worker queues, eventually exhausting retry pools. Monitor `gitea.log` or `forgejo.log` for recurring HTTP 404/500 delivery attempts.

---

## References

- [Forgejo System Administrator Documentation](https://forgejo.org/docs/latest/admin/)
- [RFC 7538: The Hypertext Transfer Protocol Status Code 308 (Permanent Redirect)](https://datatracker.ietf.org/doc/rfc7538/)
- [Git Internals: Smart HTTP Protocol Wire Specification](https://git-scm.com/docs/http-protocol)
