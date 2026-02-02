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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn make_roe(
        allowed: Vec<&str>,
        blocked: Vec<&str>,
        allowed_domains: Vec<&str>,
        blocked_domains: Vec<&str>,
    ) -> RulesOfEngagement {
        RulesOfEngagement {
            allowed_networks: allowed.into_iter().map(String::from).collect(),
            blocked_networks: blocked.into_iter().map(String::from).collect(),
            allowed_domains: allowed_domains.into_iter().map(String::from).collect(),
            blocked_domains: blocked_domains.into_iter().map(String::from).collect(),
            start_date: None,
            end_date: None,
        }
    }

    // --- RulesOfEngagement IP tests ---

    #[test]
    fn test_ip_allowed_in_cidr() {
        let roe = make_roe(vec!["10.0.0.0/8"], vec![], vec![], vec![]);
        assert!(roe.is_ip_allowed(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
    }

    #[test]
    fn test_ip_not_in_allowed_cidr() {
        let roe = make_roe(vec!["10.0.0.0/8"], vec![], vec![], vec![]);
        assert!(!roe.is_ip_allowed(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }

    #[test]
    fn test_ip_in_blocked_takes_priority() {
        let roe = make_roe(vec!["10.0.0.0/8"], vec!["10.0.0.0/24"], vec![], vec![]);
        // 10.0.0.5 is in allowed /8 but also in blocked /24
        assert!(!roe.is_ip_allowed(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))));
        // 10.1.0.5 is in allowed /8 but not in blocked /24
        assert!(roe.is_ip_allowed(IpAddr::V4(Ipv4Addr::new(10, 1, 0, 5))));
    }

    #[test]
    fn test_empty_allowed_blocks_all() {
        let roe = make_roe(vec![], vec![], vec![], vec![]);
        assert!(!roe.is_ip_allowed(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn test_ipv6_allowed() {
        let roe = make_roe(vec!["::1/128"], vec![], vec![], vec![]);
        assert!(roe.is_ip_allowed(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn test_ipv6_not_allowed() {
        let roe = make_roe(vec!["10.0.0.0/8"], vec![], vec![], vec![]);
        assert!(!roe.is_ip_allowed(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    // --- RulesOfEngagement time tests ---

    #[test]
    fn test_time_valid_no_bounds() {
        let roe = make_roe(vec![], vec![], vec![], vec![]);
        assert!(roe.is_time_valid());
    }

    #[test]
    fn test_time_valid_within_window() {
        let mut roe = make_roe(vec![], vec![], vec![], vec![]);
        roe.start_date = Some(chrono::Utc::now() - chrono::Duration::hours(1));
        roe.end_date = Some(chrono::Utc::now() + chrono::Duration::hours(1));
        assert!(roe.is_time_valid());
    }

    #[test]
    fn test_time_invalid_before_start() {
        let mut roe = make_roe(vec![], vec![], vec![], vec![]);
        roe.start_date = Some(chrono::Utc::now() + chrono::Duration::hours(1));
        assert!(!roe.is_time_valid());
    }

    #[test]
    fn test_time_invalid_after_end() {
        let mut roe = make_roe(vec![], vec![], vec![], vec![]);
        roe.end_date = Some(chrono::Utc::now() - chrono::Duration::hours(1));
        assert!(!roe.is_time_valid());
    }

    // --- RulesOfEngagement domain tests ---

    #[test]
    fn test_domain_allowed_empty_list() {
        let roe = make_roe(vec![], vec![], vec![], vec![]);
        assert!(roe.is_domain_allowed("anything.com"));
    }

    #[test]
    fn test_domain_allowed_match() {
        let roe = make_roe(vec![], vec![], vec!["example.com"], vec![]);
        assert!(roe.is_domain_allowed("test.example.com"));
        assert!(!roe.is_domain_allowed("test.other.com"));
    }

    #[test]
    fn test_domain_blocked() {
        let roe = make_roe(vec![], vec![], vec![], vec!["evil.com"]);
        assert!(!roe.is_domain_allowed("sub.evil.com"));
        assert!(roe.is_domain_allowed("good.com"));
    }

    #[test]
    fn test_domain_blocked_takes_priority() {
        let roe = make_roe(vec![], vec![], vec!["example.com"], vec!["bad.example.com"]);
        assert!(!roe.is_domain_allowed("test.bad.example.com"));
        assert!(roe.is_domain_allowed("good.example.com"));
    }

    // --- RulesOfEngagement serialization ---

    #[test]
    fn test_roe_serialization() {
        let roe = make_roe(vec!["10.0.0.0/8"], vec![], vec!["example.com"], vec![]);
        let json = serde_json::to_string(&roe).unwrap();
        let deserialized: RulesOfEngagement = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.allowed_networks, roe.allowed_networks);
        assert_eq!(deserialized.allowed_domains, roe.allowed_domains);
    }

    // --- GovernanceEngine tests ---

    #[test]
    fn test_governance_engine_new_defaults() {
        let engine = GovernanceEngine::new();
        // Default allows localhost
        assert!(engine.validate_action(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        // Default allows 10.x
        assert!(engine.validate_action(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        // Default allows 192.168.x
        assert!(engine.validate_action(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        // Default blocks public IPs
        assert!(!engine.validate_action(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn test_governance_validate_domain_default() {
        let engine = GovernanceEngine::new();
        // Default has empty domain lists, so all domains are allowed
        assert!(engine.validate_domain("example.com"));
    }

    #[test]
    fn test_governance_no_roe() {
        let engine = GovernanceEngine { active_roe: None };
        // No RoE means allow everything
        assert!(engine.validate_action(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(engine.validate_domain("anything.com"));
    }
}
