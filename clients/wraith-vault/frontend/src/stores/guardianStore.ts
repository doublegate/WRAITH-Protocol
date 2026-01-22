// Guardian Store (Zustand) for WRAITH Vault

import { create } from "zustand";
import type { Guardian, GuardianStatus } from "../types";
import * as tauri from "../lib/tauri";

interface GuardianState {
  guardians: Guardian[];
  selectedGuardian: Guardian | null;
  availableGuardians: Guardian[];
  loading: boolean;
  error: string | null;

  // Actions
  loadGuardians: () => Promise<void>;
  loadAvailableGuardians: () => Promise<void>;
  loadGuardiansByStatus: (status: GuardianStatus) => Promise<void>;
  addGuardian: (
    name: string,
    peerId: string,
    publicKey: string,
    notes: string | null
  ) => Promise<Guardian>;
  getGuardian: (guardianId: string) => Promise<Guardian | null>;
  updateGuardian: (guardian: Guardian) => Promise<void>;
  updateGuardianStatus: (
    guardianId: string,
    status: GuardianStatus
  ) => Promise<void>;
  removeGuardian: (guardianId: string) => Promise<Guardian>;
  recordHealthCheck: (
    guardianId: string,
    success: boolean,
    responseTimeMs: number | null,
    error: string | null
  ) => Promise<void>;
  selectGuardiansForDistribution: (count: number) => Promise<Guardian[]>;
  selectGuardian: (guardian: Guardian | null) => void;
  clearError: () => void;
}

export const useGuardianStore = create<GuardianState>((set, get) => ({
  guardians: [],
  selectedGuardian: null,
  availableGuardians: [],
  loading: false,
  error: null,

  loadGuardians: async () => {
    set({ loading: true, error: null });
    try {
      const guardians = await tauri.listGuardians();
      set({ guardians, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  loadAvailableGuardians: async () => {
    try {
      const availableGuardians = await tauri.listAvailableGuardians();
      set({ availableGuardians });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  loadGuardiansByStatus: async (status: GuardianStatus) => {
    set({ loading: true, error: null });
    try {
      const guardians = await tauri.listGuardiansByStatus(status);
      set({ guardians, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  addGuardian: async (
    name: string,
    peerId: string,
    publicKey: string,
    notes: string | null
  ) => {
    set({ loading: true, error: null });
    try {
      const guardian = await tauri.addGuardian(name, peerId, publicKey, notes);
      await get().loadGuardians();
      await get().loadAvailableGuardians();
      set({ loading: false });
      return guardian;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  getGuardian: async (guardianId: string) => {
    try {
      return await tauri.getGuardian(guardianId);
    } catch (error) {
      set({ error: (error as Error).message });
      return null;
    }
  },

  updateGuardian: async (guardian: Guardian) => {
    set({ loading: true, error: null });
    try {
      await tauri.updateGuardian(guardian);
      await get().loadGuardians();
      await get().loadAvailableGuardians();
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  updateGuardianStatus: async (
    guardianId: string,
    status: GuardianStatus
  ) => {
    set({ loading: true, error: null });
    try {
      await tauri.updateGuardianStatus(guardianId, status);
      await get().loadGuardians();
      await get().loadAvailableGuardians();
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  removeGuardian: async (guardianId: string) => {
    set({ loading: true, error: null });
    try {
      const guardian = await tauri.removeGuardian(guardianId);
      if (get().selectedGuardian?.id === guardianId) {
        set({ selectedGuardian: null });
      }
      await get().loadGuardians();
      await get().loadAvailableGuardians();
      set({ loading: false });
      return guardian;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  recordHealthCheck: async (
    guardianId: string,
    success: boolean,
    responseTimeMs: number | null,
    error: string | null
  ) => {
    try {
      await tauri.recordHealthCheck(guardianId, success, responseTimeMs, error);
      await get().loadGuardians();
      await get().loadAvailableGuardians();
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  selectGuardiansForDistribution: async (count: number) => {
    try {
      return await tauri.selectGuardiansForDistribution(count);
    } catch (error) {
      set({ error: (error as Error).message });
      return [];
    }
  },

  selectGuardian: (guardian: Guardian | null) => {
    set({ selectedGuardian: guardian });
  },

  clearError: () => {
    set({ error: null });
  },
}));
