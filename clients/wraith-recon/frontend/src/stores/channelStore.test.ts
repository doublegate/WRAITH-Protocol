// WRAITH Recon - Channel Store Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useChannelStore } from './channelStore';
import * as tauri from '../lib/tauri';
import type { ChannelInfo } from '../types';

vi.mock('../lib/tauri');

describe('channelStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    const store = useChannelStore.getState();
    store.channels = [];
    store.selectedChannelId = null;
    store.selectedChannelStats = null;
    store.loading = false;
    store.error = null;
  });

  it('has correct initial state', () => {
    const state = useChannelStore.getState();

    expect(state.channels).toEqual([]);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('opens a channel', async () => {
    const mockChannelId = 'channel-001';
    (tauri.openChannel as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockChannelId);
    (tauri.listChannels as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([]);

    const channelId = await useChannelStore.getState().openChannel('Udp', '192.168.1.100', 443);

    expect(channelId).toBe(mockChannelId);
    expect(tauri.openChannel).toHaveBeenCalledWith('Udp', '192.168.1.100', 443);
  });

  it('closes a channel', async () => {
    (tauri.closeChannel as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (tauri.listChannels as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([]);

    await useChannelStore.getState().closeChannel('channel-001');

    expect(tauri.closeChannel).toHaveBeenCalledWith('channel-001');
  });

  it('sends data through channel', async () => {
    const mockBytesSent = 1024;
    (tauri.sendThroughChannel as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockBytesSent);
    (tauri.getChannelStats as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      bytes_sent: 1024,
      bytes_received: 0,
      packets_sent: 1,
      packets_received: 0,
      errors: 0,
      latency_ms: 5,
    });

    const data = [1, 2, 3, 4];
    const result = await useChannelStore.getState().sendData('channel-001', data);

    expect(result).toBe(mockBytesSent);
    expect(tauri.sendThroughChannel).toHaveBeenCalledWith('channel-001', data);
  });

  it('lists channels', async () => {
    const mockChannels: ChannelInfo[] = [
      {
        id: 'channel-001',
        channel_type: 'Udp',
        target: '192.168.1.100',
        port: 443,
        state: 'Active',
        bytes_sent: 1024,
        bytes_received: 512,
        created_at: '2024-01-01T00:00:00Z',
        last_activity: '2024-01-01T00:01:00Z',
        stats: {
          bytes_sent: 1024,
          bytes_received: 512,
          packets_sent: 10,
          packets_received: 5,
          errors: 0,
          latency_ms: 5,
        },
      },
      {
        id: 'channel-002',
        channel_type: 'TcpMimicry',
        target: '192.168.1.101',
        port: 80,
        state: 'Open',
        bytes_sent: 0,
        bytes_received: 0,
        created_at: '2024-01-01T00:00:00Z',
        last_activity: '2024-01-01T00:00:00Z',
        stats: {
          bytes_sent: 0,
          bytes_received: 0,
          packets_sent: 0,
          packets_received: 0,
          errors: 0,
          latency_ms: null,
        },
      },
    ];

    (tauri.listChannels as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockChannels);

    await useChannelStore.getState().fetchChannels();

    const state = useChannelStore.getState();
    expect(state.channels).toHaveLength(2);
    expect(state.channels[0].id).toBe('channel-001');
    expect(tauri.listChannels).toHaveBeenCalled();
  });

  it('fetches channel stats', async () => {
    const mockStats = {
      bytes_sent: 10240,
      bytes_received: 5120,
      packets_sent: 100,
      packets_received: 50,
      errors: 3,
      latency_ms: 15,
    };

    (tauri.getChannelStats as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);

    await useChannelStore.getState().fetchChannelStats('channel-001');

    const state = useChannelStore.getState();
    expect(state.selectedChannelStats).toEqual(mockStats);
    expect(tauri.getChannelStats).toHaveBeenCalledWith('channel-001');
  });

  it('handles channel types', async () => {
    const channelTypes: ['Udp', 'TcpMimicry', 'Https', 'DnsTunnel', 'Icmp'] = ['Udp', 'TcpMimicry', 'Https', 'DnsTunnel', 'Icmp'];

    for (const channelType of channelTypes) {
      (tauri.openChannel as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(`channel-${channelType}`);
      (tauri.listChannels as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      await useChannelStore.getState().openChannel(channelType, '192.168.1.100', 443);

      expect(tauri.openChannel).toHaveBeenCalledWith(channelType, '192.168.1.100', 443);
    }
  });

  it('handles errors', async () => {
    const error = new Error('Target out of scope');
    (tauri.openChannel as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(error);

    await expect(
      useChannelStore.getState().openChannel('Udp', '8.8.8.8', 443)
    ).rejects.toThrow();

    const state = useChannelStore.getState();
    expect(state.error).toContain('Target out of scope');
  });

  it('selects a channel', () => {
    useChannelStore.setState({
      channels: [
        {
          id: 'channel-001',
          channel_type: 'Udp',
          target: '192.168.1.100',
          port: 443,
          state: 'Active',
          bytes_sent: 0,
          bytes_received: 0,
          created_at: '2024-01-01T00:00:00Z',
          last_activity: '2024-01-01T00:00:00Z',
          stats: { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0, errors: 0, latency_ms: null },
        },
      ],
    });

    (tauri.getChannelStats as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      bytes_sent: 0,
      bytes_received: 0,
      packets_sent: 0,
      packets_received: 0,
      errors: 0,
      latency_ms: null,
    });

    useChannelStore.getState().selectChannel('channel-001');

    const state = useChannelStore.getState();
    expect(state.selectedChannelId).toBe('channel-001');
  });

  it('clears error', () => {
    useChannelStore.setState({ error: 'Some error' });

    useChannelStore.getState().clearError();

    const state = useChannelStore.getState();
    expect(state.error).toBeNull();
  });
});
