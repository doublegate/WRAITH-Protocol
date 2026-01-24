// WRAITH Recon - Audit Store

import { create } from 'zustand';
import * as tauri from '../lib/tauri';
import type { AuditEntry, ChainVerificationResult, DatabaseStats } from '../types';

interface AuditState {
  // State
  entries: AuditEntry[];
  lastSequence: number;
  chainValid: boolean | null;
  chainVerificationResult: ChainVerificationResult | null;
  databaseStats: DatabaseStats | null;
  loading: boolean;
  error: string | null;

  // Actions
  fetchEntries: (sinceSequence?: number, limit?: number) => Promise<void>;
  verifyChain: () => Promise<ChainVerificationResult>;
  exportLog: () => Promise<string>;
  addNote: (note: string) => Promise<void>;
  fetchDatabaseStats: () => Promise<void>;
  clearError: () => void;
}

export const useAuditStore = create<AuditState>((set, get) => ({
  // Initial state
  entries: [],
  lastSequence: 0,
  chainValid: null,
  chainVerificationResult: null,
  databaseStats: null,
  loading: false,
  error: null,

  // Fetch audit entries
  fetchEntries: async (sinceSequence: number = 0, limit: number = 100) => {
    set({ loading: true, error: null });
    try {
      const entries = await tauri.getAuditEntries(sinceSequence, limit);
      const lastSeq = entries.length > 0
        ? Math.max(...entries.map(e => e.sequence))
        : get().lastSequence;
      set({
        entries: sinceSequence === 0 ? entries : [...get().entries, ...entries],
        lastSequence: lastSeq,
        loading: false,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Verify audit chain integrity
  verifyChain: async () => {
    set({ loading: true, error: null });
    try {
      const result = await tauri.verifyAuditChain();
      set({
        chainValid: result.valid,
        chainVerificationResult: result,
        loading: false,
      });
      return result;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Export audit log
  exportLog: async () => {
    set({ loading: true, error: null });
    try {
      const exportPath = await tauri.exportAuditLog();
      set({ loading: false });
      return exportPath;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Add operator note to audit log
  addNote: async (note: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.addAuditNote(note);
      // Refresh entries after adding note
      await get().fetchEntries(get().lastSequence + 1, 10);
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Fetch database statistics
  fetchDatabaseStats: async () => {
    try {
      const stats = await tauri.getDatabaseStats();
      set({ databaseStats: stats });
    } catch {
      // Stats might not be available
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
