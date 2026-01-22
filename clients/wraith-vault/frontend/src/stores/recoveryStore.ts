// Recovery Store (Zustand) for WRAITH Vault

import { create } from "zustand";
import type {
  RecoveryProgress,
  RecoveryResult,
  EncryptedShard,
} from "../types";
import * as tauri from "../lib/tauri";

interface RecoveryState {
  activeSessionId: string | null;
  progress: RecoveryProgress | null;
  result: RecoveryResult | null;
  activeSessions: string[];
  loading: boolean;
  error: string | null;

  // Actions
  startRecovery: (secretId: string) => Promise<string>;
  addShard: (
    sessionId: string,
    shard: EncryptedShard,
    encryptionKey: string
  ) => Promise<RecoveryProgress>;
  completeRecovery: (sessionId: string) => Promise<RecoveryResult>;
  getProgress: (sessionId: string) => Promise<RecoveryProgress>;
  cancelRecovery: (sessionId: string) => Promise<void>;
  loadActiveSessions: () => Promise<void>;
  clearResult: () => void;
  clearError: () => void;
}

export const useRecoveryStore = create<RecoveryState>((set, get) => ({
  activeSessionId: null,
  progress: null,
  result: null,
  activeSessions: [],
  loading: false,
  error: null,

  startRecovery: async (secretId: string) => {
    set({ loading: true, error: null, result: null });
    try {
      const sessionId = await tauri.startRecovery(secretId);
      set({ activeSessionId: sessionId, loading: false });
      await get().loadActiveSessions();
      return sessionId;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  addShard: async (
    sessionId: string,
    shard: EncryptedShard,
    encryptionKey: string
  ) => {
    set({ loading: true, error: null });
    try {
      const progress = await tauri.addRecoveryShard(
        sessionId,
        shard,
        encryptionKey
      );
      set({ progress, loading: false });
      return progress;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  completeRecovery: async (sessionId: string) => {
    set({ loading: true, error: null });
    try {
      const result = await tauri.completeRecovery(sessionId);
      set({
        result,
        activeSessionId: null,
        progress: null,
        loading: false,
      });
      await get().loadActiveSessions();
      return result;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  getProgress: async (sessionId: string) => {
    try {
      const progress = await tauri.getRecoveryProgress(sessionId);
      set({ progress });
      return progress;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  cancelRecovery: async (sessionId: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.cancelRecovery(sessionId);
      if (get().activeSessionId === sessionId) {
        set({ activeSessionId: null, progress: null });
      }
      await get().loadActiveSessions();
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  loadActiveSessions: async () => {
    try {
      const activeSessions = await tauri.listRecoverySessions();
      set({ activeSessions });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  clearResult: () => {
    set({ result: null });
  },

  clearError: () => {
    set({ error: null });
  },
}));
