// Tauri IPC Bindings

import { invoke } from '@tauri-apps/api/core';
import type {
  Contact,
  Conversation,
  Message,
  NodeStatus,
  CallInfo,
  AudioDevice,
  GroupInfo,
  GroupMember,
} from '../types';

// Contact Commands

export async function createContact(
  peerId: string,
  displayName: string | null,
  identityKey: number[]
): Promise<number> {
  return await invoke('create_contact', { peerId, displayName, identityKey });
}

export async function getContact(peerId: string): Promise<Contact | null> {
  return await invoke('get_contact', { peerId });
}

export async function listContacts(): Promise<Contact[]> {
  return await invoke('list_contacts');
}

// Conversation Commands

export async function createConversation(
  convType: string,
  peerId: string | null,
  groupId: string | null,
  displayName: string | null
): Promise<number> {
  return await invoke('create_conversation', {
    convType,
    peerId,
    groupId,
    displayName,
  });
}

export async function getConversation(id: number): Promise<Conversation | null> {
  return await invoke('get_conversation', { id });
}

export async function listConversations(): Promise<Conversation[]> {
  return await invoke('list_conversations');
}

// Message Commands

export async function sendMessage(
  conversationId: number,
  peerId: string,
  body: string
): Promise<number> {
  return await invoke('send_message', { conversationId, peerId, body });
}

export async function getMessages(
  conversationId: number,
  limit: number = 50,
  offset: number = 0
): Promise<Message[]> {
  return await invoke('get_messages', { conversationId, limit, offset });
}

export async function markAsRead(conversationId: number): Promise<void> {
  await invoke('mark_as_read', { conversationId });
}

// Node Commands

export async function startNode(listenAddr: string = '0.0.0.0:0'): Promise<void> {
  await invoke('start_node', { listenAddr });
}

export async function getNodeStatus(): Promise<NodeStatus> {
  return await invoke('get_node_status');
}

// Voice Call Commands (Sprint 17.5)

export async function startCall(peerId: string): Promise<CallInfo> {
  return await invoke('start_call', { peerId });
}

export async function answerCall(callId: string): Promise<CallInfo> {
  return await invoke('answer_call', { callId });
}

export async function rejectCall(callId: string, reason?: string): Promise<void> {
  await invoke('reject_call', { callId, reason });
}

export async function endCall(callId: string, reason?: string): Promise<void> {
  await invoke('end_call', { callId, reason });
}

export async function toggleMute(callId: string): Promise<boolean> {
  return await invoke('toggle_mute', { callId });
}

export async function toggleSpeaker(callId: string): Promise<boolean> {
  return await invoke('toggle_speaker', { callId });
}

export async function getCallInfo(callId: string): Promise<CallInfo | null> {
  return await invoke('get_call_info', { callId });
}

export async function getActiveCalls(): Promise<CallInfo[]> {
  return await invoke('get_active_calls');
}

export async function listAudioInputDevices(): Promise<AudioDevice[]> {
  return await invoke('list_audio_input_devices');
}

export async function listAudioOutputDevices(): Promise<AudioDevice[]> {
  return await invoke('list_audio_output_devices');
}

export async function setAudioInputDevice(deviceId: string | null): Promise<void> {
  await invoke('set_audio_input_device', { deviceId });
}

export async function setAudioOutputDevice(deviceId: string | null): Promise<void> {
  await invoke('set_audio_output_device', { deviceId });
}

// Group Messaging Commands (Sprint 17.7)

export async function createGroup(
  name: string,
  memberPeerIds?: string[]
): Promise<GroupInfo> {
  return await invoke('create_group', { name, memberPeerIds });
}

export async function getGroupInfo(groupId: string): Promise<GroupInfo | null> {
  return await invoke('get_group_info', { groupId });
}

export async function updateGroupSettings(
  groupId: string,
  name?: string,
  description?: string,
  avatar?: number[]
): Promise<GroupInfo> {
  return await invoke('update_group_settings', {
    groupId,
    name,
    description,
    avatar,
  });
}

export async function addGroupMember(
  groupId: string,
  peerId: string,
  displayName?: string
): Promise<GroupMember> {
  return await invoke('add_group_member', { groupId, peerId, displayName });
}

export async function removeGroupMember(
  groupId: string,
  peerId: string
): Promise<void> {
  await invoke('remove_group_member', { groupId, peerId });
}

export async function leaveGroup(groupId: string): Promise<void> {
  await invoke('leave_group', { groupId });
}

export async function promoteToAdmin(
  groupId: string,
  peerId: string
): Promise<void> {
  await invoke('promote_to_admin', { groupId, peerId });
}

export async function demoteFromAdmin(
  groupId: string,
  peerId: string
): Promise<void> {
  await invoke('demote_from_admin', { groupId, peerId });
}

export async function sendGroupMessage(
  groupId: string,
  body: string
): Promise<number> {
  return await invoke('send_group_message', { groupId, body });
}

export async function getGroupMembers(groupId: string): Promise<GroupMember[]> {
  return await invoke('get_group_members', { groupId });
}

export async function rotateGroupKeys(groupId: string): Promise<void> {
  await invoke('rotate_group_keys', { groupId });
}
