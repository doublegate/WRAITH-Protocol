//! DNS-over-HTTPS tunneling for traffic obfuscation.
//!
//! Encodes WRAITH traffic as DNS queries and responses,
//! allowing it to blend with legitimate DoH traffic.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

/// DNS-over-HTTPS tunnel
///
/// Encodes payloads as fake DNS queries/responses for stealth.
///
/// # Examples
///
/// ```
/// use wraith_obfuscation::doh_tunnel::DohTunnel;
///
/// let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
/// let query_url = tunnel.encode_query(b"secret data");
/// assert!(query_url.contains("dns="));
/// ```
pub struct DohTunnel {
    resolver_url: String,
}

impl DohTunnel {
    /// Create a new DoH tunnel
    ///
    /// # Arguments
    ///
    /// * `resolver_url` - The DoH resolver endpoint URL
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::doh_tunnel::DohTunnel;
    ///
    /// let tunnel = DohTunnel::new("https://1.1.1.1/dns-query".to_string());
    /// ```
    #[must_use]
    pub fn new(resolver_url: String) -> Self {
        Self { resolver_url }
    }

    /// Encode payload as fake DNS query URL
    ///
    /// Encodes the payload using base64url and formats it as a DoH query.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::doh_tunnel::DohTunnel;
    ///
    /// let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
    /// let url = tunnel.encode_query(b"test");
    /// assert!(url.starts_with("https://dns.example.com"));
    /// ```
    #[must_use]
    pub fn encode_query(&self, payload: &[u8]) -> String {
        // Encode payload as base64url (DNS wireformat simulation)
        let encoded = URL_SAFE_NO_PAD.encode(payload);

        // Format as DNS query parameter
        format!("{}?dns={}", self.resolver_url, encoded)
    }

    /// Decode DNS response to get payload
    ///
    /// Extracts the original payload from a base64url-encoded response.
    ///
    /// # Errors
    ///
    /// Returns `DohError::DecodeFailed` if the response cannot be decoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::doh_tunnel::DohTunnel;
    ///
    /// let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
    /// let encoded = b"dGVzdA"; // "test" in base64url
    /// let decoded = tunnel.decode_response(encoded).unwrap();
    /// assert_eq!(decoded, b"test");
    /// ```
    pub fn decode_response(&self, response: &[u8]) -> Result<Vec<u8>, DohError> {
        URL_SAFE_NO_PAD
            .decode(response)
            .map_err(|_| DohError::DecodeFailed)
    }

