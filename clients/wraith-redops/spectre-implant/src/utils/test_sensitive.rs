#[cfg(test)]
mod tests {
    use crate::utils::sensitive::SensitiveData;

    #[test]
    fn test_sensitive_round_trip() {
        let secret = b"secret password";
        let wrapper = SensitiveData::new(secret);

        let guard = wrapper.unlock().expect("Failed to unlock");
        assert_eq!(&guard[..], secret);
    }
}
