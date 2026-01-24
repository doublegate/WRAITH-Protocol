// WRAITH Recon - Channel Store

import { create } from 'zustand';
import * as tauri from '../lib/tauri';
import type { ChannelType, ChannelInfo, ChannelStats } from '../types';

interface ChannelState {
  // State
  channels: ChannelInfo[];
  selectedChannelId: string | null;
  selectedChannelStats: ChannelStats | null;
  loading: boolean;
  error: string | null;

  // Actions
  openChannel: (channelType: ChannelType, target: string, port?: number) => Promise<string>;
  closeChannel: (channelId: string) => Promise<void>;
  sendData: (channelId: string, data: number[]) => Promise<number>;
  fetchChannels: () => Promise<void>;
  selectChannel: (channelId: string | null) => void;
  fetchChannelStats: (channelId: string) => Promise<void>;
  clearError: () => void;
}

export const useChannelStore = create<ChannelState>((set, get) => ({
  // Initial state
  channels: [],
  selectedChannelId: null,
  selectedChannelStats: null,
  loading: false,
  error: null,

  // Open a new channel
  openChannel: async (channelType: ChannelType, target: string, port?: number) => {
    set({ loading: true, error: null });
    try {
      const channelId = await tauri.openChannel(channelType, target, port);
      await get().fetchChannels();
      set({ loading: false });
      return channelId;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Close a channel
  closeChannel: async (channelId: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.closeChannel(channelId);
      await get().fetchChannels();
      const { selectedChannelId } = get();
      if (selectedChannelId === channelId) {
        set({ selectedChannelId: null, selectedChannelStats: null });
      }
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  // Send data through a channel
  sendData: async (channelId: string, data: number[]) => {
    set({ loading: true, error: null });
    try {
      const bytesSent = await tauri.sendThroughChannel(channelId, data);
      await get().fetchChannelStats(channelId);
      set({ loading: false });
      return bytesSent;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  // Fetch all channels
  fetchChannels: async () => {
    try {
      const channels = await tauri.listChannels();
      set({ channels });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Select a channel
  selectChannel: (channelId: string | null) => {
    set({ selectedChannelId: channelId, selectedChannelStats: null });
    if (channelId) {
      get().fetchChannelStats(channelId);
    }
  },

  // Fetch channel statistics
  fetchChannelStats: async (channelId: string) => {
    try {
      const stats = await tauri.getChannelStats(channelId);
      set({ selectedChannelStats: stats });
    } catch {
      // Stats might not be available
    }
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