    /// Create fake DNS query packet
    ///
    /// Generates a DNS query packet with EDNS0 OPT record carrying the payload.
    ///
    /// # Errors
    ///
    /// Returns `DohError::InvalidLabel` if a DNS label is empty or exceeds 63 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::doh_tunnel::DohTunnel;
    ///
    /// let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
    /// let query = tunnel.create_dns_query("wraith.example.com", b"payload").unwrap();
    /// assert!(query.len() > 12); // Has DNS header
    /// ```
    pub fn create_dns_query(&self, domain: &str, payload: &[u8]) -> Result<Vec<u8>, DohError> {
        // Pre-allocate: 12 (header) + domain.len() + 2 (label overhead estimate) + 5 (null+type+class) + 11 (EDNS) + 2 (len) + payload
        let mut query = Vec::with_capacity(12 + domain.len() + 2 + 5 + 11 + 2 + payload.len());

        // DNS header (12 bytes)
        query.extend_from_slice(&[0x00, 0x01]); // Transaction ID
        query.extend_from_slice(&[0x01, 0x00]); // Flags (standard query)
        query.extend_from_slice(&[0x00, 0x01]); // Questions: 1
        query.extend_from_slice(&[0x00, 0x00]); // Answers: 0
        query.extend_from_slice(&[0x00, 0x00]); // Authority: 0
        query.extend_from_slice(&[0x00, 0x01]); // Additional: 1 (EDNS)

        // Question section with label validation (T5.14)
        for label in domain.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(DohError::InvalidLabel);
            }
            query.push(label.len() as u8);
            query.extend_from_slice(label.as_bytes());
        }
        query.push(0); // End of name

        query.extend_from_slice(&[0x00, 0x10]); // Type: TXT
        query.extend_from_slice(&[0x00, 0x01]); // Class: IN

        // EDNS OPT record (used to carry payload)
        query.push(0); // Name: root
        query.extend_from_slice(&[0x00, 0x29]); // Type: OPT
        query.extend_from_slice(&[0x10, 0x00]); // UDP payload size: 4096
        query.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Extended RCODE and flags

        // Encode payload in EDNS data length field
        let payload_len = payload.len() as u16;
        query.extend_from_slice(&payload_len.to_be_bytes());
        query.extend_from_slice(payload);

        Ok(query)
    }

    /// Parse fake DNS response
    ///
    /// Extracts payload from EDNS0 OPT record in a DNS response.
    ///
    /// # Errors
    ///
    /// Returns `DohError::InvalidResponse` if the response is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::doh_tunnel::DohTunnel;
    ///
    /// let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
    /// let query = tunnel.create_dns_query("test.com", b"data").unwrap();
    /// let parsed = tunnel.parse_dns_response(&query).unwrap();
    /// assert_eq!(parsed, b"data");
    /// ```
    pub fn parse_dns_response(&self, response: &[u8]) -> Result<Vec<u8>, DohError> {
        if response.len() < 12 {
            return Err(DohError::InvalidResponse);
        }

        // Skip DNS header
        let mut offset = 12;

        // Skip question section (simplified parsing) with bounds checking (T5.15)
        while offset < response.len() && response[offset] != 0 {
            let label_len = response[offset] as usize;
            if offset + 1 + label_len > response.len() {
                return Err(DohError::InvalidResponse);
            }
            offset += 1 + label_len;
        }

        if offset >= response.len() {
            return Err(DohError::InvalidResponse);
        }

        offset += 1; // Skip null terminator
        offset += 4; // Skip type + class

        // Parse EDNS section
        // Skip name (1 byte for root)
        if offset >= response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 1;

        // Skip OPT type and UDP size (class)
        if offset + 4 > response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 4;

        // Skip extended RCODE and flags
        if offset + 4 > response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 4;

        // Extract payload
        if offset + 2 > response.len() {
            return Err(DohError::InvalidResponse);
        }

        let payload_len = u16::from_be_bytes([response[offset], response[offset + 1]]) as usize;
        offset += 2;

        if offset + payload_len > response.len() {
            return Err(DohError::InvalidResponse);
        }

        Ok(response[offset..offset + payload_len].to_vec())
    }

    /// Parse fake DNS response returning a zero-copy slice reference.
    ///
    /// Like [`parse_dns_response`](Self::parse_dns_response) but returns a borrowed slice
    /// instead of allocating a Vec, for use in hot paths.
    ///
    /// # Errors
    ///
    /// Returns `DohError::InvalidResponse` if the response is malformed.
    pub fn parse_dns_response_slice<'a>(&self, response: &'a [u8]) -> Result<&'a [u8], DohError> {
        if response.len() < 12 {
            return Err(DohError::InvalidResponse);
        }

        let mut offset = 12;

        // Skip question section with bounds checking
        while offset < response.len() && response[offset] != 0 {
            let label_len = response[offset] as usize;
            if offset + 1 + label_len > response.len() {
                return Err(DohError::InvalidResponse);
            }
            offset += 1 + label_len;
        }

        if offset >= response.len() {
            return Err(DohError::InvalidResponse);
        }

        offset += 1; // null terminator
        offset += 4; // type + class

        if offset >= response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 1; // OPT name (root)

        if offset + 4 > response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 4; // OPT type + UDP size

        if offset + 4 > response.len() {
            return Err(DohError::InvalidResponse);
        }
        offset += 4; // Extended RCODE + flags

        if offset + 2 > response.len() {
            return Err(DohError::InvalidResponse);
        }

        let payload_len = u16::from_be_bytes([response[offset], response[offset + 1]]) as usize;
        offset += 2;

        if offset + payload_len > response.len() {
            return Err(DohError::InvalidResponse);
        }

        Ok(&response[offset..offset + payload_len])
    }

    /// Get the resolver URL
    #[must_use]
    pub fn resolver_url(&self) -> &str {
        &self.resolver_url
    }

    /// Set a new resolver URL
    pub fn set_resolver_url(&mut self, url: String) {
        self.resolver_url = url;
    }
}

impl Default for DohTunnel {
    fn default() -> Self {
        Self::new("https://1.1.1.1/dns-query".to_string())
    }
}

/// DoH error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DohError {
    /// Base64 decode failed
    DecodeFailed,
    /// Invalid DNS response
    InvalidResponse,
    /// Invalid DNS label (empty or exceeds 63 bytes)
    InvalidLabel,
}

impl std::fmt::Display for DohError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeFailed => write!(f, "Failed to decode base64 response"),
            Self::InvalidResponse => write!(f, "Invalid DNS response"),
            Self::InvalidLabel => write!(f, "Invalid DNS label (empty or exceeds 63 bytes)"),
        }
    }
}

