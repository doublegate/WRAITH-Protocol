//! STUN Protocol Implementation (RFC 5389)
//!
//! This module implements the STUN (Session Traversal Utilities for NAT) protocol
//! for discovering server reflexive addresses and performing NAT type detection.
//!
//! # SEC-003: STUN Security Hardening
//!
//! This implementation includes RFC 5389 MESSAGE-INTEGRITY authentication using
//! HMAC-SHA1, transaction ID validation, and fingerprint verification for secure
//! STUN operations.
//!
//! # Security Note on Cryptographic Algorithms
//!
//! **IMPORTANT:** This module uses MD5 and SHA1 algorithms as mandated by RFC 5389
//! for STUN protocol compliance. While MD5 and SHA1 are considered cryptographically
//! weak for general purposes, their use here is:
//!
//! 1. **RFC-Mandated**: RFC 5389 Section 15.4 specifically requires HMAC-SHA1 for
//!    MESSAGE-INTEGRITY and MD5 for long-term credential key derivation.
//! 2. **Protocol-Specific**: These algorithms are used only for STUN protocol
//!    operations, not for general cryptographic purposes in WRAITH.
//! 3. **Contextually Safe**: In the STUN context with proper authentication and
//!    message integrity checks, the protocol provides adequate security for its
//!    intended NAT traversal purpose.
//!
//! **DO NOT** use MD5 or SHA1 from this module for any other cryptographic
//! operations. For general cryptography in WRAITH, use the strong algorithms
//! provided in the `wraith-crypto` crate (e.g., BLAKE3, ChaCha20-Poly1305, X25519, Ed25519).

use hmac::{Hmac, Mac};
// These imports are required by RFC 5389 for STUN protocol compliance.
// DO NOT use for general cryptographic purposes - see security note above.
use md5::Md5;
use sha1::Sha1;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use zeroize::Zeroizing;

/// STUN magic cookie (0x2112A442)
const MAGIC_COOKIE: u32 = 0x2112_A442;

/// STUN message header size (20 bytes)
const HEADER_SIZE: usize = 20;

/// Default STUN timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

/// STUN message class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StunMessageClass {
    /// Request message
    Request,
    /// Success response
    SuccessResponse,
    /// Error response
    ErrorResponse,
    /// Indication (no response expected)
    Indication,
}

/// STUN message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StunMessageType {
    /// Binding request/response
    Binding,
}

impl StunMessageType {
    /// Encode message type and class into a 16-bit value
    ///
    /// RFC 5389 Section 6 encoding:
    /// ```text
    ///  0                 1
    ///  2  3  4 5 6 7 8 9 0 1 2 3 4 5
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |M |M |M|M|M|C|M|M|M|C|M|M|M|M|
    /// |11|10|9|8|7|1|6|5|4|0|3|2|1|0|
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    /// ```
    fn encode(&self, class: StunMessageClass) -> u16 {
        let method = match self {
            Self::Binding => 0x0001,
        };

        let class_bits = match class {
            StunMessageClass::Request => 0b00,
            StunMessageClass::Indication => 0b01,
            StunMessageClass::SuccessResponse => 0b10,
            StunMessageClass::ErrorResponse => 0b11,
        };

        // STUN message type encoding (RFC 5389 Section 6)
        // Bits 0-3: M0-M3
        let m0_m3 = method & 0x0F;
        // Bit 4: C0
        let c0 = (class_bits & 0x01) << 4;
        // Bits 5-7: M4-M6
        let m4_m6 = (method & 0x70) << 1;
        // Bit 8: C1
        let c1 = (class_bits & 0x02) << 7;
        // Bits 9-13: M7-M11
        let m7_m11 = (method & 0xF80) << 2;

        m0_m3 | c0 | m4_m6 | c1 | m7_m11
    }
}

// ============================================================================
// SEC-003: STUN Authentication and Security
// ============================================================================

/// STUN authentication credentials (SEC-003)
///
/// Provides MESSAGE-INTEGRITY authentication as per RFC 5389.
/// Credentials are zeroized on drop to prevent memory disclosure.
#[derive(Clone, Debug)]
pub struct StunAuthentication {
    /// Username for authentication
    pub username: String,
    /// Password (zeroized on drop)
    password: Zeroizing<String>,
    /// Realm for long-term credentials (optional)
    pub realm: Option<String>,
}

impl StunAuthentication {
    /// Create new STUN authentication credentials
    ///
    /// # Arguments
    ///
    /// * `username` - Username
    /// * `password` - Password (will be zeroized on drop)
    /// * `realm` - Optional realm for long-term credentials
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::nat::StunAuthentication;
    ///
    /// let auth = StunAuthentication::new("user", "pass", Some("example.com".to_string()));
    /// ```
    #[must_use]
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        realm: Option<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: Zeroizing::new(password.into()),
            realm,
        }
    }

    /// Derive HMAC key for MESSAGE-INTEGRITY
    ///
    /// For long-term credentials: MD5(username:realm:password)
    /// For short-term credentials: password
    ///
    /// # Security Note
    ///
    /// This function uses MD5 as required by RFC 5389 Section 15.4 for STUN
    /// long-term credential key derivation. This is a protocol requirement
    /// and cannot be changed without breaking STUN compatibility.
    /// MD5 is used here only for protocol compliance, not for collision resistance.
    fn derive_key(&self) -> Zeroizing<Vec<u8>> {
        if let Some(realm) = &self.realm {
            // Long-term credentials: MD5(username:realm:password)
            // Required by RFC 5389 Section 15.4 - DO NOT change to a different hash
            use md5::Digest;
            let mut hasher = Md5::new();
            hasher.update(self.username.as_bytes());
            hasher.update(b":");
            hasher.update(realm.as_bytes());
            hasher.update(b":");
            hasher.update(self.password.as_bytes());
            Zeroizing::new(hasher.finalize().to_vec())
        } else {
            // Short-term credentials: use password directly
            Zeroizing::new(self.password.as_bytes().to_vec())
        }
    }
}

