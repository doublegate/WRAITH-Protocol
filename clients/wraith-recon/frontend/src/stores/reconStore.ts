// WRAITH Recon - Reconnaissance Store

import { create } from 'zustand';
import * as tauri from '../lib/tauri';
import type {
  PassiveReconStats,
  NetworkAsset,
  ActiveScanConfig,
  ActiveScanProgress,
  ProbeResult,
} from '../types';

interface ReconState {
  // Passive recon state
  passiveStats: PassiveReconStats | null;
  discoveredAssets: NetworkAsset[];
  passiveRunning: boolean;

  // Active scan state
  activeScanProgress: ActiveScanProgress | null;
  activeScanResults: ProbeResult[];
  activeRunning: boolean;

  // Loading states
  loading: boolean;
  error: string | null;

  // Passive recon actions
  startPassiveRecon: (interfaceName?: string, timeoutSecs?: number) => Promise<string>;
  stopPassiveRecon: () => Promise<void>;
  fetchPassiveStats: () => Promise<void>;
  fetchDiscoveredAssets: () => Promise<void>;

  // Active scan actions
  startActiveScan: (config: ActiveScanConfig) => Promise<string>;
  stopActiveScan: () => Promise<void>;
  fetchActiveScanProgress: () => Promise<void>;
  fetchActiveScanResults: () => Promise<void>;

  // Common actions
  clearError: () => void;
}

export const useReconStore = create<ReconState>((set, get) => ({
  // Initial state
  passiveStats: null,
  discoveredAssets: [],
  passiveRunning: false,
  activeScanProgress: null,
  activeScanResults: [],
  activeRunning: false,
  loading: false,
  error: null,

  // Start passive reconnaissance
  startPassiveRecon: async (interfaceName?: string, timeoutSecs?: number) => {
    set({ loading: true, error: null });
    try {
      const sessionId = await tauri.startPassiveRecon(interfaceName, timeoutSecs);
      set({ passiveRunning: true, loading: false });
      return sessionId;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Stop passive reconnaissance
  stopPassiveRecon: async () => {
    set({ loading: true, error: null });
    try {
      const finalStats = await tauri.stopPassiveRecon();
      set({ passiveStats: finalStats, passiveRunning: false, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Fetch passive recon statistics
  fetchPassiveStats: async () => {
    try {
      const stats = await tauri.getPassiveReconStats();
      set({ passiveStats: stats, passiveRunning: stats.is_running });
    } catch {
      // Stats might not be available if not running
    }
  },

  // Fetch discovered network assets
  fetchDiscoveredAssets: async () => {
    try {
      const assets = await tauri.getDiscoveredAssets();
      set({ discoveredAssets: assets });
    } catch {
      // Assets might not be available
    }
  },

  // Start active scan
  startActiveScan: async (config: ActiveScanConfig) => {
    set({ loading: true, error: null });
    try {
      const scanId = await tauri.startActiveScan(config);
      set({ activeRunning: true, loading: false });
      return scanId;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Stop active scan
  stopActiveScan: async () => {
    set({ loading: true, error: null });
    try {
      await tauri.stopActiveScan();
      set({ activeRunning: false, loading: false });
      await get().fetchActiveScanProgress();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Fetch active scan progress
  fetchActiveScanProgress: async () => {
    try {
      const progress = await tauri.getActiveScanProgress();
      set({
        activeScanProgress: progress,
        activeRunning: progress?.status === 'Running',
      });
    } catch {
      // Progress might not be available
    }
  },

  // Fetch active scan results
  fetchActiveScanResults: async () => {
    try {
      const results = await tauri.getActiveScanResults();
      set({ activeScanResults: results });
    } catch {
      // Results might not be available
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
