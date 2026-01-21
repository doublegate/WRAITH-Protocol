// Tauri IPC Bindings for WRAITH-Sync

import { invoke } from '@tauri-apps/api/core';
import type {
  OverallStatus,
  FolderInfo,
  FileInfo,
  VersionInfo,
  ConflictInfo,
  DeviceInfo,
  AppSettings,
  ConflictResolution,
} from '../types';

// ============================================================================
// Status Commands
// ============================================================================

export async function getStatus(): Promise<OverallStatus> {
  return await invoke('get_status');
}

export async function pauseSync(): Promise<void> {
  await invoke('pause_sync');
}

export async function resumeSync(): Promise<void> {
  await invoke('resume_sync');
}

// ============================================================================
// Folder Commands
// ============================================================================

export async function addFolder(
  localPath: string,
  remotePath: string
): Promise<FolderInfo> {
  return await invoke('add_folder', { localPath, remotePath });
}

export async function removeFolder(folderId: number): Promise<void> {
  await invoke('remove_folder', { folderId });
}

export async function listFolders(): Promise<FolderInfo[]> {
  return await invoke('list_folders');
}

export async function getFolder(folderId: number): Promise<FolderInfo> {
  return await invoke('get_folder', { folderId });
}

export async function pauseFolder(folderId: number): Promise<void> {
  await invoke('pause_folder', { folderId });
}

export async function resumeFolder(folderId: number): Promise<void> {
  await invoke('resume_folder', { folderId });
}

export async function forceSyncFolder(folderId: number): Promise<void> {
  await invoke('force_sync_folder', { folderId });
}

// ============================================================================
// Conflict Commands
// ============================================================================

export async function listConflicts(): Promise<ConflictInfo[]> {
  return await invoke('list_conflicts');
}

export async function resolveConflict(
  conflictId: number,
  resolution: ConflictResolution
): Promise<void> {
  await invoke('resolve_conflict', { conflictId, resolution });
}

// ============================================================================
// Version History Commands
// ============================================================================

export async function getFileVersions(
  folderId: number,
  relativePath: string
): Promise<VersionInfo[]> {
  return await invoke('get_file_versions', { folderId, relativePath });
}

export async function restoreVersion(
  folderId: number,
  relativePath: string,
  versionId: number
): Promise<void> {
  await invoke('restore_version', { folderId, relativePath, versionId });
}

// ============================================================================
// Device Commands
// ============================================================================

export async function listDevices(): Promise<DeviceInfo[]> {
  return await invoke('list_devices');
}

export async function removeDevice(deviceId: string): Promise<void> {
  await invoke('remove_device', { deviceId });
}

// ============================================================================
// Settings Commands
// ============================================================================

export async function getSettings(): Promise<AppSettings> {
  return await invoke('get_settings');
}

export async function updateSettings(settings: AppSettings): Promise<void> {
  await invoke('update_settings', { settings });
}

// ============================================================================
// Ignored Patterns Commands
// ============================================================================

export async function getIgnoredPatterns(
  folderId?: number
): Promise<string[]> {
  return await invoke('get_ignored_patterns', { folderId });
}

export async function addIgnoredPattern(
  pattern: string,
  folderId?: number
): Promise<void> {
  await invoke('add_ignored_pattern', { folderId, pattern });
}

// ============================================================================
// File Browser Commands
// ============================================================================

export async function listFolderFiles(folderId: number): Promise<FileInfo[]> {
  return await invoke('list_folder_files', { folderId });
}
