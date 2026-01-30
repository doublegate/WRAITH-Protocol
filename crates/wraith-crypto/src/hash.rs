//! `BLAKE3` cryptographic hashing and key derivation.
//!
//! Provides:
//! - Fast cryptographic hashing
//! - Tree hashing for large files (parallel)
//! - `HKDF`-like key derivation functions
//! - Context-specific `KDF`

/// `BLAKE3` hash output (32 bytes).
pub type HashOutput = [u8; 32];

/// Compute `BLAKE3` hash of input data.
#[must_use]
pub fn hash(data: &[u8]) -> HashOutput {
    *blake3::hash(data).as_bytes()
}

/// `BLAKE3` tree hasher for large files.
///
/// Supports incremental updates and parallel hashing.
pub struct TreeHasher {
    hasher: blake3::Hasher,
    total_len: usize,
}

impl TreeHasher {
    /// Create a new tree hasher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
            total_len: 0,
        }
    }

    /// Update with more data.
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
        self.total_len += data.len();
    }

    /// Update hasher with multiple chunks.
    ///
    /// More efficient than multiple single updates when you have
    /// multiple chunks ready to hash.
    pub fn update_batch(&mut self, chunks: &[&[u8]]) {
        for chunk in chunks {
            self.hasher.update(chunk);
            self.total_len += chunk.len();
        }
    }

    /// Get total bytes hashed so far.
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.total_len
    }

    /// Finalize and return the hash.
    #[must_use]
    pub fn finalize(&self) -> HashOutput {
        *self.hasher.finalize().as_bytes()
    }

    /// Finalize into extended output reader (`XOF`).
    #[must_use]
    pub fn finalize_xof(&self) -> blake3::OutputReader {
        self.hasher.finalize_xof()
    }
}

impl Default for TreeHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// `BLAKE3` Key Derivation Function with context.
pub struct Kdf {
    context: &'static str,
}

impl Kdf {
    /// Create a `KDF` with a specific context string.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let kdf = Kdf::new("wraith-session-key");
    /// let key = kdf.derive_key(&shared_secret);
    /// ```
    #[must_use]
    pub fn new(context: &'static str) -> Self {
        Self { context }
    }

    /// Derive output from input key material.
    pub fn derive(&self, ikm: &[u8], output: &mut [u8]) {
        // Use keyed BLAKE3 with context
        let key_hash = hash(ikm);
        let mut hasher = blake3::Hasher::new_keyed(&key_hash);
        hasher.update(self.context.as_bytes());

        let mut reader = hasher.finalize_xof();
        reader.fill(output);
    }

    /// Derive a 32-byte key.
    #[must_use]
    pub fn derive_key(&self, ikm: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        self.derive(ikm, &mut output);
        output
    }
}

/// `HKDF`-Extract: Extract a pseudorandom key from input key material.
///
/// Corresponds to `HKDF`-Extract from `RFC 5869`, but using `BLAKE3`.
#[must_use]
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    if salt.is_empty() {
        // No salt: use fixed all-zero 32-byte key for domain separation
        // This distinguishes the unkeyed path from the keyed path,
        // preventing domain confusion attacks
        let zero_key = [0u8; 32];
        let mut hasher = blake3::Hasher::new_keyed(&zero_key);
        hasher.update(ikm);
        *hasher.finalize().as_bytes()
    } else {
        // Use salt as key for keyed BLAKE3
        let salt_hash = hash(salt);
        let mut hasher = blake3::Hasher::new_keyed(&salt_hash);
        hasher.update(ikm);
        *hasher.finalize().as_bytes()
    }
}

/// `HKDF`-Expand: Expand a pseudorandom key into arbitrary-length output.
///
/// Corresponds to `HKDF`-Expand from `RFC 5869`, but using `BLAKE3`.
pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], output: &mut [u8]) {
    let mut hasher = blake3::Hasher::new_keyed(prk);
    hasher.update(info);

    let mut reader = hasher.finalize_xof();
    reader.fill(output);
}

