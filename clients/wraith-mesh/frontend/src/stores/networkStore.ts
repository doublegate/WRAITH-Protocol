import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type {
  NetworkSnapshot,
  MetricsEntry,
  RoutingBucket,
  LookupResult,
  StoredKey,
  PingResult,
  BandwidthResult,
  HealthReport,
  NatDetectionResult,
} from '../types';

interface NetworkState {
  // Network snapshot
  snapshot: NetworkSnapshot | null;
  metricsHistory: MetricsEntry[];
  loading: boolean;
  error: string | null;

  // DHT data
  routingTable: RoutingBucket[] | null;
  lookupResult: LookupResult | null;
  storedKeys: StoredKey[] | null;

  // Diagnostic results
  lastPingResult: PingResult | null;
  lastBandwidthResult: BandwidthResult | null;
  lastHealthReport: HealthReport | null;
  natResult: NatDetectionResult | null;

  // Selected peer for details
  selectedPeerId: string | null;

  // Actions
  fetchSnapshot: () => Promise<void>;
  fetchMetricsHistory: (limit?: number) => Promise<void>;
  fetchRoutingTable: () => Promise<void>;
  getStoredKeys: () => Promise<void>;
  lookupKey: (key: string) => Promise<void>;
  pingPeer: (peerId: string, count?: number) => Promise<PingResult>;
  testBandwidth: (peerId: string, size?: number) => Promise<BandwidthResult>;
  checkHealth: (peerId: string) => Promise<HealthReport>;
  detectNat: () => Promise<NatDetectionResult>;
  exportData: (format: 'json' | 'csv') => Promise<string>;
  exportMetrics: (limit?: number) => Promise<string>;
  setSelectedPeer: (peerId: string | null) => void;
  clearError: () => void;
}

export const useNetworkStore = create<NetworkState>((set) => ({
  // Initial state
  snapshot: null,
  metricsHistory: [],
  loading: false,
  error: null,
  routingTable: null,
  lookupResult: null,
  storedKeys: null,
  lastPingResult: null,
  lastBandwidthResult: null,
  lastHealthReport: null,
  natResult: null,
  selectedPeerId: null,

  // Fetch current network snapshot
  fetchSnapshot: async () => {
    try {
      const snapshot = await invoke<NetworkSnapshot>('get_network_snapshot');
      set({ snapshot, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch metrics history
  fetchMetricsHistory: async (limit = 60) => {
    try {
      const metricsHistory = await invoke<MetricsEntry[]>('get_metrics_history', { limit });
      set({ metricsHistory, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Fetch DHT routing table
  fetchRoutingTable: async () => {
    set({ loading: true });
    try {
      const routingTable = await invoke<RoutingBucket[]>('get_routing_table');
      set({ routingTable, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Fetch stored keys
  getStoredKeys: async () => {
    set({ loading: true });
    try {
      const storedKeys = await invoke<StoredKey[]>('get_stored_keys');
      set({ storedKeys, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Look up a key in the DHT
  lookupKey: async (key: string) => {
    set({ loading: true, lookupResult: null });
    try {
      const result = await invoke<LookupResult>('lookup_key', { key });
      set({ lookupResult: result, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Ping a peer
  pingPeer: async (peerId: string, count = 5) => {
    set({ loading: true, lastPingResult: null });
    try {
      const result = await invoke<PingResult>('ping_peer', { peerId, count });
      set({ loading: false, lastPingResult: result, error: null });
      return result;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Run bandwidth test
  testBandwidth: async (peerId: string, size = 1048576) => {
    set({ loading: true, lastBandwidthResult: null });
    try {
      const result = await invoke<BandwidthResult>('bandwidth_test', { peerId, size });
      set({ loading: false, lastBandwidthResult: result, error: null });
      return result;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Check connection health
  checkHealth: async (peerId: string) => {
    set({ loading: true, lastHealthReport: null });
    try {
      const result = await invoke<HealthReport>('check_connection_health', { peerId });
      set({ loading: false, lastHealthReport: result, error: null });
      return result;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Detect NAT type
  detectNat: async () => {
    set({ loading: true, natResult: null });
    try {
      const result = await invoke<NatDetectionResult>('detect_nat_type');
      set({ loading: false, natResult: result, error: null });
      return result;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Export network data
  exportData: async (format: 'json' | 'csv') => {
    try {
      const data = await invoke<string>('export_snapshot', { format });
      return data;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  // Export metrics history
  exportMetrics: async (limit = 3600) => {
    try {
      const data = await invoke<string>('export_metrics', { limit });
      return data;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  // Set selected peer
  setSelectedPeer: (peerId) => {
    set({ selectedPeerId: peerId });
  },

  // Clear error
  clearError: () => {
    set({ error: null });
  },
}));
