use alloc::vec::Vec;
use alloc::string::String;

pub enum SocksState {
    Greeting,
    Auth,
    Request,
    Forwarding,
    Error,
}

pub struct SocksProxy {
    state: SocksState,
}

impl SocksProxy {
    pub fn new() -> Self {
        Self { state: SocksState::Greeting }
    }

    /// Processes incoming data from the SOCKS client and returns the response bytes.
    /// Manages the internal state machine for the SOCKS handshake.
    pub fn process(&mut self, data: &[u8]) -> Vec<u8> {
        match self.state {
            SocksState::Greeting => self.handle_greeting(data),
            SocksState::Auth => self.handle_auth(data),
            SocksState::Request => self.handle_request(data),
            SocksState::Forwarding => data.to_vec(), // In real use, this is relayed to target
            SocksState::Error => Vec::new(),
        }
    }

    fn handle_greeting(&mut self, data: &[u8]) -> Vec<u8> {
        if data.len() < 2 {
            return Vec::new();
        }

        match data[0] {
            0x05 => { // SOCKS5
                let n_methods = data[1] as usize;
                if data.len() < 2 + n_methods {
                    return Vec::new();
                }
                
                // Select 0x00 (No Authentication Required)
                let methods = &data[2..2 + n_methods];
                if methods.contains(&0x00) {
                    self.state = SocksState::Request;
                    Vec::from([0x05, 0x00])
                } else {
                    // 0xFF = No acceptable methods
                    self.state = SocksState::Error;
                    Vec::from([0x05, 0xFF])
                }
            },
            0x04 => { // SOCKS4
                // SOCKS4 doesn't have a greeting, it goes straight to Request
                self.state = SocksState::Request;
                self.handle_request(data)
            },
            _ => {
                self.state = SocksState::Error;
                Vec::new()
            }
        }
    }

    pub fn handle_auth(&self, _data: &[u8]) -> Vec<u8> {
        // Placeholder for future auth methods (e.g. User/Pass)
        // Currently we only support 'No Auth'
        Vec::new()
    }

    pub fn handle_request(&mut self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() { return Vec::new(); }

        match data[0] {
            0x05 => { // SOCKS5 Request
                if data.len() < 7 { return Vec::new(); }
                let cmd = data[1];
                if cmd != 0x01 { // CONNECT
                    return Vec::from([0x05, 0x07, 0x00, 0x01, 0,0,0,0, 0,0]);
                }

                let atyp = data[3];
                match atyp {
                    0x01 => { // IPv4
                        if data.len() < 10 { return Vec::new(); }
                        // IP: data[4..8], Port: data[8..10]
                    },
                    0x03 => { // Domain Name
                        let domain_len = data[4] as usize;
                        if data.len() < 5 + domain_len + 2 { return Vec::new(); }
                        // Domain: data[5..5+domain_len]
                    },
                    0x04 => { // IPv6
                        if data.len() < 22 { return Vec::new(); }
                    },
                    _ => {
                        return Vec::from([0x05, 0x08, 0x00, 0x01, 0,0,0,0, 0,0]);
                    }
                }

                // Simulate successful connection for the state machine
                self.state = SocksState::Forwarding;
                // Reply: Success (00), RSVD (00), ATYP (01), IP (0,0,0,0), Port (0,0)
                Vec::from([0x05, 0x00, 0x00, 0x01, 0,0,0,0, 0,0])
            },
            0x04 => { // SOCKS4 Request
                if data.len() < 8 { return Vec::new(); }
                let cmd = data[1];
                if cmd != 0x01 {
                    return Vec::from([0x00, 0x5B, 0,0, 0,0,0,0]); // Request rejected
                }
                
                // Port: data[2..4], IP: data[4..8]
                self.state = SocksState::Forwarding;
                Vec::from([0x00, 0x5A, 0,0, 0,0,0,0]) // Request granted
            },
            _ => {
                self.state = SocksState::Error;
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socks5_greeting() {
        let mut proxy = SocksProxy::new();
        let greeting = [0x05, 0x01, 0x00];
        let response = proxy.process(&greeting);
        assert_eq!(response, [0x05, 0x00]);
    }

    #[test]
    fn test_socks5_connect_ipv4() {
        let mut proxy = SocksProxy::new();
        proxy.process(&[0x05, 0x01, 0x00]); // Handshake
        
        let request = [0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0, 80];
        let response = proxy.process(&request);
        assert_eq!(response, [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    }
}
