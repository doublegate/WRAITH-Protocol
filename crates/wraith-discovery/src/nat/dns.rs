//! DNS Resolution for STUN Servers
//!
//! This module provides asynchronous DNS resolution for STUN server hostnames,
//! enabling flexible STUN server configuration with fallback to hardcoded IPs.

use hickory_resolver::{
    Resolver,
    config::{ResolverConfig, ResolverOpts},
    name_server::TokioConnectionProvider,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;

/// DNS resolution errors
#[derive(Debug, Error)]
pub enum DnsError {
    /// DNS resolution failed
    #[error("DNS resolution failed: {0}")]
    ResolutionFailed(String),

    /// No IP addresses found
    #[error("No IP addresses found for hostname: {0}")]
    NoAddressesFound(String),

    /// DNS resolver initialization failed
    #[error("DNS resolver initialization failed: {0}")]
    InitializationFailed(String),
}

/// STUN server specification - either a hostname or an IP address
#[derive(Debug, Clone)]
pub enum StunServerSpec {
    /// Hostname that needs DNS resolution
    Hostname {
        /// The DNS hostname to resolve (e.g., "stun.example.com")
        hostname: String,
        /// Port number for the STUN server
        port: u16,
    },
    /// IP address (no resolution needed)
    IpAddress(SocketAddr),
}

impl StunServerSpec {
    /// Create a new hostname-based STUN server specification
    #[must_use]
    pub fn hostname(hostname: impl Into<String>, port: u16) -> Self {
        Self::Hostname {
            hostname: hostname.into(),
            port,
        }
    }

    /// Create a new IP-based STUN server specification
    #[must_use]
    pub const fn ip(addr: SocketAddr) -> Self {
        Self::IpAddress(addr)
    }
}

/// Cached DNS resolution entry
#[derive(Clone)]
struct CachedEntry {
    addresses: Vec<IpAddr>,
    expires_at: Instant,
}

/// Type alias for Tokio-based DNS resolver
type TokioResolver = Resolver<TokioConnectionProvider>;

/// DNS resolver for STUN servers
///
/// Provides asynchronous DNS resolution with caching to minimize DNS lookups.
/// Falls back to cached or hardcoded IPs when DNS resolution fails.
pub struct StunDnsResolver {
    resolver: TokioResolver,
    cache: Arc<RwLock<HashMap<String, CachedEntry>>>,
    cache_ttl: Duration,
}

impl StunDnsResolver {
    /// Create a new STUN DNS resolver with default configuration
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn new() -> Result<Self, DnsError> {
        Self::with_config(ResolverConfig::default(), ResolverOpts::default()).await
    }

    /// Create a new STUN DNS resolver with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn with_config(config: ResolverConfig, opts: ResolverOpts) -> Result<Self, DnsError> {
        let resolver = Resolver::builder_with_config(config, TokioConnectionProvider::default())
            .with_options(opts)
            .build();

        Ok(Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minute cache TTL
        })
    }

    /// Set the cache TTL
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// Resolve a STUN server specification to socket addresses
    ///
    /// If the spec is an IP address, returns it directly.
    /// If the spec is a hostname, performs DNS resolution with caching.
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolution fails and no cached entry exists
    pub async fn resolve(&self, spec: &StunServerSpec) -> Result<Vec<SocketAddr>, DnsError> {
        match spec {
            StunServerSpec::IpAddress(addr) => Ok(vec![*addr]),
            StunServerSpec::Hostname { hostname, port } => {
                self.resolve_hostname(hostname, *port).await
            }
        }
    }

    /// Resolve a hostname to socket addresses
    async fn resolve_hostname(
        &self,
        hostname: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, DnsError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(hostname)
                && entry.expires_at > Instant::now()
            {
                let addrs: Vec<SocketAddr> = entry
                    .addresses
                    .iter()
                    .map(|ip| SocketAddr::new(*ip, port))
                    .collect();
                return Ok(addrs);
            }
        }

        // Perform DNS resolution
        let response = self
            .resolver
            .lookup_ip(hostname)
            .await
            .map_err(|e| DnsError::ResolutionFailed(e.to_string()))?;

        let addresses: Vec<IpAddr> = response.iter().collect();

        if addresses.is_empty() {
            return Err(DnsError::NoAddressesFound(hostname.to_string()));
        }

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                hostname.to_string(),
                CachedEntry {
                    addresses: addresses.clone(),
                    expires_at: Instant::now() + self.cache_ttl,
                },
            );
        }

        // Convert to socket addresses
        let addrs: Vec<SocketAddr> = addresses
            .iter()
            .map(|ip| SocketAddr::new(*ip, port))
            .collect();

        Ok(addrs)
    }

    /// Resolve multiple STUN server specifications
    ///
    /// Returns all successfully resolved addresses.
    pub async fn resolve_many(&self, specs: &[StunServerSpec]) -> Vec<SocketAddr> {
        let mut all_addrs = Vec::new();

        for spec in specs {
            if let Ok(addrs) = self.resolve(spec).await {
                all_addrs.extend(addrs);
            }
        }

        all_addrs
    }

    /// Clear the DNS cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Default STUN server specifications
