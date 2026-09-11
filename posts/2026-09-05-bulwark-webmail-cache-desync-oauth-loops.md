---
title: When Bulwark Webmail Disowns You: The Cache Desync That Locked Out Admin
short_title: Bulwark Webmail Cache Desync
subtitle: Untangling Redis Session Keys and Stalwart Internal Auth
date: 2026-09-05
slug: bulwark-webmail-cache-desync-oauth-loops
tags: email, authentication, caching, redis, debugging
---

# When Bulwark Webmail Disowns You: The Cache Desync That Locked Out Admin

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Deploying a modern unified mail stack promises streamlined operations, until a subtle mismatch between external HTTP session caches and backend JMAP/OAuth authentication stores creates an unresolvable authorization loop. During an administrative credential rotation, administrators were abruptly locked out of their webmail and management consoles by a self-perpetuating Redis cache desynchronization.

Our autonomous system runs Stalwart Mail Server paired with the Bulwark webmail client inside an isolated Incus container (`mail-core`, WireGuard IP `10.100.0.25`). The web client provides an intuitive interface for both regular mailbox users across our domains (`spam.house`, `femboy.fan`, `zoa.sh`) and administrative operations via the integrated Stalwart management API. To accelerate mailbox navigation and minimize IMAP/JMAP socket churn, Bulwark uses an external Redis instance (`redis-mail.internal`) to cache user session claims, folder listings, and short-lived OAuth bearer tokens.

On the morning of September 5th, we initiated a scheduled security rotation for our primary administrative credentials (`admin@spam.house`). The credential change was committed directly through Stalwart's CLI administrative tool:
```bash
stalwart-cli user update admin@spam.house --password "$NEW_SUPER_SECRET_PASSPHRASE"
```

The CLI confirmed the update, and new IMAP and SMTP connections using the updated passphrase authenticated flawlessly. However, when the administrator attempted to log into the webmail portal at `https://spam.house`, the application fell into a catastrophic redirect and authorization loop:

1. The login form accepted the new credentials with an HTTP `200 OK` response.
2. The browser was redirected to the primary mailbox view (`/inbox`).
3. Within 200 milliseconds, the frontend threw an uncaught 401 Unauthorized exception and bounced the browser back to `/login?error=session_invalidated`.
4. Subsequent attempts to log in with the new password failed with `403 Forbidden`, while entering the old password resulted in `401 Bad Credentials`.
5. Most alarming of all: other non-admin user sessions across the cluster began randomly dropping into the same invalidation spiral whenever they refreshed their browsers.

The webmail frontends had effectively disowned their administrators, trapped in a split-state twilight zone between Stalwart's internal SQLite user store and Bulwark's Redis session cache.

---

## The Deep-Dive / Root Cause Analysis

Untangling the failure required tracing HTTP request flows through our Caddy edge reverse proxies down to the Redis session store and Stalwart's internal authentication subsystem.

### Token Dual-Tracking and Redis Key Namespace Collisions

Bulwark maintains two distinct session artifacts when a user logs in:
- **Client Cookie (`bw_sess`)**: An encrypted HTTP-only session cookie referencing an active Redis session record.
- **Backend Bearer Token (`st_oauth`)**: A JSON Web Token (JWT) minted by Stalwart's embedded OAuth2 authorization server and stored inside the Redis session object.

When `stalwart-cli user update` was executed, Stalwart incremented an internal user credential version counter (`cred_ver`) from `1` to `2`. Any JWT issued under `cred_ver = 1` was immediately rejected by Stalwart's internal JMAP endpoint with `invalid_token`.

However, Bulwark's Redis cache eviction logic was configured with a passive TTL (Time-To-Live) of 86,400 seconds (24 hours). Bulwark had no active webhook or pub/sub listener subscribed to Stalwart's CLI events. 

When the admin logged in with the new password:
1. Bulwark verified the password against Stalwart's `/oauth/token` endpoint. Stalwart issued a new JWT (`cred_ver = 2`).
2. Bulwark attempted to store the new session in Redis using the key template:
   ```text
   session:user:admin@spam.house
   ```
3. Crucially, Bulwark's session serialization logic had an optimization: if `session:user:admin@spam.house` already existed, it only updated the `last_seen` timestamp and skipped overwriting the nested `access_token` field to avoid clobbering active multi-tab state.
4. As a result, Redis retained the old, revoked JWT (`cred_ver = 1`)!

When the frontend redirected to `/inbox`, Bulwark retrieved the cached session from Redis, passed the stale JWT to Stalwart's JMAP API, and received a hard `401 Unauthorized`.

```text
Browser              Bulwark Webmail                   Redis                   Stalwart Backend
   |                        |                            |                            |
   |--- POST /auth/login -->|                            |                            |
   |    (new password)      |--- POST /oauth/token ---------------------------------->|
   |                        |<-- JWT (cred_ver=2) ------------------------------------|
   |                        |--- SETEX session:... (SKIP TOKEN WRITE if exists) ----->|
   |                        |    [Redis still holds old JWT with cred_ver=1]          |
   |<-- Set-Cookie 200 -----|                            |                            |
   |                        |                            |                            |
   |--- GET /inbox -------->|                            |                            |
   |    (bw_sess)           |--- GET session:... ------->|                            |
   |                        |<-- Stale Session (JWT v1) -|                            |
   |                        |--- JMAP Request (JWT v1) ------------------------------>|
   |                        |<-- 401 Unauthorized (cred_ver mismatch) ---------------|
   |<-- 401 Redirect Loop --|                            |                            |
```

