// TypeScript types for WRAITH-Share

// =============================================================================
// Group Types
// =============================================================================

/** Group information */
export interface Group {
  id: string;
  name: string;
  description: string | null;
  created_at: number;
  created_by: string;
  encryption_key: string;
}

/** Group member information */
export interface GroupMember {
  group_id: string;
  peer_id: string;
  role: MemberRole;
  display_name: string | null;
  joined_at: number;
  invited_by: string | null;
}

/** Extended group info with counts */
export interface GroupInfo {
  id: string;
  name: string;
  description: string | null;
  created_at: number;
  created_by: string;
  member_count: number;
  file_count: number;
  is_admin: boolean;
  my_role: MemberRole;
}

/** Member role types */
export type MemberRole = 'admin' | 'write' | 'read';

/** Exported invitation for sharing */
export interface ExportedInvitation {
  group_id: string;
  group_name: string;
  invitation_token: string;
  role: MemberRole;
  expires_at: number;
  inviter_id: string;
}

// =============================================================================
// File Types
// =============================================================================

/** Shared file information */
export interface SharedFile {
  id: string;
  group_id: string;
  path: string;
  name: string;
  size: number;
  mime_type: string | null;
  content_hash: string;
  encrypted_key: string;
  version: number;
  uploaded_by: string;
  uploaded_at: number;
  modified_at: number;
}

/** File version information */
export interface VersionInfo {
  version: number;
  size: number;
  content_hash: string;
  uploaded_by: string;
  uploaded_at: number;
  comment: string | null;
}

/** Version summary for a file */
export interface VersionSummary {
  file_id: string;
  current_version: number;
  total_versions: number;
  total_size: number;
  oldest_version_at: number;
  newest_version_at: number;
}

// =============================================================================
// Activity Types
// =============================================================================

/** Activity log entry */
export interface ActivityInfo {
  id: number;
  group_id: string;
  actor_id: string;
  actor_name: string | null;
  action_type: ActivityType;
  target_type: string;
  target_id: string | null;
  target_name: string | null;
  details: string | null;
  created_at: number;
}

/** Activity type enum */
export type ActivityType =
  | 'file_uploaded'
  | 'file_downloaded'
  | 'file_deleted'
  | 'file_version_restored'
  | 'member_joined'
  | 'member_left'
  | 'member_invited'
  | 'member_removed'
  | 'member_role_changed'
  | 'group_created'
  | 'group_updated'
  | 'share_link_created'
  | 'share_link_accessed'
  | 'share_link_revoked';

/** Activity statistics */
export interface ActivityStats {
  total_activities: number;
  uploads: number;
  downloads: number;
  members_joined: number;
  members_left: number;
  first_activity_at: number | null;
  last_activity_at: number | null;
}

// =============================================================================
// Share Link Types
// =============================================================================

/** Share link information */
export interface ShareLinkInfo {
  id: string;
  file_id: string;
  file_name: string;
  created_by: string;
  created_at: number;
  expires_at: number | null;
  max_downloads: number | null;
  download_count: number;
  has_password: boolean;
  is_active: boolean;
}

/** Share link creation options */
export interface ShareLinkOptions {
  expires_in_hours: number | null;
  password: string | null;
  max_downloads: number | null;
}

// =============================================================================
// UI State Types
// =============================================================================

/** View mode for file browser */
export type ViewMode = 'grid' | 'list';

/** Sort options for files */
export type SortBy = 'name' | 'size' | 'date' | 'type';
export type SortOrder = 'asc' | 'desc';

/** File upload status */
export interface UploadProgress {
  id: string;
  fileName: string;
  progress: number;
  status: 'pending' | 'uploading' | 'completed' | 'failed';
  error?: string;
}

/** Toast notification */
export interface Toast {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration?: number;
}

// =============================================================================
// Utility Types
// =============================================================================

/** Format bytes to human readable */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

/** Format timestamp to relative time */
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp * 1000;

  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;

  return new Date(timestamp * 1000).toLocaleDateString();
}

/** Get file icon based on mime type */
export function getFileIcon(mimeType: string | null): string {
  if (!mimeType) return 'file';

  if (mimeType.startsWith('image/')) return 'image';
  if (mimeType.startsWith('video/')) return 'video';
  if (mimeType.startsWith('audio/')) return 'audio';
  if (mimeType.startsWith('text/')) return 'text';
  if (mimeType.includes('pdf')) return 'pdf';
  if (mimeType.includes('zip') || mimeType.includes('archive')) return 'archive';
  if (mimeType.includes('document') || mimeType.includes('word')) return 'document';
  if (mimeType.includes('spreadsheet') || mimeType.includes('excel')) return 'spreadsheet';
  if (mimeType.includes('presentation') || mimeType.includes('powerpoint')) return 'presentation';

  return 'file';
}

/** Truncate peer ID for display */
export function truncatePeerId(peerId: string, chars: number = 8): string {
  if (peerId.length <= chars * 2 + 3) return peerId;
  return `${peerId.slice(0, chars)}...${peerId.slice(-4)}`;
}
