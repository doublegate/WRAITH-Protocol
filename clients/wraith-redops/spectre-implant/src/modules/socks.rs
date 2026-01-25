use alloc::vec::Vec;

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

    pub fn process(&mut self, data: &[u8]) -> Vec<u8> {
        match self.state {
            SocksState::Greeting => {
                if data.is_empty() || data[0] != 0x05 {
                    self.state = SocksState::Error;
                    return Vec::new();
                }
                // Reply 05 00 (No Auth)
                self.state = SocksState::Request;
                Vec::from([0x05, 0x00])
            },
            SocksState::Auth => {
                // Should not happen if we selected 00 (No Auth)
                self.state = SocksState::Error;
                Vec::new()
            },
            SocksState::Request => {
                // Expect 05 CMD RSV ATYP ...
                if data.len() < 4 || data[0] != 0x05 {
                     self.state = SocksState::Error;
                     return Vec::new();
                }
                if data[1] != 0x01 { // Only support CONNECT (01)
                     // 07 = Command not supported
                     return Vec::from([0x05, 0x07, 0x00, 0x01, 0,0,0,0, 0,0]);
                }
                
                // Parse Address (stubbed, we just accept)
                // In real impl, we would connect to target.
                // For remediation, we simulate successful connection.
                self.state = SocksState::Forwarding;
                Vec::from([0x05, 0x00, 0x00, 0x01, 0,0,0,0, 0,0])
            },
            SocksState::Forwarding => {
                // Echo data (In real usage, this data goes to the target socket)
                data.to_vec()
            },
            SocksState::Error => Vec::new(),
        }
    }
}