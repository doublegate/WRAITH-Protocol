// Node Store (Zustand) for WRAITH Vault

import { create } from "zustand";
import type { NodeStatus, VaultStats, RuntimeStatistics } from "../types";
import * as tauri from "../lib/tauri";

interface NodeState {
  status: NodeStatus;
  vaultStats: VaultStats | null;
  runtimeStats: RuntimeStatistics | null;
  loading: boolean;
  error: string | null;

  // Actions
  loadStatus: () => Promise<void>;
  loadVaultStats: () => Promise<void>;
  loadRuntimeStats: () => Promise<void>;
  startNode: () => Promise<void>;
  stopNode: () => Promise<void>;
  clearError: () => void;
}

export const useNodeStore = create<NodeState>((set) => ({
  status: {
    running: false,
    peer_id: null,
    active_routes: 0,
  },
  vaultStats: null,
  runtimeStats: null,
  loading: false,
  error: null,

  loadStatus: async () => {
    try {
      const status = await tauri.getNodeStatus();
      set({ status });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  loadVaultStats: async () => {
    try {
      const vaultStats = await tauri.getVaultStats();
      set({ vaultStats });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  loadRuntimeStats: async () => {
    try {
      const runtimeStats = await tauri.getRuntimeStatistics();
      set({ runtimeStats });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  startNode: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.startNode();
      const status = await tauri.getNodeStatus();
      set({ status, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  stopNode: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.stopNode();
      const status = await tauri.getNodeStatus();
      set({ status, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  clearError: () => {
    set({ error: null });
  },
}));
