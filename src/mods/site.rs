//! Per-request effective domain. The site is served on zoa.sh, vx.gay, and
//! namecheap.wtf (a vanity URL for one post). Every self-referential URL, OG
//! tag, email, and `{domain}`/`{email}` token is rendered for the domain the
//! visitor arrived on. namecheap.wtf and any unknown host default to vx.gay.

#![allow(dead_code)]

/// The resolved identity for one request.
pub struct Site {
    pub domain: String,
    pub base_url: String,
    pub email: String,
}

impl Site {
    fn new(domain: &str, email: &str) -> Self {
        Self {
            domain: domain.to_string(),
            base_url: format!("https://{domain}"),
            email: email.to_string(),
        }
    }

    /// Local development: mirror the exact host+port the visitor used over plain
    /// `http`, so self-referential links stay on localhost instead of bouncing to
    /// the production `https://vx.gay`. `host` is the full `Host` header (with port).
    fn dev(host: &str) -> Self {
        Self {
            domain: host.to_string(),
            base_url: format!("http://{host}"),
            email: "zoa@zoa.sh".to_string(),
        }
    }

    /// Substitute the opt-in tokens. Only `{domain}` and `{email}` swap; every
    /// other character (including literal domain names) is left untouched.
    pub fn apply(&self, s: &str) -> String {
        s.replace("{domain}", &self.domain)
            .replace("{email}", &self.email)
    }

    /// Absolute OG/social image URL. `social` is a path under `posts/assets/`
    /// (frontmatter `social:`); absent → the default site image.
    pub fn og_image(&self, social: Option<&str>) -> String {
        match social {
            Some(path) => format!(
                "{}/post/assets/{}",
                self.base_url,
                path.trim_start_matches('/')
            ),
            None => format!("{}/static/og-image.gif", self.base_url),
        }
    }
}

/// Resolve the effective site from a request `Host` header value.
pub fn resolve(host: &str) -> Site {
    let full = host.trim();
    let bare = full.split(':').next().unwrap_or(full).trim().to_lowercase();
    let bare = bare.strip_prefix("www.").unwrap_or(&bare);
    match bare {
        "zoa.sh" => Site::new("zoa.sh", "zoa@zoa.sh"),
        // Local dev: keep the visitor on the host+port they actually used.
        "localhost" | "127.0.0.1" => Site::dev(full),
        // vx.gay, namecheap.wtf (defers), and anything else unknown → vx.gay.
        _ => Site::new("vx.gay", "z@vx.gay"),
    }
}
