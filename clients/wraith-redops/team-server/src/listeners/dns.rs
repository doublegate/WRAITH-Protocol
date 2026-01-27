use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::services::protocol::ProtocolHandler;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use wraith_crypto::noise::NoiseKeypair;

// --- DNS Protocol Implementation ---

#[derive(Debug, Default)]
struct DnsHeader {
    id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

#[derive(Debug, Clone)]
struct DnsQuestion {
    name: String,
    qtype: u16,
    qclass: u16,
}

#[derive(Debug, Clone)]
struct DnsResource {
    name: String,
    rtype: u16,
    rclass: u16,
    ttl: u32,
    rdata: Vec<u8>,
}

struct DnsPacket {
    header: DnsHeader,
    questions: Vec<DnsQuestion>,
    answers: Vec<DnsResource>,
}

impl DnsPacket {
    fn new() -> Self {
        Self {
            header: DnsHeader::default(),
            questions: Vec::new(),
            answers: Vec::new(),
        }
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, String> {
        if buf.len() < 12 {
            return Err("Packet too short".to_string());
        }

        let header = DnsHeader {
            id: u16::from_be_bytes([buf[0], buf[1]]),
            flags: u16::from_be_bytes([buf[2], buf[3]]),
            qdcount: u16::from_be_bytes([buf[4], buf[5]]),
            ancount: u16::from_be_bytes([buf[6], buf[7]]),
            nscount: u16::from_be_bytes([buf[8], buf[9]]),
            arcount: u16::from_be_bytes([buf[10], buf[11]]),
        };

        let mut pos = 12;
        let mut questions = Vec::new();
        for _ in 0..header.qdcount {
            let (name, next_pos) = parse_name(buf, pos)?;
            pos = next_pos;
            if pos + 4 > buf.len() {
                return Err("Insufficient data for question".to_string());
            }
            let qtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            let qclass = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]);
            pos += 4;
            questions.push(DnsQuestion { name, qtype, qclass });
        }

        Ok(DnsPacket {
            header,
            questions,
            answers: Vec::new(),
        })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.header.id.to_be_bytes());
        buf.extend_from_slice(&self.header.flags.to_be_bytes());
        buf.extend_from_slice(&self.header.qdcount.to_be_bytes());
        buf.extend_from_slice(&self.header.ancount.to_be_bytes());
        buf.extend_from_slice(&self.header.nscount.to_be_bytes());
        buf.extend_from_slice(&self.header.arcount.to_be_bytes());

        for q in &self.questions {
            encode_name(&mut buf, &q.name);
            buf.extend_from_slice(&q.qtype.to_be_bytes());
            buf.extend_from_slice(&q.qclass.to_be_bytes());
        }

        for a in &self.answers {
            encode_name(&mut buf, &a.name);
            buf.extend_from_slice(&a.rtype.to_be_bytes());
            buf.extend_from_slice(&a.rclass.to_be_bytes());
            buf.extend_from_slice(&a.ttl.to_be_bytes());
            buf.extend_from_slice(&(a.rdata.len() as u16).to_be_bytes());
            buf.extend_from_slice(&a.rdata);
        }

        buf
    }

    fn add_answer(&mut self, name: &str, rtype: u16, ttl: u32, rdata: &[u8]) {
        self.answers.push(DnsResource {
            name: name.to_string(),
            rtype,
            rclass: 1, // IN
            ttl,
            rdata: rdata.to_vec(),
        });
        self.header.ancount += 1;
    }

    fn make_response(&self) -> Self {
        let mut resp = DnsPacket::new();
        resp.header.id = self.header.id;
        resp.header.flags = 0x8180; // Standard response, recursive, no error
        resp.header.qdcount = self.header.qdcount;
        resp.questions = self.questions.clone();
        resp
    }
}

fn parse_name(buf: &[u8], mut pos: usize) -> Result<(String, usize), String> {
    let mut name = String::new();
    let mut jumped = false;
    let mut jump_pos = 0;
    let mut loops = 0;

    loop {
        if loops > 5 {
            return Err("Too many jumps in DNS name".to_string());
        }
        if pos >= buf.len() {
            return Err("Unexpected end of buffer parsing name".to_string());
        }
        let len = buf[pos];
        if len & 0xC0 == 0xC0 {
            if pos + 2 > buf.len() {
                return Err("Invalid jump pointer".to_string());
            }
            let offset = (((len & 0x3F) as u16) << 8 | buf[pos + 1] as u16) as usize;
            if !jumped {
                jump_pos = pos + 2;
                jumped = true;
            }
            pos = offset;
            loops += 1;
            continue;
        }

        pos += 1;
        if len == 0 {
            break;
        }

        if !name.is_empty() {
            name.push('.');
        }
        if pos + len as usize > buf.len() {
            return Err("Label length exceeds buffer".to_string());
        }
        name.push_str(&String::from_utf8_lossy(&buf[pos..pos + len as usize]));
        pos += len as usize;
    }

    let final_pos = if jumped { jump_pos } else { pos };
    Ok((name, final_pos))
}

