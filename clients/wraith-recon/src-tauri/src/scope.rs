//! Target Scope Enforcement Module
//!
//! This module provides target whitelist enforcement using CIDR ranges and domain patterns.
//! ALL reconnaissance and exfiltration operations MUST validate targets against the scope
//! before proceeding.
//!
//! ## Security Requirements
//! - Target validation is MANDATORY before any network operation
//! - Excluded targets override authorized targets
//! - Scope changes require audit logging

use crate::error::{ReconError, Result};
use crate::roe::RulesOfEngagement;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

/// Target type for scope validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TargetType {
    /// IPv4 or IPv6 address
    IpAddress(String),
    /// IPv4 or IPv6 CIDR range
    CidrRange(String),
    /// Domain name
    Domain(String),
    /// Hostname
    Hostname(String),
    /// URL
    Url(String),
    /// Port specification (IP:port)
    PortSpec(String),
}

/// Target specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Unique target identifier
    pub id: String,
    /// Target type
    pub target_type: TargetType,
    /// Target value
    pub value: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this target is excluded (off-limits)
    pub excluded: bool,
    /// Creation timestamp
    pub created_at: i64,
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let id = uuid::Uuid::new_v4().to_string();

        // Try to parse as IP address
        if let Ok(ip) = s.parse::<IpAddr>() {
            return Ok(Self::new(
                id,
                TargetType::IpAddress(ip.to_string()),
                s.to_string(),
            ));
        }

        // Try to parse as CIDR
        if s.contains('/') && IpNetwork::from_str(s).is_ok() {
            return Ok(Self::new(
                id,
                TargetType::CidrRange(s.to_string()),
                s.to_string(),
            ));
        }

        // Try to parse as URL
        if s.starts_with("http://") || s.starts_with("https://") {
            return Ok(Self::new(id, TargetType::Url(s.to_string()), s.to_string()));
        }

        // Try to parse as port spec (IP:port)
        if let Some((ip_part, port_part)) = s.rsplit_once(':')
            && ip_part.parse::<IpAddr>().is_ok()
            && port_part.parse::<u16>().is_ok()
        {
            return Ok(Self::new(
                id,
                TargetType::PortSpec(s.to_string()),
                s.to_string(),
            ));
        }

        // Default to domain
        Ok(Self::new(
            id,
            TargetType::Domain(s.to_string()),
            s.to_string(),
        ))
    }
}

impl Target {
    /// Create a new target
    pub fn new(id: String, target_type: TargetType, value: String) -> Self {
        Self {
            id,
            target_type,
            value,
            description: None,
            excluded: false,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create an IP address target
    pub fn ip(id: String, ip: &str) -> Result<Self> {
        // Validate IP address
        ip.parse::<IpAddr>()
            .map_err(|e| ReconError::InvalidTarget(format!("Invalid IP address: {}", e)))?;
        Ok(Self::new(
            id,
            TargetType::IpAddress(ip.to_string()),
            ip.to_string(),
        ))
    }

    /// Create a CIDR range target
    pub fn cidr(id: String, cidr: &str) -> Result<Self> {
        // Validate CIDR
        IpNetwork::from_str(cidr)?;
        Ok(Self::new(
            id,
            TargetType::CidrRange(cidr.to_string()),
            cidr.to_string(),
        ))
    }

    /// Create a domain target
    pub fn domain(id: String, domain: &str) -> Self {
        Self::new(
            id,
            TargetType::Domain(domain.to_string()),
            domain.to_string(),
        )
    }
}

/// Scope validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeValidationResult {
    /// Whether the target is within scope
    pub in_scope: bool,
    /// Reason for the result
    pub reason: String,
    /// Whether this is an excluded target
    pub is_excluded: bool,
    /// Matching rule (if any)
    pub matching_rule: Option<String>,
}

/// Scope manager for target validation
pub struct ScopeManager {
    /// Authorized CIDR ranges
    authorized_cidrs: Vec<IpNetwork>,
    /// Authorized domains
    authorized_domains: HashSet<String>,
    /// Excluded CIDR ranges (always off-limits)
    excluded_cidrs: Vec<IpNetwork>,
    /// Excluded domains (always off-limits)
    excluded_domains: HashSet<String>,
    /// Custom targets added during engagement
    custom_targets: Vec<Target>,
}

impl ScopeManager {
    /// Create a new scope manager
    pub fn new() -> Self {
        Self {
            authorized_cidrs: Vec::new(),
            authorized_domains: HashSet::new(),
            excluded_cidrs: Vec::new(),
            excluded_domains: HashSet::new(),
            custom_targets: Vec::new(),
        }
    }

