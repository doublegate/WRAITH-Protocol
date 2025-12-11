// Tauri IPC Bindings

import { invoke } from '@tauri-apps/api/core';
import type { Contact, Conversation, Message, NodeStatus } from '../types';

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
