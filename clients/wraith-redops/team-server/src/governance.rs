use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesOfEngagement {
    pub allowed_networks: Vec<String>, // CIDRs
    pub blocked_networks: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
}

impl RulesOfEngagement {
    pub fn is_ip_allowed(&self, ip: IpAddr) -> bool {
        // Check blocks first
        for net_str in &self.blocked_networks {
            if let Ok(net) = net_str.parse::<IpNetwork>()
                && net.contains(ip)
            {
                return false;
            }
        }

        // Check allows
        // If allowed list is empty, default to BLOCK all (whitelist mode)
        // Unless "0.0.0.0/0" is explicit.
        if self.allowed_networks.is_empty() {
            return false;
        }

        for net_str in &self.allowed_networks {
            if let Ok(net) = net_str.parse::<IpNetwork>()
                && net.contains(ip)
            {
                return true;
            }
        }

        false
    }

    pub fn is_time_valid(&self) -> bool {
        let now = chrono::Utc::now();
        if let Some(start) = self.start_date
            && now < start
        {
            return false;
        }
        if let Some(end) = self.end_date
            && now > end
        {
            return false;
        }
        true
    }

    pub fn is_domain_allowed(&self, domain: &str) -> bool {
        for blocked in &self.blocked_domains {
            if domain.ends_with(blocked) {
                // simplified suffix check
                return false;
            }
        }
        if self.allowed_domains.is_empty() {
            return true;
        }
        for allowed in &self.allowed_domains {
            if domain.ends_with(allowed) {
                return true;
            }
        }
        false
    }
}

// Global/Shared governance state
pub struct GovernanceEngine {
    active_roe: Option<RulesOfEngagement>,
}

impl GovernanceEngine {
    pub fn new() -> Self {
        // Default RoE: Allow localhost/private for dev
        Self {
            active_roe: Some(RulesOfEngagement {
                allowed_networks: vec![
                    "127.0.0.0/8".to_string(),
                    "10.0.0.0/8".to_string(),
                    "192.168.0.0/16".to_string(),
                ],
                blocked_networks: vec![],
                allowed_domains: vec![],
                blocked_domains: vec![],
                start_date: None,
                end_date: None,
            }),
        }
    }

    pub fn validate_action(&self, source_ip: IpAddr) -> bool {
        if let Some(roe) = &self.active_roe {
            if !roe.is_time_valid() {
                tracing::warn!("RoE Violation: Operation outside time window");
                return false;
            }
            if !roe.is_ip_allowed(source_ip) {
                tracing::warn!("RoE Violation: IP {} not in scope", source_ip);
                return false;
            }
        }
        true
    }

    pub fn validate_domain(&self, domain: &str) -> bool {
        if let Some(roe) = &self.active_roe
            && !roe.is_domain_allowed(domain)
        {
            tracing::warn!("RoE Violation: Domain {} not in scope", domain);
            return false;
        }
        true
    }
}
