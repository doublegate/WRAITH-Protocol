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
  conv_type: "direct" | "group";
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
  content_type: "text" | "media" | "voice" | "file";
  body?: string;
  media_path?: string;
  media_mime_type?: string;
  media_size?: number;
  timestamp: number;
  sent: boolean;
  delivered: boolean;
  read_by_me: boolean;
  expires_at?: number;
  direction: "incoming" | "outgoing";
}

export interface NodeStatus {
  running: boolean;
  local_peer_id: string;
  session_count: number;
  active_conversations: number;
}

// Voice Call Types (Sprint 17.5)
export interface CallInfo {
  call_id: string;
  peer_id: string;
  state: CallState;
  direction: "outgoing" | "incoming";
  started_at: number;
  connected_at?: number;
  muted: boolean;
  speaker_on: boolean;
  stats: CallStats;
}

export type CallState =
  | "initiating"
  | "ringing"
  | "incoming"
  | "connected"
  | "on_hold"
  | "reconnecting"
  | "ended";

export interface CallStats {
  duration_secs: number;
  packets_sent: number;
  packets_received: number;
  packets_lost: number;
  avg_latency_ms: number;
  jitter_ms: number;
  current_bitrate: number;
}

export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

// Group Messaging Types (Sprint 17.7)
export interface GroupInfo {
  group_id: string;
  name: string;
  description?: string;
  member_count: number;
  created_at: number;
  am_i_admin: boolean;
}

export interface GroupMember {
  peer_id: string;
  display_name?: string;
  role: "admin" | "member";
  joined_at: number;
  key_generation: number;
}
