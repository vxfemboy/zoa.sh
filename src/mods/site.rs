//! Per-request effective domain. The site is served on zoa.sh, vx.gay, and
//! namecheap.wtf (a vanity URL for one post). Every self-referential URL, OG
//! tag, email, and `{domain}`/`{email}` token is rendered for the domain the
//! visitor arrived on. namecheap.wtf and any unknown host default to vx.gay.

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
    let host = host.split(':').next().unwrap_or(host).trim().to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    match host {
        "zoa.sh" => Site::new("zoa.sh", "zoa@zoa.sh"),
        // vx.gay, namecheap.wtf (defers), and anything unknown → vx.gay.
        _ => Site::new("vx.gay", "z@vx.gay"),
    }
}
