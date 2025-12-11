// TypeScript types for WRAITH-Chat

export interface Contact {
  id: number;
  peer_id: string;
  display_name?: string;
  identity_key: number[];
  safety_number: string;
  verified: boolean;
  blocked: boolean;
  created_at: number;
  last_seen?: number;
}

export interface Conversation {
  id: number;
  conv_type: 'direct' | 'group';
  peer_id?: string;
  group_id?: string;
  display_name?: string;
  avatar?: number[];
  muted: boolean;
  archived: boolean;
  last_message_id?: number;
  last_message_at?: number;
  unread_count: number;
  expires_in?: number;
}

export interface Message {
  id: number;
  conversation_id: number;
  sender_peer_id: string;
  content_type: 'text' | 'media' | 'voice' | 'file';
  body?: string;
  media_path?: string;
  media_mime_type?: string;
  media_size?: number;
  timestamp: number;
  sent: boolean;
  delivered: boolean;
  read_by_me: boolean;
  expires_at?: number;
  direction: 'incoming' | 'outgoing';
}

export interface NodeStatus {
  running: boolean;
  local_peer_id: string;
  session_count: number;
  active_conversations: number;
}
