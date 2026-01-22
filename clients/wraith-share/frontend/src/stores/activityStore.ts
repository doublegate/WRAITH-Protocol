// Activity Store (Zustand) - Activity state management

import { create } from 'zustand';
import type { ActivityInfo, ActivityStats } from '../types';
import * as tauri from '../lib/tauri';

interface ActivityState {
  // State
  activities: ActivityInfo[];
  stats: ActivityStats | null;
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  offset: number;

  // Actions
  fetchActivities: (groupId: string) => Promise<void>;
  fetchMoreActivities: (groupId: string) => Promise<void>;
  fetchRecentActivities: () => Promise<void>;
  fetchStats: (groupId: string) => Promise<void>;
  searchActivities: (groupId: string, query: string) => Promise<void>;

  // Utility
  clearActivities: () => void;
  clearError: () => void;
}

const PAGE_SIZE = 20;

export const useActivityStore = create<ActivityState>((set, get) => ({
  // Initial state
  activities: [],
  stats: null,
  loading: false,
  error: null,
  hasMore: true,
  offset: 0,

  fetchActivities: async (groupId: string) => {
    set({ loading: true, error: null, offset: 0 });
    try {
      const activities = await tauri.getActivityLog(groupId, PAGE_SIZE, 0);
      set({
        activities,
        loading: false,
        hasMore: activities.length === PAGE_SIZE,
        offset: activities.length,
      });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  fetchMoreActivities: async (groupId: string) => {
    const { loading, hasMore, offset } = get();
    if (loading || !hasMore) return;

    set({ loading: true });
    try {
      const moreActivities = await tauri.getActivityLog(
        groupId,
        PAGE_SIZE,
        offset
      );
      set((state) => ({
        activities: [...state.activities, ...moreActivities],
        loading: false,
        hasMore: moreActivities.length === PAGE_SIZE,
        offset: state.offset + moreActivities.length,
      }));
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  fetchRecentActivities: async () => {
    set({ loading: true, error: null });
    try {
      const activities = await tauri.getRecentActivity(50);
      set({ activities, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  fetchStats: async (groupId: string) => {
    try {
      const stats = await tauri.getActivityStats(groupId);
      set({ stats });
    } catch (error) {
      console.error('Failed to fetch activity stats:', error);
    }
  },

  searchActivities: async (groupId: string, query: string) => {
    set({ loading: true, error: null });
    try {
      const activities = await tauri.searchActivity(groupId, query, 50);
      set({ activities, loading: false, hasMore: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  clearActivities: () =>
    set({ activities: [], stats: null, offset: 0, hasMore: true }),

  clearError: () => set({ error: null }),
}));