fn encode_name(buf: &mut Vec<u8>, name: &str) {
    for label in name.split('.') {
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0);
}

// --- Listener ---

pub async fn start_dns_listener(
    db: Arc<Database>,
    port: u16,
    event_tx: broadcast::Sender<Event>,
    governance: Arc<GovernanceEngine>,
    static_key: NoiseKeypair,
    session_manager: Arc<SessionManager>,
) {
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("DNS Listener starting on {}", addr);

    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to bind DNS socket: {}", e);
            return;
        }
    };

    let protocol = ProtocolHandler::new(db, session_manager, Arc::new(static_key), event_tx);
    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                if !governance.validate_action(src.ip()) {
                    continue;
                }

                let packet = match DnsPacket::from_bytes(&buf[..len]) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("Failed to parse DNS packet from {}: {}", src, e);
                        continue;
                    }
                };

                for question in &packet.questions {
                    if !governance.validate_domain(&question.name) {
                        continue;
                    }

                    // Extract data from subdomains: <label1>.<label2>...<session_id>.domain.com
                    // Multi-label encoding: concatenate labels until we hit the session ID or base domain
                    let parts: Vec<&str> = question.name.split('.').collect();
                    if parts.len() < 3 {
                        continue;
                    }

                    let mut response_packet = packet.make_response();

                    if question.qtype == 16 { // TXT
                        // Multi-label payload extraction
                        // Assume the last 2 parts are the domain (e.g., domain.com)
                        // The part before that is the session ID.
                        // All parts before that are the chunked payload.
                        let mut payload_hex = String::new();
                        for i in 0..parts.len().saturating_sub(3) {
                            payload_hex.push_str(parts[i]);
                        }

                        if let Ok(data) = hex::decode(&payload_hex) {
                            if let Some(reply) = protocol.handle_packet(&data, src.to_string()).await {
                                // Respond with TXT record containing the reply
                                // Note: Max TXT string length is 255. For larger replies, we use multiple strings.
                                let reply_hex = hex::encode(reply);
                                let mut rdata = Vec::new();
                                for chunk in reply_hex.as_bytes().chunks(255) {
                                    rdata.push(chunk.len() as u8);
                                    rdata.extend_from_slice(chunk);
                                }
                                response_packet.add_answer(&question.name, 16, 60, &rdata);
                            }
                        }
                    } else if question.qtype == 1 { // A
                         // A record beaconing - respond with a signaling IP
                         response_packet.add_answer(&question.name, 1, 60, &[127, 0, 0, 1]);
                    }

                    let resp_bytes = response_packet.to_bytes();
                    if let Err(e) = socket.send_to(&resp_bytes, src).await {
                        tracing::error!("Failed to send DNS response to {}: {}", src, e);
                    }
                }
            }
            Err(e) => tracing::error!("DNS Recv error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_parsing() {
        // Standard query for example.com A record
        let packet = hex::decode("123401000001000000000000076578616d706c6503636f6d0000010001").unwrap();
        let dns = DnsPacket::from_bytes(&packet).unwrap();
        assert_eq!(dns.header.id, 0x1234);
        assert_eq!(dns.questions.len(), 1);
        assert_eq!(dns.questions[0].name, "example.com");
        assert_eq!(dns.questions[0].qtype, 1);
    }

    #[test]
    fn test_dns_response_building() {
        let mut dns = DnsPacket::new();
        dns.header.id = 0x1234;
        dns.header.flags = 0x8180;
        dns.header.qdcount = 1;
        dns.questions.push(DnsQuestion {
            name: "example.com".to_string(),
            qtype: 1,
            qclass: 1,
        });
        dns.add_answer("example.com", 1, 60, &[127, 0, 0, 1]);

        let bytes = dns.to_bytes();
        let parsed = DnsPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header.id, 0x1234);
        assert_eq!(parsed.header.ancount, 1);
        // answers field parsing is not implemented yet in from_bytes, but it proves basic structure
    }
}
