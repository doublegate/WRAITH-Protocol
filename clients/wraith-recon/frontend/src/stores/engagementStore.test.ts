// WRAITH Recon - Engagement Store Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useEngagementStore } from './engagementStore';
import * as tauri from '../lib/tauri';

vi.mock('../lib/tauri');

describe('engagementStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    const store = useEngagementStore.getState();
    store.status = 'NotLoaded';
    store.engagementId = null;
    store.roe = null;
    store.scopeSummary = null;
    store.killSwitchState = null;
    store.timingInfo = null;
    store.operatorId = '';
    store.loading = false;
    store.error = null;
  });

  it('has correct initial state', () => {
    const state = useEngagementStore.getState();

    expect(state.status).toBe('NotLoaded');
    expect(state.roe).toBeNull();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetches engagement status', async () => {
    const mockStatus = {
      status: 'Active',
      engagement_id: 'eng-123',
      roe_id: 'roe-123',
      roe_title: 'Test Engagement',
      operator_id: 'test-op',
      time_remaining: '1:00:00',
      kill_switch_active: false,
    };

    (tauri.getEngagementStatus as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockStatus);

    await useEngagementStore.getState().fetchStatus();

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Active');
    expect(state.engagementId).toBe('eng-123');
    expect(tauri.getEngagementStatus).toHaveBeenCalled();
  });

  it('loads RoE from object', async () => {
    const mockRoe = {
      id: 'test-roe',
      organization: 'Test Org',
      title: 'Test Engagement',
      start_time: '2024-01-01T00:00:00Z',
      end_time: '2024-01-02T00:00:00Z',
      authorized_operators: ['op-1'],
      authorized_cidrs: ['192.168.1.0/24'],
      authorized_domains: ['test.com'],
      excluded_cidrs: [],
      excluded_domains: [],
      authorized_techniques: ['T1046'],
      prohibited_techniques: ['T1485'],
      signer_public_key: 'key123',
      signature: 'sig123',
    };

    (tauri.loadRoe as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (tauri.getScopeSummary as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      authorized_cidr_count: 1,
      authorized_domain_count: 1,
      excluded_cidr_count: 0,
      excluded_domain_count: 0,
      custom_target_count: 0,
    });

    await useEngagementStore.getState().loadRoe(mockRoe);

    const state = useEngagementStore.getState();
    expect(state.roe).toEqual(mockRoe);
    expect(state.status).toBe('Ready');
    expect(tauri.loadRoe).toHaveBeenCalledWith(mockRoe);
  });

  it('starts engagement', async () => {
    const mockEngagementId = 'eng-456';
    (tauri.startEngagement as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockEngagementId);

    const engagementId = await useEngagementStore.getState().startEngagement();

    expect(engagementId).toBe(mockEngagementId);
    expect(tauri.startEngagement).toHaveBeenCalled();

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Active');
    expect(state.engagementId).toBe(mockEngagementId);
  });

  it('stops engagement', async () => {
    (tauri.stopEngagement as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

    await useEngagementStore.getState().stopEngagement('Test complete');

    expect(tauri.stopEngagement).toHaveBeenCalledWith('Test complete');

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Completed');
  });

  it('pauses engagement', async () => {
    (tauri.pauseEngagement as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

    await useEngagementStore.getState().pauseEngagement();

    expect(tauri.pauseEngagement).toHaveBeenCalled();

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Paused');
  });

  it('resumes engagement', async () => {
    (tauri.resumeEngagement as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

    await useEngagementStore.getState().resumeEngagement();

    expect(tauri.resumeEngagement).toHaveBeenCalled();

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Active');
  });

  it('activates kill switch', async () => {
    (tauri.activateKillSwitch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (tauri.isKillSwitchActive as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      activated: true,
      activated_at: '2024-01-01T00:00:00Z',
      reason: 'Emergency',
      activated_by: 'test-op',
      signal_id: 'sig-123',
    });

    await useEngagementStore.getState().activateKillSwitch('Emergency');

    expect(tauri.activateKillSwitch).toHaveBeenCalledWith('Emergency');

    const state = useEngagementStore.getState();
    expect(state.status).toBe('Terminated');
  });

  it('handles fetch error', async () => {
    const error = new Error('Network error');
    (tauri.getEngagementStatus as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(error);

    await useEngagementStore.getState().fetchStatus();

    const state = useEngagementStore.getState();
    expect(state.error).toContain('Network error');
  });

  it('fetches timing info', async () => {
    const mockTimingInfo = {
      start_time: '2024-01-01T00:00:00Z',
      end_time: '2024-01-02T00:00:00Z',
      is_active: true,
      time_remaining_secs: 3600,
      status: 'Active' as const,
    };

    (tauri.getTimingInfo as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockTimingInfo);

    await useEngagementStore.getState().fetchTimingInfo();

    const state = useEngagementStore.getState();
    expect(state.timingInfo).toEqual(mockTimingInfo);
  });

  it('clears error', () => {
    useEngagementStore.setState({ error: 'Some error' });

    useEngagementStore.getState().clearError();

    const state = useEngagementStore.getState();
    expect(state.error).toBeNull();
  });
});