    /// Initialize from Rules of Engagement
    pub fn from_roe(roe: &RulesOfEngagement) -> Result<Self> {
        let mut manager = Self::new();

        // Parse authorized CIDRs
        for cidr_str in &roe.authorized_cidrs {
            let cidr = IpNetwork::from_str(cidr_str)?;
            manager.authorized_cidrs.push(cidr);
        }

        // Add authorized domains
        for domain in &roe.authorized_domains {
            manager.authorized_domains.insert(domain.to_lowercase());
        }

        // Parse excluded targets
        for excluded in &roe.excluded_targets {
            // Try parsing as CIDR first
            if let Ok(cidr) = IpNetwork::from_str(excluded) {
                manager.excluded_cidrs.push(cidr);
            } else {
                // Treat as domain
                manager.excluded_domains.insert(excluded.to_lowercase());
            }
        }

        Ok(manager)
    }

    /// Add an authorized CIDR range
    pub fn add_cidr(&mut self, cidr: &str) -> Result<()> {
        let network = IpNetwork::from_str(cidr)?;
        if !self.authorized_cidrs.contains(&network) {
            self.authorized_cidrs.push(network);
        }
        Ok(())
    }

    /// Add an authorized domain
    pub fn add_domain(&mut self, domain: &str) {
        self.authorized_domains.insert(domain.to_lowercase());
    }

    /// Add an excluded CIDR range
    pub fn add_excluded_cidr(&mut self, cidr: &str) -> Result<()> {
        let network = IpNetwork::from_str(cidr)?;
        if !self.excluded_cidrs.contains(&network) {
            self.excluded_cidrs.push(network);
        }
        Ok(())
    }

    /// Add an excluded domain
    pub fn add_excluded_domain(&mut self, domain: &str) {
        self.excluded_domains.insert(domain.to_lowercase());
    }

    /// Add a custom target
    pub fn add_target(&mut self, target: Target) {
        self.custom_targets.push(target);
    }

    /// Remove a custom target
    pub fn remove_target(&mut self, target_id: &str) -> Option<Target> {
        if let Some(pos) = self.custom_targets.iter().position(|t| t.id == target_id) {
            Some(self.custom_targets.remove(pos))
        } else {
            None
        }
    }

    /// Validate an IP address against the scope
    pub fn validate_ip(&self, ip: &str) -> ScopeValidationResult {
        let ip_addr = match ip.parse::<IpAddr>() {
            Ok(addr) => addr,
            Err(_) => {
                return ScopeValidationResult {
                    in_scope: false,
                    reason: "Invalid IP address format".to_string(),
                    is_excluded: false,
                    matching_rule: None,
                };
            }
        };

        // Check excluded CIDRs first (they take priority)
        for excluded in &self.excluded_cidrs {
            if excluded.contains(ip_addr) {
                return ScopeValidationResult {
                    in_scope: false,
                    reason: "IP is in excluded CIDR range".to_string(),
                    is_excluded: true,
                    matching_rule: Some(excluded.to_string()),
                };
            }
        }

        // Check authorized CIDRs
        for authorized in &self.authorized_cidrs {
            if authorized.contains(ip_addr) {
                return ScopeValidationResult {
                    in_scope: true,
                    reason: "IP is in authorized CIDR range".to_string(),
                    is_excluded: false,
                    matching_rule: Some(authorized.to_string()),
                };
            }
        }

        // Check custom targets
        for target in &self.custom_targets {
            if target.excluded {
                continue;
            }
            match &target.target_type {
                TargetType::IpAddress(target_ip) if target_ip == ip => {
                    return ScopeValidationResult {
                        in_scope: true,
                        reason: "IP matches custom target".to_string(),
                        is_excluded: false,
                        matching_rule: Some(target.id.clone()),
                    };
                }
                TargetType::CidrRange(cidr) => {
                    if let Ok(network) = IpNetwork::from_str(cidr)
                        && network.contains(ip_addr)
                    {
                        return ScopeValidationResult {
                            in_scope: true,
                            reason: "IP is in custom CIDR range".to_string(),
                            is_excluded: false,
                            matching_rule: Some(target.id.clone()),
                        };
                    }
                }
                _ => {}
            }
        }

        ScopeValidationResult {
            in_scope: false,
            reason: "IP is not in any authorized scope".to_string(),
            is_excluded: false,
            matching_rule: None,
        }
    }