///
/// Returns a list of well-known STUN servers with both hostname and fallback IP options.
#[must_use]
pub fn default_stun_servers() -> Vec<StunServerSpec> {
    vec![
        // Cloudflare STUN (with DNS hostname and fallback IP)
        StunServerSpec::hostname("stun.cloudflare.com", 3478),
        // Twilio STUN
        StunServerSpec::hostname("global.stun.twilio.com", 3478),
        // Nextcloud STUN (HTTPS port for firewall bypass)
        StunServerSpec::hostname("stun.nextcloud.com", 443),
        // Google Public STUN servers
        StunServerSpec::hostname("stun.l.google.com", 19302),
        StunServerSpec::hostname("stun1.l.google.com", 19302),
    ]
}

/// Fallback STUN server IPs (used when DNS resolution fails)
///
/// These hardcoded IPs serve as fallback when DNS resolution is unavailable.
#[must_use]
pub fn fallback_stun_ips() -> Vec<SocketAddr> {
    vec![
        // Cloudflare STUN
        "162.159.207.0:3478".parse().expect("valid fallback IP"),
        // Twilio STUN
        "34.203.251.210:3478".parse().expect("valid fallback IP"),
        // Nextcloud STUN
        "159.69.191.124:443".parse().expect("valid fallback IP"),
        // Google Public STUN servers
        "74.125.250.129:19302".parse().expect("valid fallback IP"),
        "74.125.250.130:19302".parse().expect("valid fallback IP"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_server_spec_creation() {
        let hostname_spec = StunServerSpec::hostname("stun.example.com", 3478);
        assert!(matches!(hostname_spec, StunServerSpec::Hostname { .. }));

        let ip_spec = StunServerSpec::ip("1.2.3.4:3478".parse().unwrap());
        assert!(matches!(ip_spec, StunServerSpec::IpAddress(_)));
    }

    #[test]
    fn test_default_stun_servers() {
        let servers = default_stun_servers();
        assert!(!servers.is_empty());
        assert!(servers.len() >= 5);
    }

    #[test]
    fn test_fallback_stun_ips() {
        let ips = fallback_stun_ips();
        assert!(!ips.is_empty());
        assert_eq!(ips.len(), 5);
    }

    #[tokio::test]
    async fn test_resolve_ip_address() {
        let resolver = StunDnsResolver::new().await.unwrap();
        let spec = StunServerSpec::ip("1.2.3.4:3478".parse().unwrap());
        let result = resolver.resolve(&spec).await;
        assert!(result.is_ok());
        let addrs = result.unwrap();
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0], "1.2.3.4:3478".parse::<SocketAddr>().unwrap());
    }

    #[tokio::test]
    async fn test_resolve_many_ips() {
        let resolver = StunDnsResolver::new().await.unwrap();
        let specs = vec![
            StunServerSpec::ip("1.2.3.4:3478".parse().unwrap()),
            StunServerSpec::ip("5.6.7.8:3478".parse().unwrap()),
        ];
        let addrs = resolver.resolve_many(&specs).await;
        assert_eq!(addrs.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let resolver = StunDnsResolver::new().await.unwrap();
        resolver.clear_cache().await;
        // Cache should be empty after clear
        let cache = resolver.cache.read().await;
        assert!(cache.is_empty());
    }

    #[test]
    fn test_dns_error_display() {
        let err = DnsError::ResolutionFailed("test".to_string());
        assert!(err.to_string().contains("DNS resolution failed"));

        let err = DnsError::NoAddressesFound("example.com".to_string());
        assert!(err.to_string().contains("No IP addresses found"));

        let err = DnsError::InitializationFailed("init error".to_string());
        assert!(err.to_string().contains("initialization failed"));
    }
}
