import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

export type TabId = 'graph' | 'stats' | 'dht' | 'diagnostics';

interface UiState {
  // Active tab
  activeTab: TabId;

  // Monitoring state
  monitoringActive: boolean;
  monitorInterval: number;

  // Graph settings
  showLabels: boolean;
  showIndirectPeers: boolean;
  graphLayout: 'force' | 'radial' | 'tree';

  // Local peer ID
  localPeerId: string | null;

  // Actions
  setActiveTab: (tab: TabId) => void;
  setMonitoringActive: (active: boolean) => void;
  setMonitorInterval: (interval: number) => void;
  setShowLabels: (show: boolean) => void;
  setShowIndirectPeers: (show: boolean) => void;
  setGraphLayout: (layout: 'force' | 'radial' | 'tree') => void;
  fetchLocalPeerId: () => Promise<void>;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      // Initial state
      activeTab: 'graph',
      monitoringActive: true,
      monitorInterval: 1000,
      showLabels: true,
      showIndirectPeers: true,
      graphLayout: 'force',
      localPeerId: null,

      // Actions
      setActiveTab: (tab) => set({ activeTab: tab }),

      setMonitoringActive: async (active) => {
        try {
          await invoke('set_monitoring_active', { active });
          set({ monitoringActive: active });
        } catch (e) {
          console.error('Failed to set monitoring state:', e);
        }
      },

      setMonitorInterval: async (interval) => {
        try {
          await invoke('set_monitor_interval', { intervalMs: interval });
          set({ monitorInterval: interval });
        } catch (e) {
          console.error('Failed to set monitor interval:', e);
        }
      },

      setShowLabels: (show) => set({ showLabels: show }),
      setShowIndirectPeers: (show) => set({ showIndirectPeers: show }),
      setGraphLayout: (layout) => set({ graphLayout: layout }),

      fetchLocalPeerId: async () => {
        try {
          const peerId = await invoke<string | null>('get_peer_id');
          set({ localPeerId: peerId });
        } catch (e) {
          console.error('Failed to fetch local peer ID:', e);
        }
      },
    }),
    {
      name: 'wraith-mesh-ui',
      partialize: (state) => ({
        showLabels: state.showLabels,
        showIndirectPeers: state.showIndirectPeers,
        graphLayout: state.graphLayout,
        monitorInterval: state.monitorInterval,
      }),
    }
  )
);