/// Rate limiter for STUN requests (SEC-003)
///
/// Limits requests per IP address to prevent abuse.
#[derive(Clone)]
pub struct StunRateLimiter {
    /// Request timestamps per IP
    requests: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    /// Maximum requests per second
    max_requests_per_second: usize,
    /// Time window for rate limiting
    window: Duration,
}

impl StunRateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_requests_per_second` - Maximum requests allowed per IP per second
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::nat::StunRateLimiter;
    ///
    /// let limiter = StunRateLimiter::new(10); // 10 requests per second
    /// ```
    #[must_use]
    pub fn new(max_requests_per_second: usize) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_second,
            window: Duration::from_secs(1),
        }
    }

    /// Check if request from IP should be allowed
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address of requester
    ///
    /// # Returns
    ///
    /// `true` if request is allowed, `false` if rate limit exceeded
    pub fn allow_request(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        // Get or create request history for this IP
        let history = requests.entry(ip).or_default();

        // Remove old requests outside the window
        history.retain(|&timestamp| now.duration_since(timestamp) < self.window);

        // Check rate limit
        if history.len() >= self.max_requests_per_second {
            return false;
        }

        // Record this request
        history.push(now);
        true
    }

    /// Clear old entries from rate limiter (periodic cleanup)
    pub fn cleanup(&self) {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        requests.retain(|_, history| {
            history.retain(|&timestamp| now.duration_since(timestamp) < self.window);
            !history.is_empty()
        });
    }
}

impl Default for StunRateLimiter {
    fn default() -> Self {
        Self::new(10) // 10 requests per second by default
    }
}

/// STUN attribute types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StunAttribute {
    /// Mapped address (0x0001)
    MappedAddress(SocketAddr),
    /// XOR-Mapped address (0x0020) - preferred over MAPPED-ADDRESS
    XorMappedAddress(SocketAddr),
    /// Username (0x0006) - for authentication
    Username(String),
    /// Message integrity (0x0008) - HMAC-SHA1
    MessageIntegrity([u8; 20]),
    /// Software identifier (0x8022)
    Software(String),
    /// Fingerprint (0x8028) - CRC-32
    Fingerprint(u32),
    /// Unknown attribute type
    Unknown(u16, Vec<u8>),
}

impl StunAttribute {
    /// Attribute type code
    fn attr_type(&self) -> u16 {
        match self {
            Self::MappedAddress(_) => 0x0001,
            Self::Username(_) => 0x0006,
            Self::MessageIntegrity(_) => 0x0008,
            Self::XorMappedAddress(_) => 0x0020,
            Self::Software(_) => 0x8022,
            Self::Fingerprint(_) => 0x8028,
            Self::Unknown(t, _) => *t,
        }
    }

    /// Encode attribute to bytes
    fn encode(&self, transaction_id: &[u8; 12]) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Attribute type (2 bytes)
        bytes.extend_from_slice(&self.attr_type().to_be_bytes());

        // Attribute value
        let value = self.encode_value(transaction_id);

