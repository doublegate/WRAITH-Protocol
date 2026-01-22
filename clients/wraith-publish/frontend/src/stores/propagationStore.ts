import { create } from 'zustand';
import type { PropagationStatus, StorageStats } from '../types';
import * as api from '../lib/tauri';

interface PropagationState {
  // State
  activePropagations: PropagationStatus[];
  storageStats: StorageStats | null;
  pinnedCids: string[];
  loading: boolean;
  error: string | null;

  // Actions
  fetchActivePropagations: () => Promise<void>;
  fetchStorageStats: () => Promise<void>;
  fetchPinnedCids: () => Promise<void>;
  getPropagationStatus: (cid: string) => Promise<PropagationStatus | null>;
  pinContent: (cid: string) => Promise<boolean>;
  unpinContent: (cid: string) => Promise<boolean>;
  clearError: () => void;
}

export const usePropagationStore = create<PropagationState>((set, get) => ({
  // Initial state
  activePropagations: [],
  storageStats: null,
  pinnedCids: [],
  loading: false,
  error: null,

  // Fetch active propagations
  fetchActivePropagations: async () => {
    set({ loading: true, error: null });
    try {
      const propagations = await api.listActivePropagations();
      set({ activePropagations: propagations, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Fetch storage stats
  fetchStorageStats: async () => {
    try {
      const stats = await api.getStorageStats();
      set({ storageStats: stats });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch pinned CIDs
  fetchPinnedCids: async () => {
    try {
      const cids = await api.listPinned();
      set({ pinnedCids: cids });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Get propagation status for a specific CID
  getPropagationStatus: async (cid) => {
    try {
      return await api.getPropagationStatus(cid);
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  // Pin content
  pinContent: async (cid) => {
    try {
      const result = await api.pinContent(cid);
      if (result) {
        set((state) => ({
          pinnedCids: [...state.pinnedCids, cid],
        }));
      }
      await get().fetchStorageStats();
      return result;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  // Unpin content
  unpinContent: async (cid) => {
    try {
      const result = await api.unpinContent(cid);
      if (result) {
        set((state) => ({
          pinnedCids: state.pinnedCids.filter((c) => c !== cid),
        }));
      }
      await get().fetchStorageStats();
      return result;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
