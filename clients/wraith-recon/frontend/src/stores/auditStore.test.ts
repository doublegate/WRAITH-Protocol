// WRAITH Recon - Audit Store Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useAuditStore } from './auditStore';
import * as tauri from '../lib/tauri';

vi.mock('../lib/tauri');

describe('auditStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    const store = useAuditStore.getState();
    store.entries = [];
    store.chainValid = null;
    store.loading = false;
    store.error = null;
  });

  it('has correct initial state', () => {
    const state = useAuditStore.getState();

    expect(state.entries).toEqual([]);
    expect(state.chainValid).toBeNull();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetches audit entries', async () => {
    const mockEntries = [
      {
        id: 'entry-1',
        sequence: 0,
        timestamp: '2024-01-01T00:00:00Z',
        level: 'Info' as const,
        category: 'System',
        summary: 'System started',
        details: null,
        operator_id: 'test-op',
        mitre_technique: null,
        mitre_tactic: null,
        previous_hash: '000',
        signature: 'sig123',
      },
      {
        id: 'entry-2',
        sequence: 1,
        timestamp: '2024-01-01T00:01:00Z',
        level: 'Info' as const,
        category: 'Reconnaissance',
        summary: 'Port scan started',
        details: null,
        operator_id: 'test-op',
        mitre_technique: 'T1046',
        mitre_tactic: 'Discovery',
        previous_hash: 'abc',
        signature: 'sig456',
      },
    ];

    (tauri.getAuditEntries as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockEntries);

    await useAuditStore.getState().fetchEntries(0, 100);

    const state = useAuditStore.getState();
    expect(state.entries).toHaveLength(2);
    expect(state.entries[0].id).toBe('entry-1');
    expect(tauri.getAuditEntries).toHaveBeenCalledWith(0, 100);
  });

  it('verifies audit chain', async () => {
    const mockResult = {
      valid: true,
      entries_verified: 100,
      first_invalid_sequence: null,
      errors: [],
    };

    (tauri.verifyAuditChain as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult);

    await useAuditStore.getState().verifyChain();

    const state = useAuditStore.getState();
    expect(state.chainValid).toBe(true);
    expect(tauri.verifyAuditChain).toHaveBeenCalled();
  });

  it('detects tampered chain', async () => {
    const mockResult = {
      valid: false,
      entries_verified: 50,
      first_invalid_sequence: 25,
      errors: ['Entry 25 hash mismatch'],
    };

    (tauri.verifyAuditChain as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult);

    await useAuditStore.getState().verifyChain();

    const state = useAuditStore.getState();
    expect(state.chainValid).toBe(false);
  });

  it('exports audit log', async () => {
    const mockExportPath = '/tmp/audit-export.json';

    (tauri.exportAuditLog as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockExportPath);

    const result = await useAuditStore.getState().exportLog();

    expect(result).toBe(mockExportPath);
    expect(tauri.exportAuditLog).toHaveBeenCalled();
  });

  it('adds audit note', async () => {
    (tauri.addAuditNote as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (tauri.getAuditEntries as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([]);

    await useAuditStore.getState().addNote('Manual observation note');

    expect(tauri.addAuditNote).toHaveBeenCalledWith('Manual observation note');
  });

  it('handles fetch error', async () => {
    const error = new Error('Failed to fetch entries');
    (tauri.getAuditEntries as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(error);

    await useAuditStore.getState().fetchEntries(0, 100);

    const state = useAuditStore.getState();
    expect(state.error).toContain('Failed to fetch entries');
  });

  it('fetches database stats', async () => {
    const mockStats = {
      audit_entries: 100,
      roe_entries: 5,
      db_size_bytes: 1024000,
    };

    (tauri.getDatabaseStats as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);

    await useAuditStore.getState().fetchDatabaseStats();

    const state = useAuditStore.getState();
    expect(state.databaseStats).toEqual(mockStats);
  });
});
