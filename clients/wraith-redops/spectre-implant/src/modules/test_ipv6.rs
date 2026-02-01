#[cfg(test)]
mod tests {
    use crate::modules::socks::SocksProxy;

    #[test]
    fn test_socks5_request_ipv6_success() {
        let mut proxy = SocksProxy::new();
        proxy.process(&[0x05, 0x01, 0x00]); // Handshake

        // SOCKS5 Request: CMD=CONNECT, ATYP=0x04 (IPv6), ADDR=::1, PORT=80
        let mut request = [0u8; 4 + 16 + 2];
        request[0] = 0x05;
        request[1] = 0x01;
        request[2] = 0x00;
        request[3] = 0x04; // ATYP IPv6
        request[19] = 1; // Last byte of ::1
        request[20] = 0; // Port 80 high
        request[21] = 80; // Port 80 low

        let response = proxy.process(&request);
        
        // Should return 0x00 (Success) or 0x04 (Host unreachable) if socket fails
        // but it should NOT return 0x08 (Address type not supported)
        assert_ne!(response[1], 0x08);
    }
}