        // Attribute length (2 bytes)
        bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());

        // Attribute value
        bytes.extend_from_slice(&value);

        // Padding to 4-byte boundary
        let padding = (4 - (value.len() % 4)) % 4;
        bytes.extend(std::iter::repeat_n(0, padding));

        bytes
    }

    fn encode_value(&self, transaction_id: &[u8; 12]) -> Vec<u8> {
        match self {
            Self::XorMappedAddress(addr) => {
                let mut value = Vec::new();
                value.push(0); // Reserved
                value.push(if addr.is_ipv4() { 0x01 } else { 0x02 });

                // XOR port with most significant 16 bits of magic cookie
                let xor_port = addr.port() ^ (MAGIC_COOKIE >> 16) as u16;
                value.extend_from_slice(&xor_port.to_be_bytes());

                // XOR address with magic cookie (+ transaction ID for IPv6)
                match addr.ip() {
                    std::net::IpAddr::V4(ipv4) => {
                        let ip_bytes = ipv4.octets();
                        let magic_bytes = MAGIC_COOKIE.to_be_bytes();
                        for i in 0..4 {
                            value.push(ip_bytes[i] ^ magic_bytes[i]);
                        }
                    }
                    std::net::IpAddr::V6(ipv6) => {
                        let ip_bytes = ipv6.octets();
                        let mut xor_key = MAGIC_COOKIE.to_be_bytes().to_vec();
                        xor_key.extend_from_slice(transaction_id);
                        for i in 0..16 {
                            value.push(ip_bytes[i] ^ xor_key[i]);
                        }
                    }
                }

                value
            }
            Self::Username(u) => u.as_bytes().to_vec(),
            Self::MessageIntegrity(hmac) => hmac.to_vec(),
            Self::Software(s) => s.as_bytes().to_vec(),
            Self::Fingerprint(f) => f.to_be_bytes().to_vec(),
            Self::Unknown(_, data) => data.clone(),
            Self::MappedAddress(_) => Vec::new(), // Not implemented
        }
    }

    /// Decode attribute from bytes
    fn decode(attr_type: u16, value: &[u8], transaction_id: &[u8; 12]) -> Result<Self, StunError> {
        match attr_type {
            0x0006 => {
                // USERNAME
                let s = String::from_utf8_lossy(value).to_string();
                Ok(Self::Username(s))
            }
            0x0008 => {
                // MESSAGE-INTEGRITY
                if value.len() != 20 {
                    return Err(StunError::InvalidAttribute);
                }
                let mut hmac = [0u8; 20];
                hmac.copy_from_slice(value);
                Ok(Self::MessageIntegrity(hmac))
            }
            0x0020 => {
                // XOR-MAPPED-ADDRESS
                if value.len() < 4 {
                    return Err(StunError::InvalidAttribute);
                }

                let family = value[1];
                let xor_port = u16::from_be_bytes([value[2], value[3]]);
                let port = xor_port ^ (MAGIC_COOKIE >> 16) as u16;

                let addr = if family == 0x01 {
                    // IPv4
                    if value.len() < 8 {
                        return Err(StunError::InvalidAttribute);
                    }
                    let magic_bytes = MAGIC_COOKIE.to_be_bytes();
                    let mut ip_bytes = [0u8; 4];
                    for i in 0..4 {
                        ip_bytes[i] = value[4 + i] ^ magic_bytes[i];
                    }
                    let ip = std::net::Ipv4Addr::from(ip_bytes);
                    SocketAddr::new(ip.into(), port)
                } else {
                    // IPv6
                    if value.len() < 20 {
                        return Err(StunError::InvalidAttribute);
                    }
                    let mut xor_key = MAGIC_COOKIE.to_be_bytes().to_vec();
                    xor_key.extend_from_slice(transaction_id);
                    let mut ip_bytes = [0u8; 16];
                    for i in 0..16 {
                        ip_bytes[i] = value[4 + i] ^ xor_key[i];
                    }
                    let ip = std::net::Ipv6Addr::from(ip_bytes);
                    SocketAddr::new(ip.into(), port)
                };

                Ok(Self::XorMappedAddress(addr))
            }
            0x8022 => {
                // SOFTWARE
                let s = String::from_utf8_lossy(value).to_string();
                Ok(Self::Software(s))
            }
            0x8028 => {
                // FINGERPRINT
                if value.len() != 4 {
                    return Err(StunError::InvalidAttribute);
                }
                let fingerprint = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
                Ok(Self::Fingerprint(fingerprint))
            }
            _ => Ok(Self::Unknown(attr_type, value.to_vec())),
        }
    }
}

/// STUN message
#[derive(Debug, Clone)]
pub struct StunMessage {
    /// Message type
    pub message_type: StunMessageType,
    /// Message class
    pub message_class: StunMessageClass,
    /// Transaction ID (96 bits)
    pub transaction_id: [u8; 12],
    /// Message attributes
    pub attributes: Vec<StunAttribute>,
}

impl StunMessage {
    /// Create a new STUN Binding Request
    #[must_use]
    pub fn binding_request() -> Self {
        let mut transaction_id = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut transaction_id);

