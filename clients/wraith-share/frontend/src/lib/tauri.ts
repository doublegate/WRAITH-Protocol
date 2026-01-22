// Tauri IPC Bindings for WRAITH-Share

import { invoke } from '@tauri-apps/api/core';
import type {
  Group,
  GroupMember,
  GroupInfo,
  SharedFile,
  VersionInfo,
  VersionSummary,
  ActivityInfo,
  ActivityStats,
  ShareLinkInfo,
  ExportedInvitation,
  MemberRole,
} from '../types';

// =============================================================================
// Group Commands
// =============================================================================

/** Create a new group */
export async function createGroup(
  name: string,
  description?: string
): Promise<Group> {
  return await invoke('create_group', { name, description });
}

/** Delete a group */
export async function deleteGroup(groupId: string): Promise<void> {
  await invoke('delete_group', { groupId });
}

/** Get a group by ID */
export async function getGroup(groupId: string): Promise<Group | null> {
  return await invoke('get_group', { groupId });
}

/** List all groups */
export async function listGroups(): Promise<Group[]> {
  return await invoke('list_groups');
}

/** Get extended group info */
export async function getGroupInfo(groupId: string): Promise<GroupInfo | null> {
  return await invoke('get_group_info', { groupId });
}

/** Invite a member to a group */
export async function inviteMember(
  groupId: string,
  peerId: string | null,
  role: MemberRole
): Promise<ExportedInvitation> {
  return await invoke('invite_member', { groupId, peerId, role });
}

/** Accept a group invitation */
export async function acceptInvitation(
  invitation: ExportedInvitation
): Promise<GroupMember> {
  return await invoke('accept_invitation', { invitation });
}

/** Remove a member from a group */
export async function removeMember(
  groupId: string,
  peerId: string
): Promise<void> {
  await invoke('remove_member', { groupId, peerId });
}

/** Set a member's role */
export async function setMemberRole(
  groupId: string,
  peerId: string,
  role: MemberRole
): Promise<void> {
  await invoke('set_member_role', { groupId, peerId, role });
}

/** List members of a group */
export async function listMembers(groupId: string): Promise<GroupMember[]> {
  return await invoke('list_members', { groupId });
}

// =============================================================================
// File Commands
// =============================================================================

/** Upload a file to a group */
export async function uploadFile(
  groupId: string,
  path: string,
  data: Uint8Array
): Promise<SharedFile> {
  return await invoke('upload_file', {
    groupId,
    path,
    data: Array.from(data),
  });
}

/** Download a file */
export async function downloadFile(fileId: string): Promise<Uint8Array> {
  const data = await invoke<number[]>('download_file', { fileId });
  return new Uint8Array(data);
}

/** Delete a file */
export async function deleteFile(fileId: string): Promise<void> {
  await invoke('delete_file', { fileId });
}

/** List files in a group */
export async function listFiles(groupId: string): Promise<SharedFile[]> {
  return await invoke('list_files', { groupId });
}

/** Search files in a group */
export async function searchFiles(
  groupId: string,
  query: string
): Promise<SharedFile[]> {
  return await invoke('search_files', { groupId, query });
}

// =============================================================================
// Version Commands
// =============================================================================

/** Get file versions */
export async function getFileVersions(fileId: string): Promise<VersionInfo[]> {
  return await invoke('get_file_versions', { fileId });
}

/** Restore a file to a previous version */
export async function restoreVersion(
  fileId: string,
  version: number
): Promise<SharedFile> {
  return await invoke('restore_version', { fileId, version });
}

/** Get version summary for a file */
export async function getVersionSummary(fileId: string): Promise<VersionSummary> {
  return await invoke('get_version_summary', { fileId });
}

// =============================================================================
// Activity Commands
// =============================================================================

/** Get activity log for a group */
export async function getActivityLog(
  groupId: string,
  limit: number,
  offset: number
): Promise<ActivityInfo[]> {
  return await invoke('get_activity_log', { groupId, limit, offset });
}

/** Get recent activity across all groups */
export async function getRecentActivity(limit: number): Promise<ActivityInfo[]> {
  return await invoke('get_recent_activity', { limit });
}

/** Search activity log */
export async function searchActivity(
  groupId: string,
  query: string,
  limit: number
): Promise<ActivityInfo[]> {
  return await invoke('search_activity', { groupId, query, limit });
}

/** Get activity statistics for a group */
export async function getActivityStats(groupId: string): Promise<ActivityStats> {
  return await invoke('get_activity_stats', { groupId });
}

// =============================================================================
// Share Link Commands
// =============================================================================

/** Create a share link */
export async function createShareLink(
  fileId: string,
  expiresInHours?: number,
  password?: string,
  maxDownloads?: number
): Promise<ShareLinkInfo> {
  return await invoke('create_share_link', {
    fileId,
    expiresInHours,
    password,
    maxDownloads,
  });
}

/** Get a share link by ID */
export async function getShareLink(linkId: string): Promise<ShareLinkInfo | null> {
  return await invoke('get_share_link', { linkId });
}

/** Revoke a share link */
export async function revokeShareLink(linkId: string): Promise<void> {
  await invoke('revoke_share_link', { linkId });
}

/** Download via share link */
export async function downloadViaLink(
  linkId: string,
  password?: string
): Promise<Uint8Array> {
  const data = await invoke<number[]>('download_via_link', { linkId, password });
  return new Uint8Array(data);
}

/** List share links for a file */
export async function listFileShareLinks(
  fileId: string
): Promise<ShareLinkInfo[]> {
  return await invoke('list_file_share_links', { fileId });
}

/** Check if a share link requires a password */
export async function linkRequiresPassword(linkId: string): Promise<boolean> {
  return await invoke('link_requires_password', { linkId });
}

// =============================================================================
// Identity Commands
// =============================================================================

/** Get local peer ID */
export async function getPeerId(): Promise<string> {
  return await invoke('get_peer_id');
}

/** Get display name */
export async function getDisplayName(): Promise<string> {
  return await invoke('get_display_name');
}

/** Set display name */
export async function setDisplayName(name: string): Promise<void> {
  await invoke('set_display_name', { name });
}
