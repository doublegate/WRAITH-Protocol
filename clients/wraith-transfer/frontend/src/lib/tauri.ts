// WRAITH Transfer - Tauri IPC Bindings

import { invoke } from '@tauri-apps/api/core';
import type { NodeStatus, TransferInfo, SessionInfo } from '../types';

// Node commands
export async function getNodeStatus(): Promise<NodeStatus> {
  return invoke<NodeStatus>('get_node_status');
}

export async function startNode(): Promise<NodeStatus> {
  return invoke<NodeStatus>('start_node');
}

export async function stopNode(): Promise<void> {
  return invoke<void>('stop_node');
}

export async function getNodeId(): Promise<string | null> {
  return invoke<string | null>('get_node_id');
}

// Session commands
export async function getSessions(): Promise<SessionInfo[]> {
  return invoke<SessionInfo[]>('get_sessions');
}

export async function closeSession(peerId: string): Promise<void> {
  return invoke<void>('close_session', { peerId });
}

// Transfer commands
export async function sendFile(peerId: string, filePath: string): Promise<string> {
  return invoke<string>('send_file', { peerId, filePath });
}

export async function getTransfers(): Promise<TransferInfo[]> {
  return invoke<TransferInfo[]>('get_transfers');
}

export async function getTransferProgress(transferId: string): Promise<TransferInfo | null> {
  return invoke<TransferInfo | null>('get_transfer_progress', { transferId });
}

export async function cancelTransfer(transferId: string): Promise<void> {
  return invoke<void>('cancel_transfer', { transferId });
}
