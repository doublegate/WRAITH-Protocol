// WRAITH Transfer - Transfer State Store

import { create } from 'zustand';
import type { TransferInfo } from '../types';
import * as api from '../lib/tauri';

interface TransferState {
  transfers: TransferInfo[];
  loading: boolean;
  error: string | null;

  // Actions
  fetchTransfers: () => Promise<void>;
  sendFile: (peerId: string, filePath: string) => Promise<string | null>;
  cancelTransfer: (transferId: string) => Promise<void>;
  clearError: () => void;
}

export const useTransferStore = create<TransferState>((set, get) => ({
  transfers: [],
  loading: false,
  error: null,

  fetchTransfers: async () => {
    try {
      const transfers = await api.getTransfers();
      set({ transfers, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  sendFile: async (peerId: string, filePath: string) => {
    set({ loading: true, error: null });
    try {
      const transferId = await api.sendFile(peerId, filePath);
      // Refresh transfers list
      await get().fetchTransfers();
      set({ loading: false });
      return transferId;
    } catch (e) {
      set({ loading: false, error: String(e) });
      return null;
    }
  },

  cancelTransfer: async (transferId: string) => {
    try {
      await api.cancelTransfer(transferId);
      // Remove from local state
      set(state => ({
        transfers: state.transfers.filter(t => t.id !== transferId)
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
