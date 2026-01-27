#[cfg(test)]
mod tests {
    use spectre_implant::modules::smb::SmbClient;
    use spectre_implant::modules::smb::SMB2Header;
    use core::mem::size_of;

    #[test]
    fn test_smb_struct_sizes() {
        assert_eq!(size_of::<SMB2Header>(), 64);
    }

    #[test]
    fn test_smb_client_init() {
        let client = SmbClient::new();
        assert_eq!(client.process_id, 0xFEFF);
    }
}
