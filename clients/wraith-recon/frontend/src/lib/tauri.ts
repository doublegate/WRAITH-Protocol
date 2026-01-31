// WRAITH Recon - Tauri IPC Bindings

import { invoke } from '@tauri-apps/api/core';
import type {
  RulesOfEngagement,
  ScopeSummary,
  ValidationResult,
  Target,
  ActiveScanConfig,
  ActiveScanProgress,
  ProbeResult,
  PassiveReconStats,
  NetworkAsset,
  ChannelType,
  ChannelInfo,
  ChannelStats,
  KillSwitchState,
  AuditEntry,
  ChainVerificationResult,
  StatisticsSummary,
  DatabaseStats,
  NodeStatus,
  TimingInfo,
  EngagementStatusResponse,
} from '../types';

// =============================================================================
// Rules of Engagement Commands
// =============================================================================

export async function loadRoe(roe: RulesOfEngagement): Promise<void> {
  return invoke<void>('load_roe', { roe });
}

export async function loadRoeFile(filePath: string): Promise<void> {
  return invoke<void>('load_roe_file', { filePath });
}

export async function getRoe(): Promise<RulesOfEngagement | null> {
  return invoke<RulesOfEngagement | null>('get_roe');
}

export async function validateRoe(roe: RulesOfEngagement): Promise<{
  valid: boolean;
  errors: string[];
}> {
  return invoke<{ valid: boolean; errors: string[] }>('validate_roe', { roe });
}

// =============================================================================
// Engagement Commands
// =============================================================================

export async function startEngagement(): Promise<string> {
  return invoke<string>('start_engagement');
}

export async function stopEngagement(reason: string): Promise<void> {
  return invoke<void>('stop_engagement', { reason });
}

export async function pauseEngagement(): Promise<void> {
  return invoke<void>('pause_engagement');
}

export async function resumeEngagement(): Promise<void> {
  return invoke<void>('resume_engagement');
}

export async function getEngagementStatus(): Promise<EngagementStatusResponse> {
  return invoke<EngagementStatusResponse>('get_engagement_status');
}

// =============================================================================
// Scope Commands
// =============================================================================

export async function validateTarget(target: string): Promise<ValidationResult> {
  return invoke<ValidationResult>('validate_target', { target });
}

export async function addCustomTarget(target: Target): Promise<void> {
  return invoke<void>('add_custom_target', { target });
}

export async function getScopeSummary(): Promise<ScopeSummary> {
  return invoke<ScopeSummary>('get_scope_summary');
}

// =============================================================================
// Kill Switch Commands
// =============================================================================

export async function activateKillSwitch(reason: string): Promise<void> {
  return invoke<void>('activate_kill_switch', { reason });
}

export async function processKillSwitchSignal(signalJson: string): Promise<void> {
  return invoke<void>('process_kill_switch_signal', { signalJson });
}

export async function isKillSwitchActive(): Promise<KillSwitchState> {
  return invoke<KillSwitchState>('is_kill_switch_active');
}

// =============================================================================
// Passive Reconnaissance Commands
// =============================================================================

export async function startPassiveRecon(
  interfaceName?: string,
  captureTimeoutSecs?: number
): Promise<string> {
  return invoke<string>('start_passive_recon', { interfaceName, captureTimeoutSecs });
}

export async function stopPassiveRecon(): Promise<PassiveReconStats> {
  return invoke<PassiveReconStats>('stop_passive_recon');
}

export async function getPassiveReconStats(): Promise<PassiveReconStats> {
  return invoke<PassiveReconStats>('get_passive_recon_stats');
}

export async function getDiscoveredAssets(): Promise<NetworkAsset[]> {
  return invoke<NetworkAsset[]>('get_discovered_assets');
}

// =============================================================================
// Active Reconnaissance Commands
// =============================================================================

export async function startActiveScan(config: ActiveScanConfig): Promise<string> {
  return invoke<string>('start_active_scan', { config });
}

export async function stopActiveScan(): Promise<void> {
  return invoke<void>('stop_active_scan');
}

export async function getActiveScanProgress(): Promise<ActiveScanProgress | null> {
  return invoke<ActiveScanProgress | null>('get_active_scan_progress');
}

export async function getActiveScanResults(): Promise<ProbeResult[]> {
  return invoke<ProbeResult[]>('get_active_scan_results');
}

// =============================================================================
// Channel Commands
// =============================================================================

export async function openChannel(
  channelType: ChannelType,
  target: string,
  port?: number
): Promise<string> {
  return invoke<string>('open_channel', { channelType, target, port });
}

export async function sendThroughChannel(
  channelId: string,
  data: number[]
): Promise<number> {
  return invoke<number>('send_through_channel', { channelId, data });
}

export async function closeChannel(channelId: string): Promise<void> {
  return invoke<void>('close_channel', { channelId });
}

export async function listChannels(): Promise<ChannelInfo[]> {
  return invoke<ChannelInfo[]>('list_channels');
}

export async function getChannelStats(channelId: string): Promise<ChannelStats> {
  return invoke<ChannelStats>('get_channel_stats', { channelId });
}

// =============================================================================
// Audit Commands
// =============================================================================

export async function getAuditEntries(
  sinceSequence: number,
  limit: number
): Promise<AuditEntry[]> {
  return invoke<AuditEntry[]>('get_audit_entries', { sinceSequence, limit });
}

export async function verifyAuditChain(): Promise<ChainVerificationResult> {
  return invoke<ChainVerificationResult>('verify_audit_chain');
}

export async function exportAuditLog(): Promise<string> {
  return invoke<string>('export_audit_log');
}

export async function addAuditNote(note: string): Promise<void> {
  return invoke<void>('add_audit_note', { note });
}

// =============================================================================
// Node Commands
// =============================================================================

export async function startNode(): Promise<void> {
  return invoke<void>('start_node');
}

export async function stopNode(): Promise<void> {
  return invoke<void>('stop_node');
}

export async function getNodeStatus(): Promise<NodeStatus> {
  return invoke<NodeStatus>('get_node_status');
}

export async function getPeerId(): Promise<string | null> {
  return invoke<string | null>('get_peer_id');
}

// =============================================================================
// Statistics Commands
// =============================================================================

export async function getStatistics(): Promise<StatisticsSummary> {
  return invoke<StatisticsSummary>('get_statistics');
}

export async function getDatabaseStats(): Promise<DatabaseStats> {
  return invoke<DatabaseStats>('get_database_stats');
}

export async function getTimingInfo(): Promise<TimingInfo> {
  return invoke<TimingInfo>('get_timing_info');
}

// =============================================================================
// Operator Commands
// =============================================================================

export async function setOperatorId(operatorId: string): Promise<void> {
  return invoke<void>('set_operator_id', { operatorId });
}

export async function getOperatorId(): Promise<string> {
  return invoke<string>('get_operator_id');
}
