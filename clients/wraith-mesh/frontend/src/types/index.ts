// Type definitions for WRAITH Mesh

/** Peer type classification */
export type PeerType = 'self' | 'direct' | 'relay' | 'indirect';

/** Connection status */
export type ConnectionStatus = 'excellent' | 'good' | 'fair' | 'poor' | 'failed';

/** NAT type classification */
export type NatType =
  | 'none'
  | 'full_cone'
  | 'restricted_cone'
  | 'port_restricted'
  | 'symmetric'
  | 'unknown';

/** Information about a peer in the network */
export interface PeerInfo {
  id: string;
  label: string;
  peer_type: PeerType;
  connected_at: number;
  last_seen: number;
  location?: string;
}

/** Information about a link between two peers */
export interface LinkInfo {
  source: string;
  target: string;
  latency_ms: number;
  bandwidth_mbps: number;
  packet_loss: number;
  strength: number;
}

/** DHT statistics */
export interface DhtStats {
  total_nodes: number;
  routing_table_size: number;
  stored_keys: number;
  lookup_count_1h: number;
  avg_lookup_latency_ms: number;
}

/** Complete network snapshot */
export interface NetworkSnapshot {
  timestamp: number;
  nodes: PeerInfo[];
  links: LinkInfo[];
  dht_stats: DhtStats;
  health_score: number;
}

/** Metrics history entry */
export interface MetricsEntry {
  timestamp: number;
  peer_count: number;
  avg_latency_ms: number;
  total_bandwidth_mbps: number;
  packet_loss_rate: number;
}

/** Peer in a routing bucket */
export interface BucketPeer {
  peer_id: string;
  address: string;
  last_seen: number;
  rtt_ms: number;
  is_alive: boolean;
}

/** Routing bucket in DHT */
export interface RoutingBucket {
  index: number;
  distance_prefix: string;
  peers: BucketPeer[];
  peer_count: number;
  capacity: number;
}

/** DHT lookup hop trace */
export interface LookupHop {
  peer_id: string;
  distance: number;
  rtt_ms: number;
  responded: boolean;
}

/** DHT lookup result */
export interface LookupResult {
  key: string;
  status: 'found' | 'not_found' | 'timeout' | 'error';
  value?: string;
  hops: LookupHop[];
  total_time_ms: number;
}

/** Stored key in DHT */
export interface StoredKey {
  key: string;
  value: string;
  provider_id: string;
  size_bytes: number;
  expires_at: number;
}

/** Ping result */
export interface PingResult {
  peer_id: string;
  packets_sent: number;
  packets_received: number;
  packet_loss: number;
  min_rtt_ms: number;
  avg_rtt_ms: number;
  max_rtt_ms: number;
  jitter_ms: number;
  rtts_ms: number[];
}

/** Bandwidth test result */
export interface BandwidthResult {
  peer_id: string;
  upload_mbps: number;
  download_mbps: number;
  bytes_sent: number;
  bytes_received: number;
  duration_ms: number;
}

/** Health issue */
export interface HealthIssue {
  severity: number;
  issue_type: string;
  description: string;
}

/** Connection health report */
export interface HealthReport {
  peer_id: string;
  score: number;
  status: ConnectionStatus;
  latency_ms: number;
  jitter_ms: number;
  packet_loss: number;
  bandwidth_mbps: number;
  uptime_secs: number;
  reconnect_count: number;
  issues: HealthIssue[];
  recommendations: string[];
}

/** NAT detection result */
export interface NatDetectionResult {
  nat_type: NatType;
  public_ip?: string;
  public_port?: number;
  hairpin_support: boolean;
  mapping_lifetime_secs?: number;
}

/** D3 node type (extends PeerInfo with position) */
export interface GraphNode extends PeerInfo {
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

/** D3 link type */
export interface GraphLink {
  source: GraphNode | string;
  target: GraphNode | string;
  latency_ms: number;
  bandwidth_mbps: number;
  packet_loss: number;
  strength: number;
}
