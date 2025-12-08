// WRAITH Transfer - Type Definitions

export interface NodeStatus {
  running: boolean;
  node_id: string | null;
  active_sessions: number;
  active_transfers: number;
}

export interface TransferInfo {
  id: string;
  peer_id: string;
  file_name: string;
  total_bytes: number;
  transferred_bytes: number;
  progress: number;
  status: 'initializing' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
  direction: 'upload' | 'download';
}

export interface SessionInfo {
  peer_id: string;
  established_at: number;
  bytes_sent: number;
  bytes_received: number;
}

export interface ConnectionHealth {
  peer_id: string;
  rtt_ms: number;
  loss_rate: number;
  last_activity: number;
  status: 'healthy' | 'degraded' | 'unhealthy';
}
