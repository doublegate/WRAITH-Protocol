// Stream types matching backend structures

export interface Stream {
  id: string;
  title: string;
  description: string | null;
  created_at: number;
  created_by: string;
  thumbnail_hash: string | null;
  duration: number | null;
  is_live: boolean;
  status: StreamStatus;
  view_count: number;
  category: string | null;
  tags: string | null;
}

export type StreamStatus = 'processing' | 'ready' | 'failed' | 'live';

export interface StreamInfo {
  id: string;
  title: string;
  description: string | null;
  created_at: number;
  created_by: string;
  thumbnail_url: string | null;
  duration: number | null;
  is_live: boolean;
  status: string;
  view_count: number;
  category: string | null;
  tags: string | null;
  qualities: string[];
}

export interface StreamQuality {
  name: string;
  width: number;
  height: number;
  bitrate: number;
}

export interface PlaybackInfo {
  stream_id: string;
  manifest_url: string;
  qualities: QualityInfo[];
  duration_secs: number | null;
  subtitle_languages: string[];
}

export interface QualityInfo {
  name: string;
  width: number;
  height: number;
  bitrate: number;
}

export interface TranscodeProgress {
  stream_id: string;
  progress: number;
  current_profile: string;
  status: TranscodeStatus;
}

export type TranscodeStatus = 'pending' | 'transcoding' | 'completed' | 'cancelled' | string;

export interface SubtitleCue {
  start_time: number;
  end_time: number;
  text: string;
}

export interface SubtitleInfo {
  language: string;
  label: string;
  format: string;
}

export interface StreamView {
  stream_id: string;
  peer_id: string;
  started_at: number;
  watch_time: number;
  last_position: number;
  quality: string | null;
}

export interface SearchResults {
  query: string;
  total: number;
  streams: StreamSummary[];
}

export interface StreamSummary {
  id: string;
  title: string;
  thumbnail_url: string | null;
  duration: number | null;
  is_live: boolean;
  view_count: number;
  created_by: string;
}

export interface Category {
  name: string;
  count: number;
}

// Player state
export interface PlayerState {
  stream_id: string | null;
  is_playing: boolean;
  is_buffering: boolean;
  current_time: number;
  duration: number;
  volume: number;
  is_muted: boolean;
  quality: string;
  available_qualities: string[];
  is_fullscreen: boolean;
  playback_rate: number;
}

// App view states
export type ViewState = 'browse' | 'player' | 'upload' | 'my-streams' | 'settings';

// Upload state
export interface UploadState {
  file: File | null;
  title: string;
  description: string;
  category: string;
  tags: string;
  status: 'idle' | 'uploading' | 'transcoding' | 'complete' | 'error';
  progress: number;
  streamId: string | null;
  error: string | null;
}
