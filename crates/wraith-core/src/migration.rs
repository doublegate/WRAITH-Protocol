//! Connection migration and path validation.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// State of a path being validated
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathState {
    /// Validation in progress
    Pending,
    /// Path has been validated
    Validated,
    /// Validation failed
    Failed,
}

/// Validated network path
#[derive(Clone, Debug)]
pub struct ValidatedPath {
    /// Path identifier (address + port)
    pub path_id: u64,
    /// Round-trip time measured during validation
    pub rtt: Duration,
    /// When path was validated
    pub validated_at: Instant,
}

/// Path validator for connection migration
pub struct PathValidator {
    /// Pending challenges: challenge_data -> (path_id, sent_at)
    pending_challenges: HashMap<[u8; 8], (u64, Instant)>,
    /// Validated paths
    validated_paths: Vec<ValidatedPath>,
    /// Challenge timeout
    timeout: Duration,
}

impl PathValidator {
    /// Create a new path validator
    ///
    /// # Arguments
    ///
    /// * `timeout` - Time to wait for challenge response before timing out
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending_challenges: HashMap::new(),
            validated_paths: Vec::new(),
            timeout,
        }
    }

    /// Generate challenge for new path
    ///
    /// Returns 8-byte challenge data to send to peer
    pub fn initiate_challenge(&mut self, path_id: u64) -> [u8; 8] {
        let mut challenge = [0u8; 8];
        getrandom::getrandom(&mut challenge).expect("getrandom failed");

        self.pending_challenges
            .insert(challenge, (path_id, Instant::now()));

        challenge
    }

    /// Handle incoming PATH_CHALLENGE, return response data
    ///
    /// When receiving a PATH_CHALLENGE frame, this generates the appropriate
    /// PATH_RESPONSE data to echo back to the peer.
    #[must_use]
    pub fn handle_challenge(&self, challenge: &[u8; 8]) -> [u8; 8] {
        // Echo back the challenge data
        *challenge
    }

    /// Handle PATH_RESPONSE, returns validated path if successful
    ///
    /// Called when receiving a PATH_RESPONSE frame. If the response matches
    /// a pending challenge, the path is validated and returned.
    pub fn handle_response(&mut self, response: &[u8; 8]) -> Option<ValidatedPath> {
        if let Some((path_id, sent_at)) = self.pending_challenges.remove(response) {
            let rtt = sent_at.elapsed();

            let validated_path = ValidatedPath {
                path_id,
                rtt,
                validated_at: Instant::now(),
            };

            self.validated_paths.push(validated_path.clone());

            Some(validated_path)
        } else {
            None
        }
    }

    /// Clean up expired challenges
    ///
    /// Removes challenges that have exceeded the timeout without receiving
    /// a response.
    pub fn cleanup_expired(&mut self) {
        let timeout = self.timeout;
        self.pending_challenges
            .retain(|_, (_, sent_at)| sent_at.elapsed() < timeout);
    }

    /// Get validated paths
    #[must_use]
    pub fn validated_paths(&self) -> &[ValidatedPath] {
        &self.validated_paths
    }

    /// Get number of pending challenges
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_challenges.len()
    }

    /// Check if a specific path is validated
    #[must_use]
    pub fn is_path_validated(&self, path_id: u64) -> bool {
        self.validated_paths.iter().any(|p| p.path_id == path_id)
    }
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new(Duration::from_secs(3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_path_validator_new() {
        let timeout = Duration::from_secs(5);
        let validator = PathValidator::new(timeout);

        assert_eq!(validator.pending_count(), 0);
        assert_eq!(validator.validated_paths().len(), 0);
        assert_eq!(validator.timeout, timeout);
    }

    #[test]
    fn test_initiate_challenge() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let challenge1 = validator.initiate_challenge(1);
        let challenge2 = validator.initiate_challenge(2);

        // Challenges should be unique
        assert_ne!(challenge1, challenge2);

        // Should have two pending challenges
        assert_eq!(validator.pending_count(), 2);
    }

    #[test]
    fn test_handle_challenge() {
        let validator = PathValidator::new(Duration::from_secs(3));

        let challenge = [1, 2, 3, 4, 5, 6, 7, 8];
        let response = validator.handle_challenge(&challenge);

        // Response should echo the challenge
        assert_eq!(response, challenge);
    }

    #[test]
    fn test_handle_response_success() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let path_id = 42;
        let challenge = validator.initiate_challenge(path_id);

        thread::sleep(Duration::from_millis(10));

        // Handle response with matching challenge
        let validated = validator.handle_response(&challenge);

        assert!(validated.is_some());

        let path = validated.unwrap();
        assert_eq!(path.path_id, path_id);
        assert!(path.rtt.as_millis() >= 10);

        // Should be in validated paths
        assert_eq!(validator.validated_paths().len(), 1);
        assert!(validator.is_path_validated(path_id));

        // Should no longer be pending
        assert_eq!(validator.pending_count(), 0);
    }

    #[test]
    fn test_handle_response_unknown() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let unknown_challenge = [9, 9, 9, 9, 9, 9, 9, 9];
        let validated = validator.handle_response(&unknown_challenge);

        // Should not validate unknown challenge
        assert!(validated.is_none());
        assert_eq!(validator.validated_paths().len(), 0);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut validator = PathValidator::new(Duration::from_millis(50));

        validator.initiate_challenge(1);
        validator.initiate_challenge(2);

        assert_eq!(validator.pending_count(), 2);

        // Wait for timeout
        thread::sleep(Duration::from_millis(100));

        validator.cleanup_expired();

        // All challenges should be expired
        assert_eq!(validator.pending_count(), 0);
    }

    #[test]
    fn test_cleanup_expired_keeps_recent() {
        let mut validator = PathValidator::new(Duration::from_secs(5));

        validator.initiate_challenge(1);

        thread::sleep(Duration::from_millis(10));

        validator.cleanup_expired();

        // Recent challenge should not be expired
        assert_eq!(validator.pending_count(), 1);
    }

    #[test]
    fn test_validated_paths() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let challenge1 = validator.initiate_challenge(10);
        let challenge2 = validator.initiate_challenge(20);

        validator.handle_response(&challenge1);
        validator.handle_response(&challenge2);

        let paths = validator.validated_paths();
        assert_eq!(paths.len(), 2);

        // Check path IDs
        let path_ids: Vec<u64> = paths.iter().map(|p| p.path_id).collect();
        assert!(path_ids.contains(&10));
        assert!(path_ids.contains(&20));
    }

    #[test]
    fn test_is_path_validated() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let path_id = 100;
        let challenge = validator.initiate_challenge(path_id);

        assert!(!validator.is_path_validated(path_id));

        validator.handle_response(&challenge);

        assert!(validator.is_path_validated(path_id));
        assert!(!validator.is_path_validated(999));
    }

    #[test]
    fn test_multiple_challenges_same_path() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let path_id = 5;
        let challenge1 = validator.initiate_challenge(path_id);
        let challenge2 = validator.initiate_challenge(path_id);

        // Both challenges should be different
        assert_ne!(challenge1, challenge2);

        // Both should be pending
        assert_eq!(validator.pending_count(), 2);

        // Respond to first challenge
        validator.handle_response(&challenge1);

        assert_eq!(validator.validated_paths().len(), 1);
        assert_eq!(validator.pending_count(), 1);

        // Respond to second challenge
        validator.handle_response(&challenge2);

        // Should have two validated entries for same path
        assert_eq!(validator.validated_paths().len(), 2);
        assert_eq!(validator.pending_count(), 0);
    }

    #[test]
    fn test_path_state_enum() {
        assert_eq!(PathState::Pending, PathState::Pending);
        assert_ne!(PathState::Pending, PathState::Validated);
        assert_ne!(PathState::Validated, PathState::Failed);
    }

    #[test]
    fn test_validated_path_clone() {
        let path = ValidatedPath {
            path_id: 42,
            rtt: Duration::from_millis(50),
            validated_at: Instant::now(),
        };

        let cloned = path.clone();

        assert_eq!(path.path_id, cloned.path_id);
        assert_eq!(path.rtt, cloned.rtt);
    }

    #[test]
    fn test_default_validator() {
        let validator = PathValidator::default();

        assert_eq!(validator.timeout, Duration::from_secs(3));
        assert_eq!(validator.pending_count(), 0);
    }

    #[test]
    fn test_rtt_measurement() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let challenge = validator.initiate_challenge(1);

        let delay = Duration::from_millis(100);
        thread::sleep(delay);

        let validated = validator.handle_response(&challenge).unwrap();

        // RTT should be at least the delay
        assert!(validated.rtt >= delay);
        // RTT should be reasonable (less than 1s for CI environments which can be slow)
        assert!(validated.rtt < Duration::from_secs(1));
    }

    #[test]
    fn test_challenge_uniqueness() {
        let mut validator = PathValidator::new(Duration::from_secs(3));

        let mut challenges = Vec::new();
        for i in 0..100 {
            challenges.push(validator.initiate_challenge(i));
        }

        // All challenges should be unique
        for i in 0..challenges.len() {
            for j in (i + 1)..challenges.len() {
                assert_ne!(challenges[i], challenges[j]);
            }
        }
    }
}
