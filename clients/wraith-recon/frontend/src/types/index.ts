// WRAITH Recon - Type Definitions

// Engagement Status
export type EngagementStatus =
  | 'NotLoaded'
  | 'Ready'
  | 'Active'
  | 'Paused'
  | 'Completed'
  | 'Terminated';

// Rules of Engagement
export interface RulesOfEngagement {
  id: string;
  organization: string;
  title: string;
  start_time: string;
  end_time: string;
  authorized_operators: string[];
  authorized_cidrs: string[];
  authorized_domains: string[];
  excluded_cidrs: string[];
  excluded_domains: string[];
  authorized_techniques: string[];
  prohibited_techniques: string[];
  signer_public_key: string;
  signature: string;
}

// Scope Summary
export interface ScopeSummary {
  authorized_cidr_count: number;
  authorized_domain_count: number;
  excluded_cidr_count: number;
  excluded_domain_count: number;
  custom_target_count: number;
}

// Validation Result
export interface ValidationResult {
  in_scope: boolean;
  reason: string;
}

// Target Types
export type TargetType =
  | { IpAddress: string }
  | { CidrRange: string }
  | { Domain: string }
  | { Hostname: string }
  | { Url: string }
  | { PortSpec: string };

export interface Target {
  id: string;
  target_type: TargetType;
  value: string;
  description?: string;
  excluded: boolean;
  created_at: number;
}

// Probe Types
export type ProbeType =
  | 'TcpSyn'
  | 'TcpConnect'
  | 'TcpAck'
  | 'UdpProbe'
  | 'IcmpPing';

// Active Scan Configuration
export interface ActiveScanConfig {
  targets: string[];
  ports: number[];
  probe_type: ProbeType;
  rate_limit: number;
  timeout_ms: number;
  retries: number;
  service_detection: boolean;
  os_detection: boolean;
}

// Active Scan Progress
export interface ActiveScanProgress {
  scan_id: string;
  status: 'Running' | 'Paused' | 'Completed' | 'Cancelled';
  total_probes: number;
  completed_probes: number;
  open_ports_found: number;
  current_target: string | null;
  started_at: string;
  estimated_completion: string | null;
}

// Probe Result
export interface ProbeResult {
  target: string;
  port: number;
  open: boolean;
  service: string | null;
  response_time_ms: number;
  probe_type: ProbeType;
  timestamp: string;
}

// Passive Scan Statistics
export interface PassiveReconStats {
  packets_captured: number;
  bytes_captured: number;
  unique_ips: number;
  services_identified: number;
  start_time: string | null;
  is_running: boolean;
}

// Network Asset
export interface NetworkAsset {
  ip: string;
  hostnames: string[];
  ports: number[];
  services: string[];
  os_fingerprint: string | null;
  first_seen: string;
  last_seen: string;
  packet_count: number;
}

// Channel Types
export type ChannelType =
  | 'Udp'
  | 'TcpMimicry'
  | 'Https'
  | 'DnsTunnel'
  | 'Icmp';

// Channel Info
export interface ChannelInfo {
  id: string;
  channel_type: ChannelType;
  target: string;
  port: number | null;
  state: 'Open' | 'Active' | 'Closed' | 'Error';
  bytes_sent: number;
  bytes_received: number;
  created_at: string;
  last_activity: string;
  stats: ChannelStats;
}

// Channel Stats
export interface ChannelStats {
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
  errors: number;
  latency_ms: number | null;
}

// Kill Switch State
export interface KillSwitchState {
  activated: boolean;
  activated_at: string | null;
  reason: string | null;
  activated_by: string | null;
  signal_id: string | null;
}

// Audit Entry
export interface AuditEntry {
  id: string;
  sequence: number;
  timestamp: string;
  level: 'Info' | 'Warning' | 'Error' | 'Emergency';
  category: string;
  operator_id: string;
  summary: string;
  details: string | null;
  mitre_technique: string | null;
  mitre_tactic: string | null;
  previous_hash: string;
  signature: string;
}

// Chain Verification Result
export interface ChainVerificationResult {
  valid: boolean;
  entries_verified: number;
  first_invalid_sequence: number | null;
  errors: string[];
}

// Statistics Summary
export interface StatisticsSummary {
  targets_scanned: number;
  ports_discovered: number;
  services_identified: number;
  bytes_exfiltrated: number;
  packets_captured: number;
  scope_violations: number;
  audit_entries: number;
  channel_operations: number;
}

// Database Stats
export interface DatabaseStats {
  audit_entries: number;
  roe_entries: number;
  db_size_bytes: number;
}

// Node Status
export interface NodeStatus {
  running: boolean;
  peer_id: string | null;
  active_routes: number;
}

// Timing Info
export interface TimingInfo {
  start_time: string | null;
  end_time: string | null;
  is_active: boolean;
  time_remaining_secs: number | null;
  status: 'NotSet' | 'NotStarted' | 'Active' | 'Expired' | 'Suspended';
}

// Engagement Status Response
export interface EngagementStatusResponse {
  status: EngagementStatus;
  engagement_id: string | null;
  roe_id: string | null;
  roe_title: string | null;
  operator_id: string;
  time_remaining: string | null;
  kill_switch_active: boolean;
}

// UI State
export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  notificationsEnabled: boolean;
  refreshIntervalMs: number;
}
