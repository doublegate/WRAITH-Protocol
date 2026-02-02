# WRAITH Protocol Incident Response Runbook

**Version:** 1.0
**Last Updated:** 2025-12-01
**Maintainer:** WRAITH Security Team

---

## Overview

This runbook provides procedures for responding to security incidents affecting WRAITH Protocol deployments.

**Incident Categories:**
- **P0 (Critical)**: Active breach, data exfiltration, complete service outage
- **P1 (High)**: Suspected breach, partial outage, key compromise risk
- **P2 (Medium)**: Performance degradation, suspicious activity
- **P3 (Low)**: Minor anomalies, non-critical alerts

---

## Incident Response Team

**Roles:**
- **Incident Commander (IC)**: Coordinates response, makes decisions
- **Security Lead**: Analyzes threats, recommends mitigations
- **Operations Lead**: Executes technical remediation
- **Communications Lead**: Handles stakeholder communications

**Contact Information:**
- **On-Call:** +1-555-WRAITH-OPS
- **Security Email:** security@wraith.example
- **Incident Slack:** #wraith-incidents

---

## Phase 1: Detection & Triage (0-15 minutes)

### 1.1 Incident Detection

**Automated Alerts:**
- Monitoring system alerts (Prometheus, Grafana)
- Log analysis alerts (ELK, Splunk)
- Intrusion detection system (Snort, Suricata)
- File integrity monitoring (AIDE, Tripwire)

**Manual Detection:**
- User reports of suspicious activity
- Anomalous network traffic
- Unexpected system behavior
- Failed authentication attempts

### 1.2 Initial Assessment

```bash
# Check system status
sudo systemctl status wraith
ps aux | grep wraith

# Check recent logs
sudo journalctl -u wraith -n 500 --no-pager

# Check network connections
sudo ss -tuanp | grep wraith

# Check file integrity
sudo aide --check

# Check running processes
sudo lsof -p $(pgrep wraith)
```

### 1.3 Incident Categorization

**Assign Priority (P0-P3):**
- P0: Node compromised, keys stolen, active data breach
- P1: Suspected compromise, unusual access patterns
- P2: Performance issues, failed authentications
- P3: Minor anomalies, non-critical warnings

**Initial Actions:**
- Document incident start time
- Create incident ticket (Jira, ServiceNow)
- Notify Incident Commander
- Assemble response team

---

## Phase 2: Containment (15-60 minutes)

### 2.1 Immediate Containment

**For Key Compromise (P0/P1):**

```bash
# IMMEDIATE: Stop the service
sudo systemctl stop wraith

# Isolate node from network
sudo iptables -A INPUT -j DROP
sudo iptables -A OUTPUT -j DROP

# Preserve evidence
sudo cp -r /var/lib/wraith /forensics/wraith-$(date +%Y%m%d-%H%M%S)
sudo cp /var/log/syslog /forensics/syslog-$(date +%Y%m%d-%H%M%S)

# Revoke compromised keys (if DHT supports revocation)
wraith key revoke <compromised_key_id>
```

**For DoS Attack (P1/P2):**

```bash
# Enable rate limiting
sudo iptables -A INPUT -p udp --dport 5000 -m limit --limit 100/s -j ACCEPT
sudo iptables -A INPUT -p udp --dport 5000 -j DROP

# Block attacking IP
sudo iptables -A INPUT -s <attacker_ip> -j DROP

# Reduce connection limits
# Edit /etc/wraith/config.toml: max_connections = 100

# Restart service
sudo systemctl restart wraith
```

**For Data Exfiltration (P0):**

```bash
# Block outbound connections
sudo iptables -A OUTPUT -p tcp --dport 443 -j DROP

# Monitor active connections
sudo tcpdump -i eth0 -w /forensics/capture-$(date +%Y%m%d-%H%M%S).pcap

# Check for data staging
find /var/lib/wraith -type f -mmin -60
```

### 2.2 Evidence Preservation

```bash
# Create forensics directory
sudo mkdir -p /forensics/$(date +%Y%m%d)

# Capture memory dump (if available)
sudo volatility -f /dev/mem pslist > /forensics/memory.txt

# Capture disk snapshot
sudo dd if=/dev/sda of=/forensics/disk.img bs=4M status=progress

# Capture network traffic
sudo tcpdump -i eth0 -s 0 -w /forensics/network.pcap &

# Copy logs
sudo cp -r /var/lib/wraith/logs /forensics/
sudo journalctl -u wraith --since "24 hours ago" > /forensics/journal.log

# Hash evidence
cd /forensics
sha256sum * > evidence.sha256
```

---

## Phase 3: Investigation (1-4 hours)

### 3.1 Log Analysis