impl std::error::Error for DohError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doh_encode_decode() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let payload = b"secret data";

        let query_url = tunnel.encode_query(payload);
        assert!(query_url.contains("dns="));

        // Extract base64 part
        let encoded = query_url.split("dns=").nth(1).unwrap();
        let decoded = tunnel.decode_response(encoded.as_bytes()).unwrap();

        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_doh_query_url_format() {
        let tunnel = DohTunnel::new("https://1.1.1.1/dns-query".to_string());
        let url = tunnel.encode_query(b"test");

        assert!(url.starts_with("https://1.1.1.1/dns-query?dns="));
    }

    #[test]
    fn test_dns_query_creation() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let payload = b"test";

        let query = tunnel
            .create_dns_query("wraith.example.com", payload)
            .unwrap();

        // Should have DNS header
        assert!(query.len() > 12);

        // Parse it back
        let parsed = tunnel.parse_dns_response(&query).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn test_dns_query_structure() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let query = tunnel.create_dns_query("test.com", b"data").unwrap();

        // Check DNS header fields
        assert_eq!(query[0..2], [0x00, 0x01]); // Transaction ID
        assert_eq!(query[2..4], [0x01, 0x00]); // Flags
        assert_eq!(query[4..6], [0x00, 0x01]); // Questions: 1
        assert_eq!(query[6..8], [0x00, 0x00]); // Answers: 0
        assert_eq!(query[8..10], [0x00, 0x00]); // Authority: 0
        assert_eq!(query[10..12], [0x00, 0x01]); // Additional: 1
    }

    #[test]
    fn test_doh_roundtrip() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        for i in 0..10 {
            let payload = format!("message {}", i);
            let query = tunnel
                .create_dns_query("test.com", payload.as_bytes())
                .unwrap();
            let parsed = tunnel.parse_dns_response(&query).unwrap();

            assert_eq!(parsed, payload.as_bytes());
        }
    }

    #[test]
    fn test_doh_empty_payload() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let empty: &[u8] = &[];

        let query = tunnel.create_dns_query("test.com", empty).unwrap();
        let parsed = tunnel.parse_dns_response(&query).unwrap();

        assert_eq!(parsed.len(), 0);
    }

    #[test]
    fn test_doh_large_payload() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let large = vec![0x42; 1000];

        let query = tunnel.create_dns_query("test.com", &large).unwrap();
        let parsed = tunnel.parse_dns_response(&query).unwrap();

        assert_eq!(parsed, large);
    }

    #[test]
    fn test_doh_decode_invalid() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        // Invalid base64
        let invalid = b"!!!invalid!!!";
        assert!(matches!(
            tunnel.decode_response(invalid),
            Err(DohError::DecodeFailed)
        ));
    }

    #[test]
    fn test_dns_parse_too_short() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let short = [0x00; 11]; // Less than 12 bytes

        assert!(matches!(
            tunnel.parse_dns_response(&short),
            Err(DohError::InvalidResponse)
        ));
    }

    #[test]
    fn test_dns_parse_incomplete() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        // Valid header but incomplete question section
        let incomplete = [
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x04, 0x74,
            0x65, 0x73, 0x74,
        ];

        assert!(matches!(
            tunnel.parse_dns_response(&incomplete),
            Err(DohError::InvalidResponse)
        ));
    }

    #[test]
    fn test_doh_default() {
        let tunnel = DohTunnel::default();
        assert_eq!(tunnel.resolver_url(), "https://1.1.1.1/dns-query");
    }

    #[test]
    fn test_doh_resolver_url_getter() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        assert_eq!(tunnel.resolver_url(), "https://dns.example.com/dns-query");
    }

    #[test]
    fn test_doh_resolver_url_setter() {
        let mut tunnel = DohTunnel::new("https://old.example.com/dns-query".to_string());
        tunnel.set_resolver_url("https://new.example.com/dns-query".to_string());

        assert_eq!(tunnel.resolver_url(), "https://new.example.com/dns-query");
    }

    #[test]
    fn test_doh_error_display() {
        assert_eq!(
            format!("{}", DohError::DecodeFailed),
            "Failed to decode base64 response"
        );
        assert_eq!(
            format!("{}", DohError::InvalidResponse),
            "Invalid DNS response"
        );
    }

    #[test]
    fn test_base64url_encoding() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        // Test that URL-safe encoding is used (no + or /)
        let payload = b"\xFF\xFE\xFD";
        let url = tunnel.encode_query(payload);

        // Extract the dns= parameter value
        let encoded = url.split("dns=").nth(1).unwrap();

        // URL-safe base64 should not contain + or /
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn test_dns_domain_encoding() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let query = tunnel.create_dns_query("example.com", b"test").unwrap();

        // Find the question section (starts after header at offset 12)
        // Should be: 7 "example" 3 "com" 0
        assert_eq!(query[12], 7); // Length of "example"
        assert_eq!(&query[13..20], b"example");
        assert_eq!(query[20], 3); // Length of "com"
        assert_eq!(&query[21..24], b"com");
        assert_eq!(query[24], 0); // End of domain
    }

    #[test]
    fn test_dns_subdomain_encoding() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let query = tunnel
            .create_dns_query("test.wraith.example.com", b"data")
            .unwrap();

        // Check domain encoding
        let offset = 12;
        assert_eq!(query[offset], 4); // "test"
        assert_eq!(&query[offset + 1..offset + 5], b"test");
        assert_eq!(query[offset + 5], 6); // "wraith"
    }

    #[test]
    fn test_doh_multiple_messages() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        let messages = vec![b"msg1".as_slice(), b"msg2", b"msg3"];

        for msg in messages {
            let query = tunnel.create_dns_query("test.com", msg).unwrap();
            let parsed = tunnel.parse_dns_response(&query).unwrap();
            assert_eq!(parsed, msg);
        }
    }

    #[test]
    fn test_doh_special_characters() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());

        // Test payload with special characters
        let payload = b"test\n\r\t\0data";
        let query = tunnel.create_dns_query("test.com", payload).unwrap();
        let parsed = tunnel.parse_dns_response(&query).unwrap();

        assert_eq!(parsed, payload);
    }
}
