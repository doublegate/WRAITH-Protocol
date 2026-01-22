// Tauri IPC Bindings for WRAITH Vault

import { invoke } from "@tauri-apps/api/core";
import type {
  SecretInfo,
  SecretCreationResult,
  Guardian,
  DistributionResult,
  DistributionStatus,
  EncryptedShard,
  RecoveryProgress,
  RecoveryResult,
  VaultStats,
  RuntimeStatistics,
  NodeStatus,
} from "../types";

// =============================================================================
// Secret Commands
// =============================================================================

export async function createSecret(
  name: string,
  secretData: number[],
  secretType: string,
  description: string | null,
  threshold: number,
  totalShares: number,
  tags: string[],
  password: string | null
): Promise<SecretCreationResult> {
  return await invoke("create_secret", {
    name,
    secretData,
    secretType,
    description,
    threshold,
    totalShares,
    tags,
    password,
  });
}

export async function getSecret(secretId: string): Promise<SecretInfo | null> {
  return await invoke("get_secret", { secretId });
}

export async function listSecrets(): Promise<SecretInfo[]> {
  return await invoke("list_secrets");
}

export async function listSecretsByType(
  secretType: string
): Promise<SecretInfo[]> {
  return await invoke("list_secrets_by_type", { secretType });
}

export async function listSecretsByTag(tag: string): Promise<SecretInfo[]> {
  return await invoke("list_secrets_by_tag", { tag });
}

export async function searchSecrets(query: string): Promise<SecretInfo[]> {
  return await invoke("search_secrets", { query });
}

export async function updateSecret(
  secretId: string,
  name: string | null,
  description: string | null,
  tags: string[] | null
): Promise<SecretInfo> {
  return await invoke("update_secret", { secretId, name, description, tags });
}

export async function deleteSecret(secretId: string): Promise<void> {
  await invoke("delete_secret", { secretId });
}

export async function getSecretsNeedingRotation(
  maxAgeDays: number
): Promise<SecretInfo[]> {
  return await invoke("get_secrets_needing_rotation", { maxAgeDays });
}

// =============================================================================
// Guardian Commands
// =============================================================================

export async function addGuardian(
  name: string,
  peerId: string,
  publicKey: string,
  notes: string | null
): Promise<Guardian> {
  return await invoke("add_guardian", { name, peerId, publicKey, notes });
}

export async function getGuardian(guardianId: string): Promise<Guardian> {
  return await invoke("get_guardian", { guardianId });
}

export async function getGuardianByPeerId(peerId: string): Promise<Guardian> {
  return await invoke("get_guardian_by_peer_id", { peerId });
}

export async function listGuardians(): Promise<Guardian[]> {
  return await invoke("list_guardians");
}

export async function listGuardiansByStatus(
  status: string
): Promise<Guardian[]> {
  return await invoke("list_guardians_by_status", { status });
}

export async function listAvailableGuardians(): Promise<Guardian[]> {
  return await invoke("list_available_guardians");
}

export async function updateGuardian(guardian: Guardian): Promise<void> {
  await invoke("update_guardian", { guardian });
}

export async function updateGuardianStatus(
  guardianId: string,
  status: string
): Promise<void> {
  await invoke("update_guardian_status", { guardianId, status });
}

export async function removeGuardian(guardianId: string): Promise<Guardian> {
  return await invoke("remove_guardian", { guardianId });
}

export async function recordHealthCheck(
  guardianId: string,
  success: boolean,
  responseTimeMs: number | null,
  error: string | null
): Promise<void> {
  await invoke("record_health_check", {
    guardianId,
    success,
    responseTimeMs,
    error,
  });
}

export async function selectGuardiansForDistribution(
  count: number
): Promise<Guardian[]> {
  return await invoke("select_guardians_for_distribution", { count });
}

// =============================================================================
// Distribution Commands
// =============================================================================

export async function prepareDistribution(
  secretId: string,
  encryptionKey: string,
  guardianIds: string[]
): Promise<DistributionResult> {
  return await invoke("prepare_distribution", {
    secretId,
    encryptionKey,
    guardianIds,
  });
}

export async function markShardDelivered(
  shardId: string,
  guardianId: string
): Promise<void> {
  await invoke("mark_shard_delivered", { shardId, guardianId });
}

export async function getDistributionStatus(
  secretId: string
): Promise<DistributionStatus | null> {
  return await invoke("get_distribution_status", { secretId });
}

// =============================================================================
// Recovery Commands
// =============================================================================

export async function startRecovery(secretId: string): Promise<string> {
  return await invoke("start_recovery", { secretId });
}

export async function addRecoveryShard(
  sessionId: string,
  shard: EncryptedShard,
  encryptionKey: string
): Promise<RecoveryProgress> {
  return await invoke("add_recovery_shard", { sessionId, shard, encryptionKey });
}

export async function completeRecovery(
  sessionId: string
): Promise<RecoveryResult> {
  return await invoke("complete_recovery", { sessionId });
}

export async function getRecoveryProgress(
  sessionId: string
): Promise<RecoveryProgress> {
  return await invoke("get_recovery_progress", { sessionId });
}

export async function cancelRecovery(sessionId: string): Promise<void> {
  await invoke("cancel_recovery", { sessionId });
}

export async function listRecoverySessions(): Promise<string[]> {
  return await invoke("list_recovery_sessions");
}

// =============================================================================
// Key Rotation Commands
// =============================================================================

export async function rotateSecretKey(
  secretId: string,
  recoveredSecret: number[],
  newPassword: string | null
): Promise<SecretCreationResult> {
  return await invoke("rotate_secret_key", {
    secretId,
    recoveredSecret,
    newPassword,
  });
}

export async function recordRotation(
  secretId: string,
  guardianIds: string[]
): Promise<SecretInfo> {
  return await invoke("record_rotation", { secretId, guardianIds });
}

// =============================================================================
// Node Commands
// =============================================================================

export async function startNode(): Promise<void> {
  await invoke("start_node");
}

export async function stopNode(): Promise<void> {
  await invoke("stop_node");
}

export async function getNodeStatus(): Promise<NodeStatus> {
  return await invoke("get_node_status");
}

export async function getPeerId(): Promise<string | null> {
  return await invoke("get_peer_id");
}

// =============================================================================
// Statistics Commands
// =============================================================================

export async function getVaultStats(): Promise<VaultStats> {
  return await invoke("get_vault_stats");
}

export async function getRuntimeStatistics(): Promise<RuntimeStatistics> {
  return await invoke("get_runtime_statistics");
}

// =============================================================================
// Shard Commands
// =============================================================================

import type { ShardAssignment } from "../types";

export async function getShardAssignments(
  secretId: string
): Promise<ShardAssignment[]> {
  return await invoke("get_shard_assignments", { secretId });
}

export async function requestShardFromGuardian(
  guardianId: string,
  secretId: string
): Promise<EncryptedShard | null> {
  return await invoke("request_shard_from_guardian", { guardianId, secretId });
}