        Self {
            message_type: StunMessageType::Binding,
            message_class: StunMessageClass::Request,
            transaction_id,
            attributes: Vec::new(),
        }
    }

    /// Add an attribute to the message
    pub fn add_attribute(&mut self, attr: StunAttribute) {
        self.attributes.push(attr);
    }

    /// Encode message to bytes
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Message Type (2 bytes)
        let msg_type = self.message_type.encode(self.message_class);
        bytes.extend_from_slice(&msg_type.to_be_bytes());

        // Message Length (2 bytes) - placeholder
        let length_offset = bytes.len();
        bytes.extend_from_slice(&[0u8; 2]);

        // Magic Cookie (4 bytes)
        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());

        // Transaction ID (12 bytes)
        bytes.extend_from_slice(&self.transaction_id);

        // Attributes
        for attr in &self.attributes {
            bytes.extend_from_slice(&attr.encode(&self.transaction_id));
        }

        // Update message length (excludes 20-byte header)
        let msg_length = bytes.len() - HEADER_SIZE;
        bytes[length_offset..length_offset + 2].copy_from_slice(&(msg_length as u16).to_be_bytes());

        bytes
    }

    /// Decode message from bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, StunError> {
        if bytes.len() < HEADER_SIZE {
            return Err(StunError::MessageTooShort);
        }

        // Parse header
        let msg_type = u16::from_be_bytes([bytes[0], bytes[1]]);
        let msg_length = u16::from_be_bytes([bytes[2], bytes[3]]) as usize;
        let magic_cookie = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        if magic_cookie != MAGIC_COOKIE {
            return Err(StunError::InvalidMagicCookie);
        }

        let mut transaction_id = [0u8; 12];
        transaction_id.copy_from_slice(&bytes[8..20]);

        // Decode message type and class
        let (message_type, message_class) = Self::decode_type(msg_type)?;

        // Parse attributes
        let mut attributes = Vec::new();
        let mut offset = HEADER_SIZE;

        while offset < bytes.len() && offset - HEADER_SIZE < msg_length {
            if offset + 4 > bytes.len() {
                break;
            }

            let attr_type = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
            let attr_length = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;

            offset += 4;

            if offset + attr_length > bytes.len() {
                break;
            }

            let attr_value = &bytes[offset..offset + attr_length];
            if let Ok(attr) = StunAttribute::decode(attr_type, attr_value, &transaction_id) {
                attributes.push(attr);
            }

            offset += attr_length;

            // Skip padding to 4-byte boundary
            let padding = (4 - (attr_length % 4)) % 4;
            offset += padding;
        }

        Ok(Self {
            message_type,
            message_class,
            transaction_id,
            attributes,
        })
    }

    fn decode_type(msg_type: u16) -> Result<(StunMessageType, StunMessageClass), StunError> {
        // Extract class bits (C0 at bit 4, C1 at bit 8)
        let c0 = (msg_type >> 4) & 0x01;
        let c1 = (msg_type >> 8) & 0x01;
        let class_bits = c0 | (c1 << 1);

        let message_class = match class_bits {
            0b00 => StunMessageClass::Request,
            0b01 => StunMessageClass::Indication,
            0b10 => StunMessageClass::SuccessResponse,
            0b11 => StunMessageClass::ErrorResponse,
            _ => return Err(StunError::InvalidMessageType),
        };

        // Extract method bits
        let m0_m3 = msg_type & 0x0F;
        let m4_m6 = (msg_type >> 1) & 0x70;
        let m7_m11 = (msg_type >> 2) & 0xF80;
        let method = m0_m3 | m4_m6 | m7_m11;

        let message_type = match method {
            0x0001 => StunMessageType::Binding,
            _ => return Err(StunError::InvalidMessageType),
        };

        Ok((message_type, message_class))
    }

    /// Get XOR-MAPPED-ADDRESS attribute
    #[must_use]
    pub fn xor_mapped_address(&self) -> Option<SocketAddr> {
        for attr in &self.attributes {
            if let StunAttribute::XorMappedAddress(addr) = attr {
                return Some(*addr);
            }
        }
        None
    }

    // ========================================================================
    // SEC-003: MESSAGE-INTEGRITY and FINGERPRINT Support
    // ========================================================================

    /// Add MESSAGE-INTEGRITY attribute (SEC-003)
    ///
    /// Computes HMAC-SHA1 over the message up to (but not including) the
    /// MESSAGE-INTEGRITY attribute itself. Must be called before encoding.
    ///
    /// # Security Note
    ///
    /// This function uses HMAC-SHA1 as mandated by RFC 5389 Section 15.4 for
    /// STUN MESSAGE-INTEGRITY computation. This is a protocol requirement and
    /// cannot be changed without breaking STUN compatibility with other implementations.
    /// The use of HMAC-SHA1 here is for protocol compliance only.
    ///
    /// # Arguments
    ///
    /// * `auth` - Authentication credentials
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::nat::{StunMessage, StunAuthentication};
    ///
    /// let mut msg = StunMessage::binding_request();
    /// let auth = StunAuthentication::new("user", "pass", None);
    /// msg.add_message_integrity(&auth);
    /// ```
    pub fn add_message_integrity(&mut self, auth: &StunAuthentication) {
        // First, encode the message without MESSAGE-INTEGRITY
        let mut temp_msg = self.clone();
        temp_msg.attributes.retain(|attr| {
            !matches!(attr, StunAttribute::MessageIntegrity(_))
                && !matches!(attr, StunAttribute::Fingerprint(_))
        });

        // Encode message
        let mut bytes = Vec::new();
        let msg_type = self.message_type.encode(self.message_class);
        bytes.extend_from_slice(&msg_type.to_be_bytes());

        // Message Length placeholder (will include MESSAGE-INTEGRITY)
        let length_offset = bytes.len();
        bytes.extend_from_slice(&[0u8; 2]);

        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        bytes.extend_from_slice(&self.transaction_id);

        // Add existing attributes (except MESSAGE-INTEGRITY and FINGERPRINT)
        for attr in &temp_msg.attributes {
            bytes.extend_from_slice(&attr.encode(&self.transaction_id));
        }

        // Update message length to include MESSAGE-INTEGRITY (24 bytes: 4 header + 20 HMAC)
        let msg_length = bytes.len() - HEADER_SIZE + 24;
        bytes[length_offset..length_offset + 2].copy_from_slice(&(msg_length as u16).to_be_bytes());

        // Compute HMAC-SHA1 (RFC 5389 Section 15.4 requirement)
        let key = auth.derive_key();
        type HmacSha1 = Hmac<Sha1>;
        let mut mac = HmacSha1::new_from_slice(&key).expect("HMAC can take key of any size");
        mac.update(&bytes);
        let result = mac.finalize();
        let hmac_bytes: [u8; 20] = result.into_bytes().into();

        // Add MESSAGE-INTEGRITY attribute
        self.attributes
            .push(StunAttribute::MessageIntegrity(hmac_bytes));
    }

    /// Verify MESSAGE-INTEGRITY attribute (SEC-003)
    ///
    /// Validates that the MESSAGE-INTEGRITY HMAC is correct.
    ///
    /// # Security Note
    ///
    /// This function uses HMAC-SHA1 as mandated by RFC 5389 Section 15.4 for
    /// STUN MESSAGE-INTEGRITY verification. This is a protocol requirement.
    ///
    /// # Arguments
    ///
    /// * `auth` - Authentication credentials
    ///
    /// # Errors
    ///
    /// Returns error if MESSAGE-INTEGRITY is missing or invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::nat::{StunMessage, StunAuthentication};
    ///
    /// let mut msg = StunMessage::binding_request();
    /// let auth = StunAuthentication::new("user", "pass", None);
    /// msg.add_message_integrity(&auth);
    ///
    /// // Verify
    /// assert!(msg.verify_message_integrity(&auth).is_ok());
    /// ```
    pub fn verify_message_integrity(&self, auth: &StunAuthentication) -> Result<(), StunError> {
        // Find MESSAGE-INTEGRITY attribute
        let mut msg_integrity_hmac = None;
        let mut msg_integrity_index = 0;
        for (i, attr) in self.attributes.iter().enumerate() {
            if let StunAttribute::MessageIntegrity(hmac) = attr {
                msg_integrity_hmac = Some(*hmac);
                msg_integrity_index = i;
                break;
            }
        }

        let expected_hmac = msg_integrity_hmac.ok_or(StunError::MissingAttribute)?;

        // Rebuild message up to MESSAGE-INTEGRITY
        let mut temp_msg = self.clone();
        temp_msg.attributes.truncate(msg_integrity_index);

        let mut bytes = Vec::new();
        let msg_type = temp_msg.message_type.encode(temp_msg.message_class);
        bytes.extend_from_slice(&msg_type.to_be_bytes());

        // Message Length (includes MESSAGE-INTEGRITY header + value)
        let mut attr_length = 0;
        for attr in &temp_msg.attributes {
            let encoded = attr.encode(&temp_msg.transaction_id);
            attr_length += encoded.len();
        }
        attr_length += 24; // MESSAGE-INTEGRITY attribute (4 + 20)

        bytes.extend_from_slice(&(attr_length as u16).to_be_bytes());
        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        bytes.extend_from_slice(&temp_msg.transaction_id);

        for attr in &temp_msg.attributes {
            bytes.extend_from_slice(&attr.encode(&temp_msg.transaction_id));
        }

        // Compute expected HMAC (RFC 5389 Section 15.4 requirement)
        let key = auth.derive_key();
        type HmacSha1 = Hmac<Sha1>;
        let mut mac = HmacSha1::new_from_slice(&key).expect("HMAC can take key of any size");
        mac.update(&bytes);
        let result = mac.finalize();
        let computed_hmac: [u8; 20] = result.into_bytes().into();

        if computed_hmac == expected_hmac {
            Ok(())
        } else {
            Err(StunError::AuthenticationFailed)
        }
    }

    /// Add FINGERPRINT attribute (SEC-003)
    ///
    /// Computes CRC-32 over the message and XORs with 0x5354554e.
    /// Must be called after MESSAGE-INTEGRITY if both are used.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::nat::StunMessage;
    ///
    /// let mut msg = StunMessage::binding_request();
    /// msg.add_fingerprint();
    /// ```
    pub fn add_fingerprint(&mut self) {
        // Remove existing FINGERPRINT if present
        self.attributes
            .retain(|attr| !matches!(attr, StunAttribute::Fingerprint(_)));

        // Encode message without FINGERPRINT
        let mut bytes = Vec::new();
        let msg_type = self.message_type.encode(self.message_class);
        bytes.extend_from_slice(&msg_type.to_be_bytes());

        // Message Length (will include FINGERPRINT)
        let length_offset = bytes.len();
        bytes.extend_from_slice(&[0u8; 2]);

        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        bytes.extend_from_slice(&self.transaction_id);

        for attr in &self.attributes {
            bytes.extend_from_slice(&attr.encode(&self.transaction_id));
        }

        // Update length to include FINGERPRINT (8 bytes: 4 header + 4 CRC)
        let msg_length = bytes.len() - HEADER_SIZE + 8;
        bytes[length_offset..length_offset + 2].copy_from_slice(&(msg_length as u16).to_be_bytes());

        // Compute CRC-32
        let crc = Self::crc32(&bytes);
        let fingerprint = crc ^ 0x5354_554e; // XOR with defined constant

        self.attributes
            .push(StunAttribute::Fingerprint(fingerprint));
    }

    /// Verify FINGERPRINT attribute (SEC-003)
    ///
    /// # Errors
    ///
    /// Returns error if FINGERPRINT is missing or invalid
    pub fn verify_fingerprint(&self) -> Result<(), StunError> {
        // Find FINGERPRINT attribute (must be last)
        let fingerprint_value = self
            .attributes
            .iter()
            .rev()
            .find_map(|attr| {
                if let StunAttribute::Fingerprint(fp) = attr {
                    Some(*fp)
                } else {
                    None
                }
            })
            .ok_or(StunError::MissingAttribute)?;

        // Rebuild message up to FINGERPRINT
        let mut temp_msg = self.clone();
        temp_msg
            .attributes
            .retain(|attr| !matches!(attr, StunAttribute::Fingerprint(_)));

        let mut bytes = Vec::new();
        let msg_type = temp_msg.message_type.encode(temp_msg.message_class);
        bytes.extend_from_slice(&msg_type.to_be_bytes());

        // Message Length
        let mut attr_length = 0;
        for attr in &temp_msg.attributes {
            let encoded = attr.encode(&temp_msg.transaction_id);
            attr_length += encoded.len();
        }
        attr_length += 8; // FINGERPRINT attribute

        bytes.extend_from_slice(&(attr_length as u16).to_be_bytes());
        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        bytes.extend_from_slice(&temp_msg.transaction_id);

        for attr in &temp_msg.attributes {
            bytes.extend_from_slice(&attr.encode(&temp_msg.transaction_id));
        }

        // Compute expected fingerprint
        let crc = Self::crc32(&bytes);
        let expected_fingerprint = crc ^ 0x5354_554e;

        if expected_fingerprint == fingerprint_value {
            Ok(())
        } else {
            Err(StunError::FingerprintMismatch)
        }
    }

    /// Compute CRC-32 (polynomial 0x04C11DB7)
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for &byte in data {
            crc ^= u32::from(byte) << 24;
            for _ in 0..8 {
                if crc & 0x8000_0000 != 0 {
                    crc = (crc << 1) ^ 0x04C1_1DB7;
                } else {
                    crc <<= 1;
                }
            }
        }
        !crc
    }
}

