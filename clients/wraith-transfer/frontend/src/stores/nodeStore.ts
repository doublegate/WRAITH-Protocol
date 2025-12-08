// WRAITH Transfer - Node State Store

import { create } from 'zustand';
import type { NodeStatus } from '../types';
import * as api from '../lib/tauri';

interface NodeState {
  status: NodeStatus | null;
  loading: boolean;
  error: string | null;

  // Actions
  fetchStatus: () => Promise<void>;
  startNode: () => Promise<void>;
  stopNode: () => Promise<void>;
  clearError: () => void;
}

export const useNodeStore = create<NodeState>((set) => ({
  status: null,
  loading: false,
  error: null,

  fetchStatus: async () => {
    try {
      const status = await api.getNodeStatus();
      set({ status, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  startNode: async () => {
    set({ loading: true, error: null });
    try {
      const status = await api.startNode();
      set({ status, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  stopNode: async () => {
    set({ loading: true, error: null });
    try {
      await api.stopNode();
      set({
        status: { running: false, node_id: null, active_sessions: 0, active_transfers: 0 },
        loading: false
      });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
