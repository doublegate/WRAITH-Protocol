// WRAITH Vault Types

// Secret Types
export type SecretType =
  | "generic"
  | "crypto_key"
  | "password"
  | "recovery_phrase"
  | "certificate"
  | "api_key"
  | "document_key"
  | "ssh_key"
  | "pgp_key";

export interface ShamirConfig {
  threshold: number;
  total_shares: number;
  key_salt?: string;
}

export interface SecretInfo {
  id: string;
  name: string;
  description: string | null;
  secret_type: SecretType;
  shamir_config: ShamirConfig;
  created_at: number;
  updated_at: number;
  modified_at?: number;
  last_accessed: number | null;
  last_accessed_at?: number | null;
  rotation_count: number;
  last_rotated_at: number | null;
  key_salt: number[] | null;
  guardian_ids: string[];
  distribution_complete: boolean;
  tags: string[];
  metadata: string | null;
  version: number;
}

export interface SecretCreationResult {
  secret: SecretInfo;
  encryption_key: string;
  distribution_ready: boolean;
}

// Guardian Types
export type GuardianStatus =
  | "online"
  | "offline"
  | "pending"
  | "failed"
  | "revoked";

export type TrustLevel =
  | "untrusted"
  | "basic"
  | "trusted"
  | "high"
  | "ultimate";

export interface GuardianCapabilities {
  can_store: boolean;
  can_recover: boolean;
  max_storage: number;
  supports_encryption: boolean;
  supports_auto_refresh: boolean;
}

export interface Guardian {
  id: string;
  name: string;
  peer_id: string;
  public_key: string;
  status: GuardianStatus;
  trust_level: TrustLevel;
  capabilities: GuardianCapabilities;
  created_at: number;
  last_seen: number | null;
  last_health_check: number | null;
  shares_held: number;
  successful_recoveries: number;
  notes: string | null;
}

export interface HealthCheckResult {
  guardian_id: string;
  success: boolean;
  response_time_ms: number | null;
  error: string | null;
  checked_at: number;
}

// Shard Types
export interface EncryptedShard {
  id: string;
  secret_id: string;
  guardian_id: string;
  shard_index: number;
  share_index?: number;
  encrypted_data: number[];
  nonce: number[];
  recipient_public_key?: string;
  created_at: number;
  share_hash?: number[];
}

export interface ShardAssignment {
  secret_id: string;
  shard_id: string;
  shard_index: number;
  guardian_id: string;
  guardian_peer_id: string;
  status: "pending" | "distributed" | "failed" | "verified";
  assigned_at: number;
  delivered_at: number | null;
  verified_at: number | null;
  last_attempt_at: number | null;
  attempt_count: number;
  last_error: string | null;
}

export type DistributionState =
  | "pending"
  | "in_progress"
  | "complete"
  | "partial_success"
  | "failed";

export interface DistributionStatus {
  secret_id: string;
  total_shards: number;
  delivered_shards: number;
  pending_shards: number;
  failed_shards: number;
  successful_distributions: number;
  assignments: ShardAssignment[];
  started_at: number;
  completed_at: number | null;
  status: DistributionState;
}

export interface DistributionResult {
  shards: EncryptedShard[];
  status: DistributionStatus;
}

// Recovery Types
export type RecoveryState =
  | "initialized"
  | "collecting_shards"
  | "ready_to_reconstruct"
  | "reconstructing"
  | "completed"
  | "failed"
  | "cancelled"
  | "timed_out";

export interface GuardianResponse {
  guardian_id: string;
  received: boolean;
  received_at: number | null;
}

export interface RecoveryProgress {
  session_id: string;
  secret_id: string;
  state: RecoveryState;
  threshold_required: number;
  shards_collected: number;
  shards_received: number;
  ready_for_reconstruction: boolean;
  elapsed_ms: number;
  contributing_guardian_ids: string[];
  guardian_responses: GuardianResponse[];
}

export interface RecoveryResult {
  session_id: string;
  secret_id: string;
  success: boolean;
  recovered_data: number[] | null;
  duration_ms: number;
  recovery_time_ms: number;
  contributing_guardians: string[];
  error: string | null;
}

// Statistics Types
export interface VaultStats {
  secret_count: number;
  guardian_count: number;
  online_guardians: number;
  total_shards: number;
  distributed_secrets: number;
}

export interface RuntimeStatistics {
  secrets_created: number;
  total_recoveries: number;
  successful_recoveries: number;
  failed_recoveries: number;
  average_recovery_time_ms: number | null;
  key_rotations: number;
  shards_distributed: number;
  recovery_success_rate: number | null;
}

// Node Types
export interface NodeStatus {
  running: boolean;
  peer_id: string | null;
  active_routes: number;
}
