// WRAITH Recon - Engagement Store

import { create } from 'zustand';
import * as tauri from '../lib/tauri';
import type {
  EngagementStatus,
  EngagementStatusResponse,
  RulesOfEngagement,
  ScopeSummary,
  KillSwitchState,
  TimingInfo,
} from '../types';

interface EngagementState {
  // State
  status: EngagementStatus;
  engagementId: string | null;
  roe: RulesOfEngagement | null;
  scopeSummary: ScopeSummary | null;
  killSwitchState: KillSwitchState | null;
  timingInfo: TimingInfo | null;
  operatorId: string;
  loading: boolean;
  error: string | null;

  // Actions
  loadRoe: (roe: RulesOfEngagement) => Promise<void>;
  loadRoeFromFile: (path: string) => Promise<void>;
  startEngagement: () => Promise<string>;
  stopEngagement: (reason: string) => Promise<void>;
  pauseEngagement: () => Promise<void>;
  resumeEngagement: () => Promise<void>;
  activateKillSwitch: (reason: string) => Promise<void>;
  fetchStatus: () => Promise<void>;
  fetchScopeSummary: () => Promise<void>;
  fetchKillSwitchState: () => Promise<void>;
  fetchTimingInfo: () => Promise<void>;
  setOperatorId: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useEngagementStore = create<EngagementState>((set, get) => ({
  // Initial state
  status: 'NotLoaded',
  engagementId: null,
  roe: null,
  scopeSummary: null,
  killSwitchState: null,
  timingInfo: null,
  operatorId: '',
  loading: false,
  error: null,

  // Load RoE from object
  loadRoe: async (roe: RulesOfEngagement) => {
    set({ loading: true, error: null });
    try {
      await tauri.loadRoe(roe);
      set({ roe, status: 'Ready', loading: false });
      await get().fetchScopeSummary();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Load RoE from file
  loadRoeFromFile: async (path: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.loadRoeFile(path);
      const roe = await tauri.getRoe();
      set({ roe, status: 'Ready', loading: false });
      await get().fetchScopeSummary();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Start engagement
  startEngagement: async () => {
    set({ loading: true, error: null });
    try {
      const engagementId = await tauri.startEngagement();
      set({ engagementId, status: 'Active', loading: false });
      return engagementId;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Stop engagement
  stopEngagement: async (reason: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.stopEngagement(reason);
      set({ status: 'Completed', loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Pause engagement
  pauseEngagement: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.pauseEngagement();
      set({ status: 'Paused', loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Resume engagement
  resumeEngagement: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.resumeEngagement();
      set({ status: 'Active', loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Activate kill switch
  activateKillSwitch: async (reason: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.activateKillSwitch(reason);
      set({ status: 'Terminated', loading: false });
      await get().fetchKillSwitchState();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Fetch current status
  fetchStatus: async () => {
    try {
      const response: EngagementStatusResponse = await tauri.getEngagementStatus();
      set({
        status: response.status,
        engagementId: response.engagement_id,
        operatorId: response.operator_id,
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch scope summary
  fetchScopeSummary: async () => {
    try {
      const scopeSummary = await tauri.getScopeSummary();
      set({ scopeSummary });
    } catch {
      // Scope might not be loaded yet
    }
  },

  // Fetch kill switch state
  fetchKillSwitchState: async () => {
    try {
      const killSwitchState = await tauri.isKillSwitchActive();
      set({ killSwitchState });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch timing info
  fetchTimingInfo: async () => {
    try {
      const timingInfo = await tauri.getTimingInfo();
      set({ timingInfo });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Set operator ID
  setOperatorId: async (id: string) => {
    try {
      await tauri.setOperatorId(id);
      set({ operatorId: id });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
