// WRAITH Transfer - Session State Store

import { create } from 'zustand';
import type { SessionInfo } from '../types';
import * as api from '../lib/tauri';

interface SessionState {
  sessions: SessionInfo[];
  loading: boolean;
  error: string | null;

  // Actions
  fetchSessions: () => Promise<void>;
  closeSession: (peerId: string) => Promise<void>;
  clearError: () => void;
}

export const useSessionStore = create<SessionState>((set) => ({
  sessions: [],
  loading: false,
  error: null,

  fetchSessions: async () => {
    try {
      const sessions = await api.getSessions();
      set({ sessions, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  closeSession: async (peerId: string) => {
    try {
      await api.closeSession(peerId);
      // Remove from local state
      set(state => ({
        sessions: state.sessions.filter(s => s.peer_id !== peerId)
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