/// STUN client for server reflexive address discovery
pub struct StunClient {
    socket: UdpSocket,
    timeout: Duration,
}

impl StunClient {
    /// Bind a new STUN client to a local address
    ///
    /// # Errors
    ///
    /// Returns an error if the socket cannot be bound
    pub async fn bind(addr: &str) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Set query timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get local socket address
    ///
    /// # Errors
    ///
    /// Returns an error if the local address cannot be determined
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.socket.local_addr()
    }

    /// Get mapped address from STUN server
    ///
    /// # Errors
    ///
    /// Returns `StunError` if:
    /// - Network I/O fails
    /// - STUN server doesn't respond within timeout
    /// - Response is invalid or missing XOR-MAPPED-ADDRESS
    pub async fn get_mapped_address(&self, server: SocketAddr) -> Result<SocketAddr, StunError> {
        // Create binding request
        let request = StunMessage::binding_request();
        let request_bytes = request.encode();
        let transaction_id = request.transaction_id;

        // Send request
        self.socket.send_to(&request_bytes, server).await?;

        // Receive response with timeout
        let mut buf = [0u8; 1024];

        let (len, _from) = tokio::time::timeout(self.timeout, self.socket.recv_from(&mut buf))
            .await
            .map_err(|_| StunError::Timeout)??;

        // Decode response
        let response = StunMessage::decode(&buf[..len])?;

        // Verify transaction ID
        if response.transaction_id != transaction_id {
            return Err(StunError::TransactionMismatch);
        }

        // Verify response type
        if response.message_class != StunMessageClass::SuccessResponse {
            return Err(StunError::ErrorResponse);
        }

        // Extract XOR-MAPPED-ADDRESS
        response
            .xor_mapped_address()
            .ok_or(StunError::MissingAttribute)
    }
}