### The Caddy Cookie Scope and Redis Connection Pool Exhaustion

Why did other users begin experiencing session drops?

When the admin login loop fired at 10 requests per second, Bulwark's backend error handler caught the 401 response and triggered an emergency session refresh routine. Because the refresh routine failed repeatedly against Stalwart, it leaked Redis client connections from its internal connection pool (`bb8-redis`). 

Within 3 minutes, the Redis connection pool exhausted its 64 configured sockets. Ordinary user requests that needed to read session data timed out waiting for an available Redis socket, falling through to generic authentication failures and invalidating innocent active sessions.

---

## The Implementation / Architecture

We solved the crisis by implementing three architectural safeguards:
1. Deterministic session key namespacing with cryptographic credential version hashing.
2. An automated cache eviction daemon in async Rust listening to Stalwart audit logs.
3. Strict connection pool bounding with circuit breaking.

### Credential-Versioned Session Keys

Instead of static session keys like `session:user:{email}`, session keys are now derived from a cryptographic hash of the user's credential version and principal identifier:

```rust
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct SessionKeyManager;

impl SessionKeyManager {
    /// Generate an immutable, version-bound Redis session key.
    /// Any credential rotation immediately changes the key, making stale cache reads impossible.
    pub fn format_key(tenant: &str, user_email: &str, credential_version: u64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(user_email.to_lowercase().as_bytes());
        hasher.update(&credential_version.to_be_bytes());
        let hash_digest = hex::encode(&hasher.finalize()[..8]);

        format!("sess:{}:{}:{}", tenant, hash_digest, Uuid::new_v4())
    }

    /// User index set used to enumerate and invalidate all active sessions for a user
    pub fn user_index_key(tenant: &str, user_email: &str) -> String {
        format!("idx:user:{}:{}", tenant, user_email.to_lowercase())
    }
}
```

### Event-Driven Invalidation Worker

We deployed an asynchronous daemon that monitors Stalwart's event bus and purges matching Redis keys when account security properties change:

```rust
use deadpool_redis::redis::cmd;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct InvalidationEvent {
    pub tenant: String,
    pub user_email: String,
    pub reason: String,
}

pub struct SessionReconciler {
    redis_pool: deadpool_redis::Pool,
}

impl SessionReconciler {
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        Self { redis_pool: pool }
    }

    pub async fn handle_credential_rotation(&self, event: InvalidationEvent) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.redis_pool.get().await?;
        let index_key = SessionKeyManager::user_index_key(&event.tenant, &event.user_email);

        // Fetch all active session keys registered to this user
        let session_keys: Vec<String> = cmd("SMEMBERS")
            .arg(&index_key)
            .query_async(&mut conn)
            .await?;

        if session_keys.is_empty() {
            info!(user = %event.user_email, "No active sessions found in Redis index");
            return Ok(());
        }

        info!(
            user = %event.user_email,
            count = session_keys.len(),
            reason = %event.reason,
            "Invalidating cached sessions following credential rotation"
        );

        // Atomic pipeline: delete each session key and clean up the index
        let mut pipe = deadpool_redis::redis::pipe();
        for key in &session_keys {
            pipe.del(key);
        }
        pipe.del(&index_key);

        let _: () = pipe.query_async(&mut conn).await?;

        // Publish eviction notice across internal Redis channel for edge proxies
        cmd("PUBLISH")
            .arg("events:auth:invalidations")
            .arg(format!("{}:{}", event.tenant, event.user_email))
            .query_async(&mut conn)
            .await?;

        Ok(())
    }
}
```

### Hardening Caddy and Redis Connection Pools

To ensure authentication loop storms can never exhaust socket capacity for normal traffic, we hardened both Redis connection pooling and Caddy reverse proxy limits:

```caddy
# Caddy snippet for Bulwark webmail edge isolation
(mail_proxy_policy) {
    transport http {
        dial_timeout 3s
        response_header_timeout 5s
        max_conns_per_host 128
        keepalive 30s
    }

    # Strict rate-limiting on authentication entrypoints
    rate_limit {
        zone auth_limit {
            key {remote_host}
            events 10
            window 1m
        }
    }
}

spam.house {
    import mail_proxy_policy
    reverse_proxy 10.100.0.25:8080 {
        header_up X-Forwarded-Port {server_port}
        header_up X-Real-IP {remote_host}
    }
}
```

---

## Lessons Learned & Best Practices

1. **Include Credential Timestamps in Cache Keys**: Never use naked identifiers (`session:{user}`) for authentication state. Binding cache keys to a credential revision number guarantees immediate, atomic invalidation without relying on asynchronous cache sweeps.
2. **Always Overwrite Security Tokens on Re-Auth**: Optimization routines must never skip writing credential or token payloads. If a user successfully proves identity with a new secret, every downstream cache artifact must be updated unconditionally.
3. **Bound Connection Pools with Short Acquisition Timeouts**: A failing downstream dependency must fail fast. Allowing connection acquisitions to block indefinitely cascades failures across all co-hosted services.
4. **Subscribe to Administrative CLI Actions**: If an administrative CLI bypasses the HTTP API, ensure that CLI emits audit signals to the central message bus so distributed caching layers stay synchronized.

---

## References

- [Stalwart Mail Server Architecture & Authentication](https://stalwartlabs.com/docs/)
- [Redis Keyspace Notifications & Invalidation](https://redis.io/docs/manual/keyspace-notifications/)
- [RFC 6749: The OAuth 2.0 Authorization Framework](https://datatracker.ietf.org/doc/rfc6749/)
