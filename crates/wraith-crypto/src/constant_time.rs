//! Constant-time cryptographic operations.
//!
//! Provides timing-safe operations to prevent side-channel attacks.
//! All comparisons and selections are constant-time with respect to
//! secret data.

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

/// Constant-time comparison of byte slices.
///
/// Returns `true` if slices are equal, `false` otherwise.
/// Execution time depends only on slice length, not content.
#[must_use]
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.ct_eq(b).into()
}

/// Constant-time conditional copy.
///
/// If `condition` is `true`, copies `a` to `out`.
/// If `condition` is `false`, copies `b` to `out`.
///
/// # Panics
///
/// Panics if slice lengths don't match.
pub fn ct_select(condition: bool, a: &[u8], b: &[u8], out: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), out.len());

    let choice = Choice::from(u8::from(condition));

    for i in 0..out.len() {
        out[i] = u8::conditional_select(&b[i], &a[i], choice);
    }
}

/// Timing-safe 16-byte array comparison.
///
/// # Example
///
/// ```ignore
/// let tag1: [u8; 16] = /* ... */;
/// let tag2: [u8; 16] = /* ... */;
///
/// if verify_16(&tag1, &tag2) {
///     // Tags match
/// }
/// ```
#[must_use]
#[inline(never)]
pub fn verify_16(a: &[u8; 16], b: &[u8; 16]) -> bool {
    ct_eq(a, b)
}

/// Timing-safe 32-byte array comparison.
#[must_use]
#[inline(never)]
pub fn verify_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    ct_eq(a, b)
}

/// Timing-safe 64-byte array comparison.
#[must_use]
#[inline(never)]
pub fn verify_64(a: &[u8; 64], b: &[u8; 64]) -> bool {
    ct_eq(a, b)
}

/// Constant-time conditional assignment.
///
/// If `condition` is `true`, assigns `value` to `target`.
/// If `condition` is `false`, `target` remains unchanged.
///
/// # Panics
///
/// Panics if `target.len()` != `value.len()`.
pub fn ct_assign(condition: bool, target: &mut [u8], value: &[u8]) {
    assert_eq!(target.len(), value.len());

    let choice = Choice::from(u8::from(condition));

    for i in 0..target.len() {
        target[i] = u8::conditional_select(&target[i], &value[i], choice);
    }
}

/// Constant-time byte-wise OR.
///
/// Computes `out[i] = a[i] | b[i]` for all i, in constant time.
///
/// # Panics
///
/// Panics if `a.len()` != `b.len()` or `a.len()` != `out.len()`.
pub fn ct_or(a: &[u8], b: &[u8], out: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), out.len());

    for i in 0..out.len() {
        out[i] = a[i] | b[i];
    }
}

/// Constant-time byte-wise AND.
///
/// Computes `out[i] = a[i] & b[i]` for all i, in constant time.
///
/// # Panics
///
/// Panics if `a.len()` != `b.len()` or `a.len()` != `out.len()`.
pub fn ct_and(a: &[u8], b: &[u8], out: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), out.len());

    for i in 0..out.len() {
        out[i] = a[i] & b[i];
    }
}

/// Constant-time byte-wise XOR.
///
/// Computes `out[i] = a[i] ^ b[i]` for all i, in constant time.
///
/// # Panics
///
/// Panics if `a.len()` != `b.len()` or `a.len()` != `out.len()`.
pub fn ct_xor(a: &[u8], b: &[u8], out: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), out.len());

    for i in 0..out.len() {
        out[i] = a[i] ^ b[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq_same() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        assert!(ct_eq(&a, &b));
    }

    #[test]
    fn test_ct_eq_different() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn test_ct_eq_different_lengths() {
        let a = [1u8; 32];
        let b = [1u8; 16];
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn test_ct_select_true() {
        let a = [1u8; 8];
        let b = [2u8; 8];
        let mut out = [0u8; 8];

        ct_select(true, &a, &b, &mut out);
        assert_eq!(out, a);
    }

    #[test]
    fn test_ct_select_false() {
        let a = [1u8; 8];
        let b = [2u8; 8];
        let mut out = [0u8; 8];

        ct_select(false, &a, &b, &mut out);
        assert_eq!(out, b);
    }

    #[test]
    fn test_verify_16() {
        let a = [0x42u8; 16];
        let b = [0x42u8; 16];
        let c = [0x43u8; 16];

        assert!(verify_16(&a, &b));
        assert!(!verify_16(&a, &c));
    }

    #[test]
    fn test_verify_32() {
        let a = [0x42u8; 32];
        let b = [0x42u8; 32];
        let c = [0x43u8; 32];

        assert!(verify_32(&a, &b));
        assert!(!verify_32(&a, &c));
    }

    #[test]
    fn test_verify_64() {
        let a = [0x42u8; 64];
        let b = [0x42u8; 64];
        let c = [0x43u8; 64];

        assert!(verify_64(&a, &b));
        assert!(!verify_64(&a, &c));
    }

    #[test]
    fn test_ct_assign_true() {
        let mut target = [0u8; 8];
        let value = [0x42u8; 8];

        ct_assign(true, &mut target, &value);
        assert_eq!(target, value);
    }

    #[test]
    fn test_ct_assign_false() {
        let mut target = [0u8; 8];
        let original = target;
        let value = [0x42u8; 8];

        ct_assign(false, &mut target, &value);
        assert_eq!(target, original);
    }

    #[test]
    fn test_ct_or() {
        let a = [0b10101010u8; 4];
        let b = [0b01010101u8; 4];
        let mut out = [0u8; 4];

        ct_or(&a, &b, &mut out);
        assert_eq!(out, [0b11111111u8; 4]);
    }

    #[test]
    fn test_ct_and() {
        let a = [0b11110000u8; 4];
        let b = [0b10101010u8; 4];
        let mut out = [0u8; 4];

        ct_and(&a, &b, &mut out);
        assert_eq!(out, [0b10100000u8; 4]);
    }

    #[test]
    fn test_ct_xor() {
        let a = [0b11110000u8; 4];
        let b = [0b10101010u8; 4];
        let mut out = [0u8; 4];

        ct_xor(&a, &b, &mut out);
        assert_eq!(out, [0b01011010u8; 4]);
    }

    #[test]
    fn test_ct_xor_self_cancels() {
        let a = [0x42u8; 8];
        let mut out = [0u8; 8];

        ct_xor(&a, &a, &mut out);
        assert_eq!(out, [0u8; 8]);
    }
}