```bash
# Analyze authentication failures
sudo journalctl -u wraith | grep "authentication failed"

# Check for suspicious IPs
sudo journalctl -u wraith | grep -oP '\d+\.\d+\.\d+\.\d+' | sort | uniq -c | sort -rn

# Analyze file transfers
sudo journalctl -u wraith | grep "transfer"

# Check for privilege escalation
sudo journalctl -u wraith | grep -i "privilege\|sudo\|root"
```

### 3.2 Network Traffic Analysis

```bash
# Analyze pcap with Wireshark
wireshark /forensics/network.pcap

# Extract suspicious flows
tshark -r /forensics/network.pcap -Y "ip.addr == <suspicious_ip>"

# Analyze DNS queries
tshark -r /forensics/network.pcap -Y "dns" -T fields -e dns.qry.name

# Check for data exfiltration patterns
tshark -r /forensics/network.pcap -z conv,tcp
```

### 3.3 File System Analysis

```bash
# Check recently modified files
find /var/lib/wraith -type f -mtime -1 -ls

# Check for suspicious files
find /var/lib/wraith -name "*.sh" -o -name "*.exe"

# Verify file integrity
sudo aide --check --report /forensics/aide-report.txt

# Check for rootkits
sudo rkhunter --check --report-warnings-only
```

### 3.4 Determine Root Cause

**Common Attack Vectors:**
1. **Compromised Dependencies**: Check `cargo audit`
2. **Social Engineering**: Review access logs
3. **Zero-Day Exploit**: Check CVE databases
4. **Insider Threat**: Review personnel access
5. **Supply Chain Attack**: Verify binary checksums

---

## Phase 4: Eradication (2-8 hours)

### 4.1 Remove Threats

**Malware Removal:**

```bash
# Stop service
sudo systemctl stop wraith

# Remove malicious files
sudo rm -rf /var/lib/wraith/malicious_dir

# Reinstall clean binary
sudo rm /usr/local/bin/wraith
sudo install -m 755 /backup/wraith-clean /usr/local/bin/wraith

# Verify checksums
sha256sum /usr/local/bin/wraith
# Compare with official release hash
```

**Patch Vulnerabilities:**

```bash
# Update system
sudo apt update && sudo apt upgrade

# Update WRAITH to patched version
git fetch --all
git checkout v0.8.1  # Patched version
cargo build --release
sudo install -m 755 target/release/wraith /usr/local/bin/

# Update dependencies
cargo update
cargo audit fix
```

### 4.2 Rotate Compromised Credentials

```bash
# Generate new node keys
wraith keygen -o /var/lib/wraith/keys/new_node_key.enc

# Update configuration
sudo sed -i 's/node_key.enc/new_node_key.enc/' /etc/wraith/config.toml

# Securely delete old keys
sudo shred -vfz -n 10 /var/lib/wraith/keys/node_key.enc

# Restart with new keys
sudo systemctl restart wraith
```

---

## Phase 5: Recovery (4-24 hours)

### 5.1 Service Restoration

```bash
# Verify clean state
sudo rkhunter --check
sudo aide --check

# Remove network isolation
sudo iptables -D INPUT -j DROP
sudo iptables -D OUTPUT -j DROP

# Restore firewall rules
sudo iptables-restore < /etc/iptables/rules.v4

# Start service
sudo systemctl start wraith

# Monitor for anomalies
sudo journalctl -u wraith -f
```

### 5.2 Verification

```bash
# Check node status
wraith status

# Verify connectivity
wraith peers

# Test file transfer
wraith send test.dat <peer_id>

# Monitor performance
wraith stats
```

### 5.3 Enhanced Monitoring

```bash
# Enable debug logging
sudo sed -i 's/level = "info"/level = "debug"/' /etc/wraith/config.toml
sudo systemctl restart wraith

# Set up anomaly detection
# Configure Prometheus alerts

# Increase log retention
sudo sed -i 's/max_age = "30d"/max_age = "90d"/' /etc/wraith/config.toml
```

---

## Phase 6: Post-Incident Activities

### 6.1 Incident Report

**Template:**

```
Incident ID: INC-2025-001
Date: 2025-12-01
Priority: P0
Status: Resolved

SUMMARY:
[Brief description of incident]

TIMELINE:
- 10:00 UTC: Incident detected
- 10:15 UTC: Service isolated
- 11:00 UTC: Root cause identified
- 14:00 UTC: Threat eradicated
- 16:00 UTC: Service restored

ROOT CAUSE:
[Detailed analysis]

IMPACT:
- Affected nodes: 3
- Downtime: 6 hours
- Data compromised: None
- Users affected: 0

ACTIONS TAKEN:
1. Isolated compromised nodes
2. Rotated all credentials
3. Patched vulnerability
4. Enhanced monitoring

LESSONS LEARNED:
1. Need faster detection
2. Improve isolation procedures
3. Add automated remediation

PREVENTIVE MEASURES:
1. Enable intrusion detection
2. Implement canary deployments
3. Enhance access controls
```