    /// Validate a domain against the scope
    pub fn validate_domain(&self, domain: &str) -> ScopeValidationResult {
        let domain_lower = domain.to_lowercase();

        // Check excluded domains first
        for excluded in &self.excluded_domains {
            if domain_lower == *excluded || domain_lower.ends_with(&format!(".{}", excluded)) {
                return ScopeValidationResult {
                    in_scope: false,
                    reason: "Domain is excluded".to_string(),
                    is_excluded: true,
                    matching_rule: Some(excluded.clone()),
                };
            }
        }

        // Check authorized domains
        for authorized in &self.authorized_domains {
            if domain_lower == *authorized || domain_lower.ends_with(&format!(".{}", authorized)) {
                return ScopeValidationResult {
                    in_scope: true,
                    reason: "Domain is authorized".to_string(),
                    is_excluded: false,
                    matching_rule: Some(authorized.clone()),
                };
            }
        }

        // Check custom targets
        for target in &self.custom_targets {
            if target.excluded {
                continue;
            }
            if let TargetType::Domain(target_domain) = &target.target_type {
                let target_lower = target_domain.to_lowercase();
                if domain_lower == target_lower
                    || domain_lower.ends_with(&format!(".{}", target_lower))
                {
                    return ScopeValidationResult {
                        in_scope: true,
                        reason: "Domain matches custom target".to_string(),
                        is_excluded: false,
                        matching_rule: Some(target.id.clone()),
                    };
                }
            }
        }

        ScopeValidationResult {
            in_scope: false,
            reason: "Domain is not in any authorized scope".to_string(),
            is_excluded: false,
            matching_rule: None,
        }
    }

    /// Validate a target (auto-detect type from string)
    pub fn validate_str(&self, target: &str) -> ScopeValidationResult {
        // Try to parse as IP address
        if target.parse::<IpAddr>().is_ok() {
            return self.validate_ip(target);
        }

        // Try to parse as CIDR
        if target.contains('/')
            && let Ok(network) = IpNetwork::from_str(target)
        {
            // For CIDR validation, check if the network is within scope
            let ip = network.network();
            return self.validate_ip(&ip.to_string());
        }

        // Treat as domain
        self.validate_domain(target)
    }

    /// Validate a Target struct
    pub fn validate(&self, target: &Target) -> Result<()> {
        let result = match &target.target_type {
            TargetType::IpAddress(ip) => self.validate_ip(ip),
            TargetType::CidrRange(cidr) => {
                if let Ok(network) = IpNetwork::from_str(cidr) {
                    self.validate_ip(&network.network().to_string())
                } else {
                    ScopeValidationResult {
                        in_scope: false,
                        reason: "Invalid CIDR format".to_string(),
                        is_excluded: false,
                        matching_rule: None,
                    }
                }
            }
            TargetType::Domain(domain) => self.validate_domain(domain),
            TargetType::Hostname(hostname) => self.validate_domain(hostname),
            TargetType::Url(url) => {
                // Extract host from URL
                if let Ok(parsed) = url::Url::parse(url) {
                    if let Some(host) = parsed.host_str() {
                        self.validate_str(host)
                    } else {
                        ScopeValidationResult {
                            in_scope: false,
                            reason: "URL has no host".to_string(),
                            is_excluded: false,
                            matching_rule: None,
                        }
                    }
                } else {
                    ScopeValidationResult {
                        in_scope: false,
                        reason: "Invalid URL format".to_string(),
                        is_excluded: false,
                        matching_rule: None,
                    }
                }
            }
            TargetType::PortSpec(spec) => {
                // Extract IP from IP:port
                if let Some((ip, _)) = spec.rsplit_once(':') {
                    self.validate_ip(ip)
                } else {
                    ScopeValidationResult {
                        in_scope: false,
                        reason: "Invalid port spec format".to_string(),
                        is_excluded: false,
                        matching_rule: None,
                    }
                }
            }
        };

        if result.in_scope {
            Ok(())
        } else {
            Err(ReconError::TargetOutOfScope {
                target: target.value.clone(),
            })
        }
    }

    /// Add a custom target to the scope (convenience method)
    pub fn add_custom_target(&mut self, target: Target) {
        self.add_target(target);
    }

    /// Get all authorized CIDRs
    pub fn authorized_cidrs(&self) -> Vec<String> {
        self.authorized_cidrs
            .iter()
            .map(|c| c.to_string())
            .collect()
    }

    /// Get all authorized domains
    pub fn authorized_domains(&self) -> Vec<String> {
        self.authorized_domains.iter().cloned().collect()
    }

    /// Get all custom targets
    pub fn custom_targets(&self) -> &[Target] {
        &self.custom_targets
    }