/// STUN error types
#[derive(Debug)]
pub enum StunError {
    /// I/O error
    Io(std::io::Error),
    /// Query timeout
    Timeout,
    /// Invalid message format
    MessageTooShort,
    /// Invalid magic cookie
    InvalidMagicCookie,
    /// Invalid message type
    InvalidMessageType,
    /// Invalid attribute
    InvalidAttribute,
    /// Transaction ID mismatch
    TransactionMismatch,
    /// Error response received
    ErrorResponse,
    /// Missing required attribute
    MissingAttribute,
    /// Authentication failed (SEC-003)
    AuthenticationFailed,
    /// Fingerprint mismatch (SEC-003)
    FingerprintMismatch,
    /// Rate limit exceeded (SEC-003)
    RateLimitExceeded,
}

impl std::fmt::Display for StunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Timeout => write!(f, "STUN query timeout"),
            Self::MessageTooShort => write!(f, "STUN message too short"),
            Self::InvalidMagicCookie => write!(f, "Invalid STUN magic cookie"),
            Self::InvalidMessageType => write!(f, "Invalid STUN message type"),
            Self::InvalidAttribute => write!(f, "Invalid STUN attribute"),
            Self::TransactionMismatch => write!(f, "Transaction ID mismatch"),
            Self::ErrorResponse => write!(f, "STUN error response"),
            Self::MissingAttribute => write!(f, "Missing required STUN attribute"),
            Self::AuthenticationFailed => write!(f, "MESSAGE-INTEGRITY authentication failed"),
            Self::FingerprintMismatch => write!(f, "FINGERPRINT verification failed"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
        }
    }
}

impl std::error::Error for StunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StunError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_message_type_encoding() {
        // Binding Request: method=0x0001, class=Request(0b00)
        let encoded = StunMessageType::Binding.encode(StunMessageClass::Request);
        assert_eq!(encoded, 0x0001);