### 6.2 Post-Mortem Meeting

**Agenda:**
1. Incident timeline review
2. Root cause analysis
3. Response effectiveness
4. Action items
5. Prevention strategies

**Attendees:**
- Incident Commander
- Security Team
- Operations Team
- Engineering leads

### 6.3 Security Improvements

```bash
# Implement security enhancements
1. Enable 2FA for administrative access
2. Implement network segmentation
3. Deploy HIDS/NIDS
4. Add log aggregation (ELK stack)
5. Schedule regular security audits

# Update runbooks
6. Document lessons learned
7. Update incident procedures
8. Conduct tabletop exercises
```

---

## Incident Response Checklist

### Detection Phase
- [ ] Alert received and acknowledged
- [ ] Incident ticket created
- [ ] Response team notified
- [ ] Initial assessment completed
- [ ] Incident priority assigned

### Containment Phase
- [ ] Service isolated (if needed)
- [ ] Evidence preserved
- [ ] Attacking IPs blocked
- [ ] Compromised keys revoked
- [ ] Stakeholders notified

### Investigation Phase
- [ ] Logs analyzed
- [ ] Network traffic reviewed
- [ ] File system examined
- [ ] Root cause identified
- [ ] Scope determined

### Eradication Phase
- [ ] Threats removed
- [ ] Vulnerabilities patched
- [ ] Credentials rotated
- [ ] Clean binary deployed
- [ ] System hardened

### Recovery Phase
- [ ] Service restored
- [ ] Functionality verified
- [ ] Monitoring enhanced
- [ ] Performance validated
- [ ] Users notified

### Post-Incident Phase
- [ ] Incident report completed
- [ ] Post-mortem conducted
- [ ] Action items tracked
- [ ] Runbooks updated
- [ ] Training delivered

---

## Common Incidents

### Scenario 1: Suspected Key Compromise

**Symptoms:**
- Unauthorized transfers
- Unknown peers connected
- Configuration changes

**Response:**
1. Immediately stop service
2. Revoke compromised keys
3. Generate new keypair
4. Update all configurations
5. Notify all peers

### Scenario 2: DoS/DDoS Attack

**Symptoms:**
- High CPU/memory usage
- Connection timeouts
- Slow response times

**Response:**
1. Enable rate limiting
2. Block attacking IPs
3. Enable DDoS mitigation (Cloudflare, etc.)
4. Scale horizontally if needed
5. Contact ISP for upstream filtering

### Scenario 3: Data Exfiltration

**Symptoms:**
- Unusual outbound traffic
- Large file transfers
- Suspicious network connections

**Response:**
1. Block outbound connections
2. Capture network traffic
3. Identify exfiltration target
4. Determine data scope
5. Notify affected parties (GDPR, etc.)

---

## Escalation Procedures

**When to Escalate:**
- P0 incident not resolved within 4 hours
- Multiple nodes compromised
- Customer data at risk
- Regulatory implications

**Escalation Path:**
1. Incident Commander
2. Security Director
3. CTO/CISO
4. CEO (for major incidents)
5. Legal/PR (for regulatory/public incidents)

---

## External Resources

- **NIST Incident Response Guide**: https://nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-61r2.pdf
- **SANS Incident Handling**: https://www.sans.org/white-papers/33901/
- **US-CERT**: https://www.cisa.gov/uscert

---

## Appendix

### A. Incident Severity Matrix

| Severity | Impact | Examples |
|----------|--------|----------|
| P0 | Critical | Active breach, complete outage, data loss |
| P1 | High | Suspected compromise, partial outage |
| P2 | Medium | Performance degradation, minor security event |
| P3 | Low | Informational alerts, minor anomalies |

### B. Communication Templates

**Internal Notification:**
```
Subject: [P0] WRAITH Incident - <Brief Description>

Team,

We are experiencing a P0 incident affecting WRAITH Protocol nodes.

Impact: <description>
ETA: <estimated resolution time>
War Room: https://meet.google.com/wraith-incident

Incident Commander: <name>
Next Update: <time>
```

**External Notification:**
```
Subject: WRAITH Service Disruption

Dear Users,

We are currently experiencing a service disruption affecting WRAITH Protocol.

Impact: <user-facing description>
Status: Our team is working to restore service
ETA: <estimated resolution time>

We will provide updates every hour.

Thank you for your patience.
```
