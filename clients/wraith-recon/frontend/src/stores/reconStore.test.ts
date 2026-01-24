// WRAITH Recon - Recon Store Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useReconStore } from './reconStore';
import * as tauri from '../lib/tauri';
import type { ActiveScanConfig, PassiveReconStats, NetworkAsset, ActiveScanProgress, ProbeResult } from '../types';

vi.mock('../lib/tauri');

describe('reconStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    const store = useReconStore.getState();
    store.passiveStats = null;
    store.discoveredAssets = [];
    store.passiveRunning = false;
    store.activeScanProgress = null;
    store.activeScanResults = [];
    store.activeRunning = false;
    store.loading = false;
    store.error = null;
  });

  it('has correct initial state', () => {
    const state = useReconStore.getState();

    expect(state.passiveRunning).toBe(false);
    expect(state.activeRunning).toBe(false);
    expect(state.loading).toBe(false);
  });

  describe('passive reconnaissance', () => {
    it('starts passive recon', async () => {
      const mockSessionId = 'session-123';
      (tauri.startPassiveRecon as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockSessionId);

      const sessionId = await useReconStore.getState().startPassiveRecon('eth0', 60);

      expect(sessionId).toBe(mockSessionId);
      expect(tauri.startPassiveRecon).toHaveBeenCalledWith('eth0', 60);

      const state = useReconStore.getState();
      expect(state.passiveRunning).toBe(true);
    });

    it('stops passive recon', async () => {
      const mockStats: PassiveReconStats = {
        packets_captured: 1000,
        bytes_captured: 500000,
        unique_ips: 25,
        services_identified: 50,
        start_time: '2024-01-01T00:00:00Z',
        is_running: false,
      };

      (tauri.stopPassiveRecon as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);

      await useReconStore.getState().stopPassiveRecon();

      expect(tauri.stopPassiveRecon).toHaveBeenCalled();

      const state = useReconStore.getState();
      expect(state.passiveRunning).toBe(false);
      expect(state.passiveStats).toEqual(mockStats);
    });

    it('fetches passive recon stats', async () => {
      const mockStats: PassiveReconStats = {
        packets_captured: 1000,
        bytes_captured: 500000,
        unique_ips: 25,
        services_identified: 50,
        start_time: '2024-01-01T00:00:00Z',
        is_running: true,
      };

      (tauri.getPassiveReconStats as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);

      await useReconStore.getState().fetchPassiveStats();

      const state = useReconStore.getState();
      expect(state.passiveStats).toEqual(mockStats);
      expect(tauri.getPassiveReconStats).toHaveBeenCalled();
    });

    it('fetches discovered assets', async () => {
      const mockAssets: NetworkAsset[] = [
        {
          ip: '192.168.1.100',
          hostnames: ['server1'],
          ports: [22, 80],
          services: ['SSH', 'HTTP'],
          os_fingerprint: 'Linux',
          first_seen: '2024-01-01T00:00:00Z',
          last_seen: '2024-01-01T00:01:00Z',
          packet_count: 100,
        },
        {
          ip: '192.168.1.101',
          hostnames: ['server2'],
          ports: [443],
          services: ['HTTPS'],
          os_fingerprint: 'Windows',
          first_seen: '2024-01-01T00:00:00Z',
          last_seen: '2024-01-01T00:01:00Z',
          packet_count: 50,
        },
      ];

      (tauri.getDiscoveredAssets as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAssets);

      await useReconStore.getState().fetchDiscoveredAssets();

      const state = useReconStore.getState();
      expect(state.discoveredAssets).toHaveLength(2);
      expect(tauri.getDiscoveredAssets).toHaveBeenCalled();
    });
  });

  describe('active reconnaissance', () => {
    it('starts active scan', async () => {
      const mockScanId = 'scan-456';
      (tauri.startActiveScan as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockScanId);

      const config: ActiveScanConfig = {
        targets: ['192.168.1.0/24'],
        ports: [22, 80, 443],
        probe_type: 'TcpSyn',
        rate_limit: 1000,
        timeout_ms: 3000,
        retries: 2,
        service_detection: true,
        os_detection: false,
      };

      const scanId = await useReconStore.getState().startActiveScan(config);

      expect(scanId).toBe(mockScanId);
      expect(tauri.startActiveScan).toHaveBeenCalledWith(config);

      const state = useReconStore.getState();
      expect(state.activeRunning).toBe(true);
    });

    it('stops active scan', async () => {
      (tauri.stopActiveScan as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
      (tauri.getActiveScanProgress as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
        scan_id: 'scan-123',
        status: 'Cancelled',
        total_probes: 1000,
        completed_probes: 500,
        open_ports_found: 10,
        current_target: null,
        started_at: '2024-01-01T00:00:00Z',
        estimated_completion: null,
      });

      await useReconStore.getState().stopActiveScan();

      expect(tauri.stopActiveScan).toHaveBeenCalled();

      const state = useReconStore.getState();
      expect(state.activeRunning).toBe(false);
    });

    it('fetches active scan progress', async () => {
      const mockProgress: ActiveScanProgress = {
        scan_id: 'scan-123',
        status: 'Running',
        total_probes: 1000,
        completed_probes: 500,
        open_ports_found: 10,
        current_target: '192.168.1.50',
        started_at: '2024-01-01T00:00:00Z',
        estimated_completion: '2024-01-01T00:10:00Z',
      };

      (tauri.getActiveScanProgress as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockProgress);

      await useReconStore.getState().fetchActiveScanProgress();

      const state = useReconStore.getState();
      expect(state.activeScanProgress).toEqual(mockProgress);
      expect(tauri.getActiveScanProgress).toHaveBeenCalled();
    });

    it('fetches active scan results', async () => {
      const mockResults: ProbeResult[] = [
        {
          target: '192.168.1.100',
          port: 80,
          open: true,
          service: 'HTTP',
          response_time_ms: 5,
          probe_type: 'TcpSyn',
          timestamp: '2024-01-01T00:00:00Z',
        },
        {
          target: '192.168.1.100',
          port: 443,
          open: true,
          service: 'HTTPS',
          response_time_ms: 3,
          probe_type: 'TcpSyn',
          timestamp: '2024-01-01T00:00:01Z',
        },
      ];

      (tauri.getActiveScanResults as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockResults);

      await useReconStore.getState().fetchActiveScanResults();

      const state = useReconStore.getState();
      expect(state.activeScanResults).toHaveLength(2);
      expect(tauri.getActiveScanResults).toHaveBeenCalled();
    });
  });

  it('handles errors', async () => {
    const error = new Error('Operation not authorized');
    (tauri.startPassiveRecon as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(error);

    await expect(
      useReconStore.getState().startPassiveRecon('eth0', 60)
    ).rejects.toThrow();

    const state = useReconStore.getState();
    expect(state.error).toContain('Operation not authorized');
  });

  it('clears error', () => {
    // Set an error first
    useReconStore.setState({ error: 'Some error' });

    useReconStore.getState().clearError();

    expect(useReconStore.getState().error).toBeNull();
  });
});