    /// Get scope summary
    pub fn summary(&self) -> ScopeSummary {
        ScopeSummary {
            authorized_cidr_count: self.authorized_cidrs.len(),
            authorized_domain_count: self.authorized_domains.len(),
            excluded_cidr_count: self.excluded_cidrs.len(),
            excluded_domain_count: self.excluded_domains.len(),
            custom_target_count: self.custom_targets.len(),
        }
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of scope configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeSummary {
    pub authorized_cidr_count: usize,
    pub authorized_domain_count: usize,
    pub excluded_cidr_count: usize,
    pub excluded_domain_count: usize,
    pub custom_target_count: usize,
}

/// CRITICAL: Scope enforcement guard
/// This struct MUST be used to wrap any network operation
pub struct ScopeGuard<'a> {
    scope: &'a ScopeManager,
}

impl<'a> ScopeGuard<'a> {
    pub fn new(scope: &'a ScopeManager) -> Self {
        Self { scope }
    }

    /// Validate a target before operation
    /// Returns Ok(()) if in scope, Err if out of scope
    pub fn check(&self, target: &str) -> Result<()> {
        let result = self.scope.validate_str(target);
        if result.in_scope {
            Ok(())
        } else {
            Err(ReconError::TargetOutOfScope {
                target: target.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_manager_creation() {
        let manager = ScopeManager::new();
        assert!(manager.authorized_cidrs.is_empty());
        assert!(manager.authorized_domains.is_empty());
    }

    #[test]
    fn test_add_cidr() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.1.0/24").unwrap();
        assert_eq!(manager.authorized_cidrs.len(), 1);
    }

    #[test]
    fn test_add_domain() {
        let mut manager = ScopeManager::new();
        manager.add_domain("example.com");
        assert!(manager.authorized_domains.contains("example.com"));
    }

    #[test]
    fn test_validate_ip_in_scope() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.1.0/24").unwrap();

        let result = manager.validate_ip("192.168.1.100");
        assert!(result.in_scope);
        assert!(!result.is_excluded);
    }

    #[test]
    fn test_validate_ip_out_of_scope() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.1.0/24").unwrap();

        let result = manager.validate_ip("10.0.0.1");
        assert!(!result.in_scope);
    }

    #[test]
    fn test_excluded_ip_takes_priority() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.0.0/16").unwrap();
        manager.add_excluded_cidr("192.168.1.0/24").unwrap();

        let result = manager.validate_ip("192.168.1.100");
        assert!(!result.in_scope);
        assert!(result.is_excluded);
    }

    #[test]
    fn test_validate_domain_in_scope() {
        let mut manager = ScopeManager::new();
        manager.add_domain("example.com");

        let result = manager.validate_domain("example.com");
        assert!(result.in_scope);

        let result = manager.validate_domain("sub.example.com");
        assert!(result.in_scope);
    }

    #[test]
    fn test_validate_domain_out_of_scope() {
        let mut manager = ScopeManager::new();
        manager.add_domain("example.com");

        let result = manager.validate_domain("other.com");
        assert!(!result.in_scope);
    }

    #[test]
    fn test_excluded_domain() {
        let mut manager = ScopeManager::new();
        manager.add_domain("example.com");
        manager.add_excluded_domain("sensitive.example.com");

        let result = manager.validate_domain("sensitive.example.com");
        assert!(!result.in_scope);
        assert!(result.is_excluded);

        // But other subdomains are still in scope
        let result = manager.validate_domain("other.example.com");
        assert!(result.in_scope);
    }

    #[test]
    fn test_scope_guard() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.1.0/24").unwrap();

        let guard = ScopeGuard::new(&manager);
        assert!(guard.check("192.168.1.100").is_ok());
        assert!(guard.check("10.0.0.1").is_err());
    }

    #[test]
    fn test_target_creation() {
        let target = Target::ip("test-1".to_string(), "192.168.1.1").unwrap();
        assert_eq!(target.value, "192.168.1.1");

        let target = Target::cidr("test-2".to_string(), "10.0.0.0/8").unwrap();
        assert_eq!(target.value, "10.0.0.0/8");

        let target = Target::domain("test-3".to_string(), "example.com");
        assert_eq!(target.value, "example.com");
    }

    #[test]
    fn test_custom_targets() {
        let mut manager = ScopeManager::new();
        let target = Target::ip("custom-1".to_string(), "10.10.10.10").unwrap();
        manager.add_target(target);

        let result = manager.validate_ip("10.10.10.10");
        assert!(result.in_scope);
        assert_eq!(result.matching_rule, Some("custom-1".to_string()));
    }

    #[test]
    fn test_auto_detect_validation() {
        let mut manager = ScopeManager::new();
        manager.add_cidr("192.168.1.0/24").unwrap();
        manager.add_domain("example.com");

        // IP detection
        let result = manager.validate_str("192.168.1.1");
        assert!(result.in_scope);

        // Domain detection
        let result = manager.validate_str("www.example.com");
        assert!(result.in_scope);
    }
}
