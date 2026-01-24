// WRAITH Recon - Node Store

import { create } from 'zustand';
import * as tauri from '../lib/tauri';
import type { NodeStatus, StatisticsSummary } from '../types';

interface NodeState {
  // State
  status: NodeStatus | null;
  peerId: string | null;
  statistics: StatisticsSummary | null;
  loading: boolean;
  error: string | null;

  // Actions
  startNode: () => Promise<void>;
  stopNode: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  fetchPeerId: () => Promise<void>;
  fetchStatistics: () => Promise<void>;
  clearError: () => void;
}

export const useNodeStore = create<NodeState>((set) => ({
  // Initial state
  status: null,
  peerId: null,
  statistics: null,
  loading: false,
  error: null,

  // Start node
  startNode: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.startNode();
      const status = await tauri.getNodeStatus();
      const peerId = await tauri.getPeerId();
      set({ status, peerId, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Stop node
  stopNode: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.stopNode();
      const status = await tauri.getNodeStatus();
      set({ status, peerId: null, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Fetch node status
  fetchStatus: async () => {
    try {
      const status = await tauri.getNodeStatus();
      set({ status });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch peer ID
  fetchPeerId: async () => {
    try {
      const peerId = await tauri.getPeerId();
      set({ peerId });
    } catch {
      // Peer ID might not be available if node not running
    }
  },

  // Fetch statistics
  fetchStatistics: async () => {
    try {
      const statistics = await tauri.getStatistics();
      set({ statistics });
    } catch {
      // Statistics might not be available
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
