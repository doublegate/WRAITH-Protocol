// TypeScript types for WRAITH-Sync

// Overall sync status
export interface OverallStatus {
  status: 'idle' | 'syncing' | 'paused' | 'error' | 'offline';
  total_folders: number;
  syncing_folders: number;
  total_files: number;
  pending_operations: number;
  unresolved_conflicts: number;
  is_paused: boolean;
}

// Folder information
export interface FolderInfo {
  id: number;
  local_path: string;
  remote_path: string;
  enabled: boolean;
  paused: boolean;
  status: 'idle' | 'syncing' | 'paused' | 'error' | 'offline';
  total_files: number;
  synced_files: number;
  pending_operations: number;
  last_sync_at?: number;
}

// File information
export interface FileInfo {
  relative_path: string;
  size: number;
  modified_at: number;
  synced: boolean;
  versions: VersionInfo[];
}

// Version information
export interface VersionInfo {
  id: number;
  version_number: number;
  size: number;
  modified_at: number;
  created_at: number;
}

// Conflict information
export interface ConflictInfo {
  id: number;
  file_path: string;
  folder_path: string;
  local_modified_at: number;
  remote_modified_at: number;
  remote_device: string;
  created_at: number;
}

// Device information
export interface DeviceInfo {
  id: number;
  device_id: string;
  device_name: string;
  last_seen: number;
  is_self: boolean;
}

// Application settings
export interface AppSettings {
  upload_limit: number;
  download_limit: number;
  conflict_strategy: 'last_writer_wins' | 'keep_both' | 'manual';
  max_versions: number;
  version_retention_days: number;
  enable_delta_sync: boolean;
  debounce_ms: number;
  auto_start: boolean;
  notifications_enabled: boolean;
  theme: 'light' | 'dark' | 'system';
  device_name: string;
}

// Sync operation types
export type SyncOperationType = 'upload' | 'download' | 'delete' | 'conflict';

// Resolution types
export type ConflictResolution = 'local' | 'remote' | 'keep_both';
