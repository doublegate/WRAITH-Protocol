// Voice Call Store (Zustand) - Sprint 17.5

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

// Types
export interface CallInfo {
  call_id: string;
  peer_id: string;
  state: CallState;
  direction: 'outgoing' | 'incoming';
  started_at: number;
  connected_at?: number;
  muted: boolean;
  speaker_on: boolean;
  stats: CallStats;
}

export type CallState =
  | 'initiating'
  | 'ringing'
  | 'incoming'
  | 'connected'
  | 'on_hold'
  | 'reconnecting'
  | 'ended';

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

interface CallStoreState {
  // Current call state
  activeCall: CallInfo | null;
  incomingCall: CallInfo | null;
  allCalls: CallInfo[];

  // Audio devices
  inputDevices: AudioDevice[];
  outputDevices: AudioDevice[];
  selectedInputDevice: string | null;
  selectedOutputDevice: string | null;

  // Loading states
  loading: boolean;
  error: string | null;

  // Actions
  startCall: (peerId: string) => Promise<CallInfo>;
  answerCall: (callId: string) => Promise<CallInfo>;
  rejectCall: (callId: string, reason?: string) => Promise<void>;
  endCall: (callId: string, reason?: string) => Promise<void>;
  toggleMute: (callId: string) => Promise<boolean>;
  toggleSpeaker: (callId: string) => Promise<boolean>;
  refreshCallInfo: (callId: string) => Promise<void>;
  refreshActiveCalls: () => Promise<void>;

  // Audio device actions
  loadAudioDevices: () => Promise<void>;
  setInputDevice: (deviceId: string | null) => Promise<void>;
  setOutputDevice: (deviceId: string | null) => Promise<void>;

  // Internal actions
  handleIncomingCall: (call: CallInfo) => void;
  clearIncomingCall: () => void;
  clearError: () => void;
}

export const useCallStore = create<CallStoreState>((set, get) => ({
  activeCall: null,
  incomingCall: null,
  allCalls: [],
  inputDevices: [],
  outputDevices: [],
  selectedInputDevice: null,
  selectedOutputDevice: null,
  loading: false,
  error: null,

  startCall: async (peerId: string) => {
    set({ loading: true, error: null });
    try {
      const call: CallInfo = await invoke('start_call', { peerId });
      set({ activeCall: call, loading: false });
      return call;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  answerCall: async (callId: string) => {
    set({ loading: true, error: null });
    try {
      const call: CallInfo = await invoke('answer_call', { callId });
      set({ activeCall: call, incomingCall: null, loading: false });
      return call;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  rejectCall: async (callId: string, reason?: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('reject_call', { callId, reason });
      set({ incomingCall: null, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  endCall: async (callId: string, reason?: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('end_call', { callId, reason });
      set({ activeCall: null, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  toggleMute: async (callId: string) => {
    try {
      const muted: boolean = await invoke('toggle_mute', { callId });
      const { activeCall } = get();
      if (activeCall && activeCall.call_id === callId) {
        set({ activeCall: { ...activeCall, muted } });
      }
      return muted;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  toggleSpeaker: async (callId: string) => {
    try {
      const speakerOn: boolean = await invoke('toggle_speaker', { callId });
      const { activeCall } = get();
      if (activeCall && activeCall.call_id === callId) {
        set({ activeCall: { ...activeCall, speaker_on: speakerOn } });
      }
      return speakerOn;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  refreshCallInfo: async (callId: string) => {
    try {
      const call: CallInfo | null = await invoke('get_call_info', { callId });
      const { activeCall } = get();
      if (call && activeCall && activeCall.call_id === callId) {
        set({ activeCall: call });
      }
      if (call && call.state === 'ended') {
        set({ activeCall: null });
      }
    } catch (error) {
      console.error('Failed to refresh call info:', error);
    }
  },

  refreshActiveCalls: async () => {
    try {
      const calls: CallInfo[] = await invoke('get_active_calls');
      set({ allCalls: calls });

      // Update active call if it exists in the list
      const { activeCall } = get();
      if (activeCall) {
        const updated = calls.find((c) => c.call_id === activeCall.call_id);
        if (updated) {
          set({ activeCall: updated });
        } else if (!calls.some((c) => c.call_id === activeCall.call_id)) {
          // Call ended
          set({ activeCall: null });
        }
      }
    } catch (error) {
      console.error('Failed to refresh active calls:', error);
    }
  },

  loadAudioDevices: async () => {
    try {
      const [inputDevices, outputDevices] = await Promise.all([
        invoke<AudioDevice[]>('list_audio_input_devices'),
        invoke<AudioDevice[]>('list_audio_output_devices'),
      ]);
      set({ inputDevices, outputDevices });
    } catch (error) {
      console.error('Failed to load audio devices:', error);
    }
  },

  setInputDevice: async (deviceId: string | null) => {
    try {
      await invoke('set_audio_input_device', { deviceId });
      set({ selectedInputDevice: deviceId });
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  setOutputDevice: async (deviceId: string | null) => {
    try {
      await invoke('set_audio_output_device', { deviceId });
      set({ selectedOutputDevice: deviceId });
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  handleIncomingCall: (call: CallInfo) => {
    set({ incomingCall: call });
  },

  clearIncomingCall: () => {
    set({ incomingCall: null });
  },

  clearError: () => {
    set({ error: null });
  },
}));

// Helper function to format call duration
export function formatCallDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

// Helper to get human-readable call state
export function getCallStateText(state: CallState): string {
  switch (state) {
    case 'initiating':
      return 'Connecting...';
    case 'ringing':
      return 'Ringing...';
    case 'incoming':
      return 'Incoming call';
    case 'connected':
      return 'Connected';
    case 'on_hold':
      return 'On hold';
    case 'reconnecting':
      return 'Reconnecting...';
    case 'ended':
      return 'Call ended';
    default:
      return state;
  }
}
