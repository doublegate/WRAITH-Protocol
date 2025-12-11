// Node Store (Zustand)

import { create } from 'zustand';
import type { NodeStatus } from '../types';
import * as tauri from '../lib/tauri';

interface NodeState {
  status: NodeStatus | null;
  loading: boolean;
  error: string | null;

  startNode: (listenAddr?: string) => Promise<void>;
  refreshStatus: () => Promise<void>;
}

export const useNodeStore = create<NodeState>((set) => ({
  status: null,
  loading: false,
  error: null,

  startNode: async (listenAddr = '0.0.0.0:0') => {
    set({ loading: true, error: null });
    try {
      await tauri.startNode(listenAddr);
      const status = await tauri.getNodeStatus();
      set({ status, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  refreshStatus: async () => {
    try {
      const status = await tauri.getNodeStatus();
      set({ status });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },
}));