        // Binding Success Response: method=0x0001, class=SuccessResponse(0b10)
        // Class bits: C0=0 (bit 4), C1=1 (bit 8)
        // Result: 0x0001 | (0 << 4) | (1 << 8) = 0x0001 | 0x0100 = 0x0101
        let encoded = StunMessageType::Binding.encode(StunMessageClass::SuccessResponse);
        assert_eq!(encoded, 0x0101);
    }

    #[test]
    fn test_stun_message_roundtrip() {
        let msg = StunMessage::binding_request();
        let encoded = msg.encode();

        assert!(encoded.len() >= HEADER_SIZE);

        let decoded = StunMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.message_type, msg.message_type);
        assert_eq!(decoded.message_class, msg.message_class);
        assert_eq!(decoded.transaction_id, msg.transaction_id);
    }

    #[test]
    fn test_xor_mapped_address_attribute() {
        let addr: SocketAddr = "192.0.2.1:32853".parse().unwrap();
        let transaction_id = [0u8; 12];

        let attr = StunAttribute::XorMappedAddress(addr);
        let encoded = attr.encode(&transaction_id);

        // Decode it back
        let value = &encoded[4..]; // Skip type and length
        let decoded = StunAttribute::decode(0x0020, value, &transaction_id).unwrap();

        if let StunAttribute::XorMappedAddress(decoded_addr) = decoded {
            assert_eq!(decoded_addr, addr);
        } else {
            panic!("Expected XorMappedAddress");
        }
    }

    #[test]
    fn test_magic_cookie() {
        assert_eq!(MAGIC_COOKIE, 0x2112_A442);
    }

    // SEC-003: STUN Security Hardening Tests
    #[test]
    fn test_stun_authentication() {
        // SECURITY: Intentional test credentials, not production secrets
        let auth = StunAuthentication::new("testuser", "testpass", None);
        assert_eq!(auth.username, "testuser");
        assert_eq!(auth.realm, None);
    }

    #[test]
    fn test_stun_authentication_with_realm() {
        // SECURITY: Intentional test credentials, not production secrets
        let auth = StunAuthentication::new("user", "pass", Some("example.com".to_string()));
        assert_eq!(auth.username, "user");
        assert_eq!(auth.realm, Some("example.com".to_string()));
    }

    #[test]
    fn test_message_integrity_roundtrip() {
        // SECURITY: Intentional test credentials, not production secrets
        let mut msg = StunMessage::binding_request();
        let auth = StunAuthentication::new("user", "pass", None);

        msg.add_message_integrity(&auth);

        // Should have MESSAGE-INTEGRITY attribute
        let has_integrity = msg
            .attributes
            .iter()
            .any(|attr| matches!(attr, StunAttribute::MessageIntegrity(_)));
        assert!(has_integrity);

        // Verify should pass
        assert!(msg.verify_message_integrity(&auth).is_ok());
    }

    #[test]
    fn test_message_integrity_wrong_password() {
        // SECURITY: Intentional test credentials, not production secrets
        let mut msg = StunMessage::binding_request();
        let auth1 = StunAuthentication::new("user", "pass1", None);
        let auth2 = StunAuthentication::new("user", "pass2", None);

        msg.add_message_integrity(&auth1);

        // Verify with wrong password should fail
        assert!(msg.verify_message_integrity(&auth2).is_err());
    }

    #[test]
    fn test_message_integrity_long_term_credentials() {
        // SECURITY: Intentional test credentials, not production secrets
        let mut msg = StunMessage::binding_request();
        let auth = StunAuthentication::new("user", "pass", Some("example.com".to_string()));

        msg.add_message_integrity(&auth);
        assert!(msg.verify_message_integrity(&auth).is_ok());
    }

    #[test]
    fn test_fingerprint_roundtrip() {
        let mut msg = StunMessage::binding_request();
        msg.add_fingerprint();

        // Should have FINGERPRINT attribute
        let has_fingerprint = msg
            .attributes
            .iter()
            .any(|attr| matches!(attr, StunAttribute::Fingerprint(_)));
        assert!(has_fingerprint);

        // Verify should pass
        assert!(msg.verify_fingerprint().is_ok());
    }

    #[test]
    fn test_fingerprint_tampered_message() {
        let mut msg = StunMessage::binding_request();
        msg.add_fingerprint();

        // Tamper with message
        msg.add_attribute(StunAttribute::Software("tampered".to_string()));

        // Fingerprint verification should fail
        assert!(msg.verify_fingerprint().is_err());
    }

    #[test]
    fn test_message_integrity_and_fingerprint() {
        // SECURITY: Intentional test credentials, not production secrets
        let mut msg = StunMessage::binding_request();
        let auth = StunAuthentication::new("user", "pass", None);

        // Add MESSAGE-INTEGRITY first, then FINGERPRINT
        msg.add_message_integrity(&auth);
        msg.add_fingerprint();

        // Both should verify
        assert!(msg.verify_message_integrity(&auth).is_ok());
        assert!(msg.verify_fingerprint().is_ok());
    }

    #[test]
    fn test_rate_limiter_allows_requests() {
        let limiter = StunRateLimiter::new(5);
        let ip = "192.168.1.1".parse().unwrap();

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.allow_request(ip));
        }

        // 6th request should be denied
        assert!(!limiter.allow_request(ip));
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let limiter = StunRateLimiter::new(5);
        let ip1 = "192.168.1.1".parse().unwrap();
        let ip2 = "192.168.1.2".parse().unwrap();

        // Each IP gets its own limit
        for _ in 0..5 {
            assert!(limiter.allow_request(ip1));
            assert!(limiter.allow_request(ip2));
        }

        assert!(!limiter.allow_request(ip1));
        assert!(!limiter.allow_request(ip2));
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let limiter = StunRateLimiter::new(10);
        let ip = "192.168.1.1".parse().unwrap();

        // Make some requests
        for _ in 0..5 {
            limiter.allow_request(ip);
        }

        // Cleanup should work
        limiter.cleanup();

        // Should still be able to track requests
        assert!(limiter.allow_request(ip));
    }

    #[test]
    fn test_crc32_computation() {
        let data = b"hello world";
        let crc = StunMessage::crc32(data);
        // CRC-32 should be deterministic
        assert_eq!(crc, StunMessage::crc32(data));
    }

    #[test]
    fn test_username_attribute_encoding() {
        let username = StunAttribute::Username("testuser".to_string());
        let transaction_id = [0u8; 12];
        let encoded = username.encode(&transaction_id);

        // Should encode type, length, value, and padding
        assert!(encoded.len() >= 4 + 8); // 4 byte header + 8 byte value
    }

    #[test]
    fn test_message_integrity_attribute_encoding() {
        let hmac = [42u8; 20];
        let msg_integrity = StunAttribute::MessageIntegrity(hmac);
        let transaction_id = [0u8; 12];
        let encoded = msg_integrity.encode(&transaction_id);

        // Should be 24 bytes: 4 byte header + 20 byte HMAC
        assert_eq!(encoded.len(), 24);
    }

    #[test]
    fn test_stun_message_decode_error_too_short() {
        let short_msg = [0u8; 10]; // Less than HEADER_SIZE (20 bytes)
        let result = StunMessage::decode(&short_msg);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StunError::MessageTooShort));
    }

    #[test]
    fn test_stun_message_decode_error_invalid_magic_cookie() {
        let mut msg = vec![0u8; 20];
        // Set invalid magic cookie
        msg[4..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let result = StunMessage::decode(&msg);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StunError::InvalidMagicCookie));
    }

    #[test]
    fn test_stun_error_display() {
        let errors = vec![
            (StunError::Timeout, "STUN query timeout"),
            (StunError::MessageTooShort, "STUN message too short"),
            (StunError::InvalidMagicCookie, "Invalid STUN magic cookie"),
            (StunError::InvalidMessageType, "Invalid STUN message type"),
            (StunError::InvalidAttribute, "Invalid STUN attribute"),
            (StunError::TransactionMismatch, "Transaction ID mismatch"),
            (StunError::ErrorResponse, "STUN error response"),
            (
                StunError::MissingAttribute,
                "Missing required STUN attribute",
            ),
            (
                StunError::AuthenticationFailed,
                "MESSAGE-INTEGRITY authentication failed",
            ),
            (
                StunError::FingerprintMismatch,
                "FINGERPRINT verification failed",
            ),
            (StunError::RateLimitExceeded, "Rate limit exceeded"),
        ];

        for (err, expected_msg) in errors {
            assert_eq!(err.to_string(), expected_msg);
        }
    }

    #[test]
    fn test_stun_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
        let stun_err: StunError = io_err.into();
        assert!(matches!(stun_err, StunError::Io(_)));
    }

    #[test]
    fn test_stun_message_class_all_variants() {
        let classes = vec![
            StunMessageClass::Request,
            StunMessageClass::SuccessResponse,
            StunMessageClass::ErrorResponse,
            StunMessageClass::Indication,
        ];

        for class in classes {
            assert_eq!(class, class);
        }
    }

    #[test]
    fn test_fingerprint_attribute_encoding() {
        let fingerprint = StunAttribute::Fingerprint(0x12345678);
        let transaction_id = [0u8; 12];
        let encoded = fingerprint.encode(&transaction_id);

        // Should be 8 bytes: 4 byte header + 4 byte CRC
        assert_eq!(encoded.len(), 8);
    }

    #[test]
    fn test_software_attribute_encoding() {
        let software = StunAttribute::Software("WRAITH/1.0".to_string());
        let transaction_id = [0u8; 12];
        let encoded = software.encode(&transaction_id);

        // Should include header, value, and padding
        assert!(encoded.len() >= 4 + 10); // 4 byte header + 10 byte value
        assert_eq!(encoded.len() % 4, 0); // Should be 4-byte aligned
    }

    #[test]
    fn test_mapped_address_attribute() {
        let addr: SocketAddr = "192.0.2.1:32853".parse().unwrap();
        let transaction_id = [0u8; 12];

        let attr = StunAttribute::MappedAddress(addr);
        let encoded = attr.encode(&transaction_id);

        assert!(encoded.len() >= 4); // At least header size
    }

    #[test]
    fn test_unknown_attribute() {
        let unknown = StunAttribute::Unknown(0x9999, vec![1, 2, 3, 4]);
        assert_eq!(unknown.attr_type(), 0x9999);
    }

    #[test]
    fn test_crc32_different_data() {
        let data1 = b"hello";
        let data2 = b"world";
        let crc1 = StunMessage::crc32(data1);
        let crc2 = StunMessage::crc32(data2);

        // Different data should produce different CRCs
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_crc32_empty_data() {
        let data = b"";
        let crc = StunMessage::crc32(data);
        // CRC of empty data should be deterministic
        assert_eq!(crc, StunMessage::crc32(data));
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = StunRateLimiter::default();
        let ip = "192.168.1.1".parse().unwrap();

        // Default should allow 10 requests per second
        for _ in 0..10 {
            assert!(limiter.allow_request(ip));
        }
        assert!(!limiter.allow_request(ip));
    }

    #[test]
    fn test_authentication_zeroization() {
        // SECURITY: Intentional test credentials, not production secrets
        let auth = StunAuthentication::new("user", "password", None);
        // Password should be zeroized on drop (can't test directly, but ensure no panics)
        drop(auth);
    }

    #[test]
    fn test_authentication_key_derivation_short_term() {
        // SECURITY: Intentional test credentials, not production secrets
        let auth = StunAuthentication::new("user", "pass", None);
        let key = auth.derive_key();
        // Short-term credentials use password directly
        assert_eq!(&*key, b"pass");
    }

    #[test]
    fn test_authentication_key_derivation_long_term() {
        // SECURITY: Intentional test credentials, not production secrets
        let auth = StunAuthentication::new("user", "pass", Some("realm".to_string()));
        let key = auth.derive_key();
        // Long-term credentials use MD5(username:realm:password) as mandated by RFC 5389
        assert_eq!(key.len(), 16); // MD5 produces 16 bytes
    }
}