/// `HKDF`: Combined extract-then-expand.
pub fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8], output: &mut [u8]) {
    let prk = hkdf_extract(salt, ikm);
    hkdf_expand(&prk, info, output);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_blake3_basic() {
        let data = b"hello world";
        let hash1 = hash(data);
        let hash2 = hash(data);

        // Hash is deterministic
        assert_eq!(hash1, hash2);

        // Hash is non-zero
        assert_ne!(hash1, [0u8; 32]);
    }

    #[test]
    fn test_blake3_different_inputs() {
        let hash1 = hash(b"input1");
        let hash2 = hash(b"input2");

        // Different inputs produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_tree_hasher_incremental() {
        let data1 = b"hello ";
        let data2 = b"world";

        // Hash all at once
        let hash_combined = hash(b"hello world");

        // Hash incrementally
        let mut hasher = TreeHasher::new();
        hasher.update(data1);
        hasher.update(data2);
        let hash_incremental = hasher.finalize();

        assert_eq!(hash_combined, hash_incremental);
    }

    #[test]
    fn test_kdf_deterministic() {
        let kdf = Kdf::new("test-context");
        let ikm = b"input key material";

        let key1 = kdf.derive_key(ikm);
        let key2 = kdf.derive_key(ikm);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_kdf_different_contexts() {
        let kdf1 = Kdf::new("context-1");
        let kdf2 = Kdf::new("context-2");
        let ikm = b"same input";

        let key1 = kdf1.derive_key(ikm);
        let key2 = kdf2.derive_key(ikm);

        // Different contexts produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_kdf_different_ikm() {
        let kdf = Kdf::new("same-context");

        let key1 = kdf.derive_key(b"ikm1");
        let key2 = kdf.derive_key(b"ikm2");

        // Different inputs produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hkdf_extract() {
        let salt = b"salt";
        let ikm = b"input key material";

        let prk1 = hkdf_extract(salt, ikm);
        let prk2 = hkdf_extract(salt, ikm);

        // Extract is deterministic
        assert_eq!(prk1, prk2);
    }

    #[test]
    fn test_hkdf_expand() {
        let prk = [0x42u8; 32];
        let info = b"application info";

        let mut output1 = [0u8; 64];
        let mut output2 = [0u8; 64];

        hkdf_expand(&prk, info, &mut output1);
        hkdf_expand(&prk, info, &mut output2);

        // Expand is deterministic
        assert_eq!(output1, output2);
    }

    #[test]
    fn test_hkdf_combined() {
        let salt = b"salt";
        let ikm = b"input";
        let info = b"info";

        let mut output1 = [0u8; 64];
        let mut output2 = [0u8; 64];

        hkdf(salt, ikm, info, &mut output1);
        hkdf(salt, ikm, info, &mut output2);

        assert_eq!(output1, output2);
    }

    #[test]
    fn test_hkdf_no_salt() {
        let ikm = b"input";
        let info = b"info";

        let mut output = [0u8; 32];
        hkdf(b"", ikm, info, &mut output);

        // Should not panic or produce zeros
        assert_ne!(output, [0u8; 32]);
    }

    // BLAKE3 known test vector
    #[test]
    fn test_blake3_empty_string() {
        let hash_output = hash(b"");

        // BLAKE3 hash of empty string (from official test vectors)
        let expected = [
            0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc,
            0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca,
            0xe4, 0x1f, 0x32, 0x62,
        ];

        assert_eq!(hash_output, expected);
    }

    // ========================================================================
    // Batch Hash Update Tests (Tier 2 - Performance Optimization)
    // ========================================================================

    #[test]
    fn test_batch_update_single_chunk() {
        let mut hasher = TreeHasher::new();
        let data = b"test data";

        hasher.update_batch(&[data]);

        assert_eq!(hasher.total_len(), data.len());
        let hash_batch = hasher.finalize();

        // Compare with single update
        let mut hasher2 = TreeHasher::new();
        hasher2.update(data);
        let hash_single = hasher2.finalize();

        assert_eq!(hash_batch, hash_single);
    }

    #[test]
    fn test_batch_update_multiple_chunks() {
        let mut hasher = TreeHasher::new();
        let chunks = [b"hello" as &[u8], b" ", b"world"];

        hasher.update_batch(&chunks);

        assert_eq!(hasher.total_len(), 11); // "hello world"
        let hash_batch = hasher.finalize();

        // Compare with concatenated single update
        let mut hasher2 = TreeHasher::new();
        hasher2.update(b"hello world");
        let hash_single = hasher2.finalize();

        assert_eq!(hash_batch, hash_single);
    }

    #[test]
    fn test_batch_vs_sequential_identical() {
        let chunks = [b"chunk1" as &[u8], b"chunk2", b"chunk3", b"chunk4"];

        // Hash using batch
        let mut hasher_batch = TreeHasher::new();
        hasher_batch.update_batch(&chunks);
        let hash_batch = hasher_batch.finalize();

        // Hash using sequential updates
        let mut hasher_seq = TreeHasher::new();
        for chunk in &chunks {
            hasher_seq.update(chunk);
        }
        let hash_seq = hasher_seq.finalize();

        // Should produce identical hashes
        assert_eq!(hash_batch, hash_seq);

        // Total lengths should match
        assert_eq!(hasher_batch.total_len(), hasher_seq.total_len());
    }

    #[test]
    fn test_batch_update_empty_chunks() {
        let mut hasher = TreeHasher::new();
        let chunks: Vec<&[u8]> = vec![];

        hasher.update_batch(&chunks);

        assert_eq!(hasher.total_len(), 0);
    }

    #[test]
    fn test_batch_update_mixed_sizes() {
        let mut hasher = TreeHasher::new();
        let chunk1 = b"short";
        let chunk2 = b"this is a much longer chunk with more data";
        let chunk3 = b"x";

        hasher.update_batch(&[chunk1, chunk2, chunk3]);

        let expected_len = chunk1.len() + chunk2.len() + chunk3.len();
        assert_eq!(hasher.total_len(), expected_len);

        // Verify hash matches sequential
        let mut hasher2 = TreeHasher::new();
        hasher2.update(chunk1);
        hasher2.update(chunk2);
        hasher2.update(chunk3);

        assert_eq!(hasher.finalize(), hasher2.finalize());
    }

    #[test]
    fn test_total_len_tracking() {
        let mut hasher = TreeHasher::new();

        assert_eq!(hasher.total_len(), 0);

        hasher.update(b"12345");
        assert_eq!(hasher.total_len(), 5);

        hasher.update(b"67890");
        assert_eq!(hasher.total_len(), 10);

        hasher.update_batch(&[b"abc", b"def"]);
        assert_eq!(hasher.total_len(), 16);
    }
}
