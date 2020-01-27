pub enum CacheabilityPolicy {
    AllowCaching,
    NoCache, //< User-agent must revalidate before using cached response
    NoStore,
}

impl Default for CacheabilityPolicy {
    fn default() -> Self {
        CacheabilityPolicy::AllowCaching
    }
}

pub struct Cacheability {
    pub private: bool,
    pub policy: CacheabilityPolicy,
}

impl Default for Cacheability {
    fn default() -> Self {
        Cacheability {
            private: true,
            policy: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Revalidation {
    pub must_revalidate: bool,
    pub proxy_revalidate: bool,
    pub immutable: bool,
}

#[derive(Default)]
pub struct CacheControl {
    pub cacheability: Cacheability,
    // expiration, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#Expiration
    pub revalidation: Revalidation,
    // other, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#Other
}

impl std::fmt::Display for CacheControl {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.cacheability.private {
            write!(fmt, "private")?;
        } else {
            write!(fmt, "public")?;
        }

        match self.cacheability.policy {
            CacheabilityPolicy::AllowCaching => (),
            CacheabilityPolicy::NoCache => write!(fmt, ", no-cache")?,
            CacheabilityPolicy::NoStore => write!(fmt, ", no-store")?,
        };

        if self.revalidation.must_revalidate {
            write!(fmt, ", must-revalidate")?;
        }

        if self.revalidation.proxy_revalidate {
            write!(fmt, ", proxy-revalidate")?;
        }

        if self.revalidation.immutable {
            write!(fmt, ", max-age=31536000, immutable")?;
        }

        Ok(())
    }
}
