// Group Messaging Store (Zustand) - Sprint 17.7

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

// Types
export interface GroupInfo {
  group_id: string;
  name: string;
  description?: string;
  member_count: number;
  created_at: number;
  am_i_admin: boolean;
}

export interface GroupMember {
  peer_id: string;
  display_name?: string;
  role: 'admin' | 'member';
  joined_at: number;
  key_generation: number;
}

interface GroupStoreState {
  // Groups
  groups: GroupInfo[];
  currentGroupId: string | null;
  currentGroupMembers: GroupMember[];

  // Loading states
  loading: boolean;
  error: string | null;

  // Group actions
  createGroup: (name: string, memberPeerIds?: string[]) => Promise<GroupInfo>;
  loadGroupInfo: (groupId: string) => Promise<GroupInfo | null>;
  updateGroupSettings: (
    groupId: string,
    name?: string,
    description?: string,
    avatar?: number[]
  ) => Promise<GroupInfo>;
  leaveGroup: (groupId: string) => Promise<void>;

  // Member actions
  loadGroupMembers: (groupId: string) => Promise<GroupMember[]>;
  addMember: (groupId: string, peerId: string, displayName?: string) => Promise<GroupMember>;
  removeMember: (groupId: string, peerId: string) => Promise<void>;
  promoteToAdmin: (groupId: string, peerId: string) => Promise<void>;
  demoteFromAdmin: (groupId: string, peerId: string) => Promise<void>;

  // Messaging
  sendGroupMessage: (groupId: string, body: string) => Promise<number>;

  // Key management
  rotateGroupKeys: (groupId: string) => Promise<void>;

  // Utility
  selectGroup: (groupId: string | null) => void;
  clearError: () => void;
}

export const useGroupStore = create<GroupStoreState>((set, get) => ({
  groups: [],
  currentGroupId: null,
  currentGroupMembers: [],
  loading: false,
  error: null,

  createGroup: async (name: string, memberPeerIds?: string[]) => {
    set({ loading: true, error: null });
    try {
      const group: GroupInfo = await invoke('create_group', {
        name,
        memberPeerIds,
      });
      set((state) => ({
        groups: [...state.groups, group],
        loading: false,
      }));
      return group;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  loadGroupInfo: async (groupId: string) => {
    try {
      const group: GroupInfo | null = await invoke('get_group_info', { groupId });
      if (group) {
        set((state) => ({
          groups: state.groups.map((g) =>
            g.group_id === groupId ? group : g
          ),
        }));
      }
      return group;
    } catch (error) {
      set({ error: (error as Error).message });
      return null;
    }
  },

  updateGroupSettings: async (
    groupId: string,
    name?: string,
    description?: string,
    avatar?: number[]
  ) => {
    set({ loading: true, error: null });
    try {
      const group: GroupInfo = await invoke('update_group_settings', {
        groupId,
        name,
        description,
        avatar,
      });
      set((state) => ({
        groups: state.groups.map((g) =>
          g.group_id === groupId ? group : g
        ),
        loading: false,
      }));
      return group;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  leaveGroup: async (groupId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('leave_group', { groupId });
      set((state) => ({
        groups: state.groups.filter((g) => g.group_id !== groupId),
        currentGroupId: state.currentGroupId === groupId ? null : state.currentGroupId,
        loading: false,
      }));
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  loadGroupMembers: async (groupId: string) => {
    try {
      const members: GroupMember[] = await invoke('get_group_members', { groupId });
      if (get().currentGroupId === groupId) {
        set({ currentGroupMembers: members });
      }
      return members;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  addMember: async (groupId: string, peerId: string, displayName?: string) => {
    set({ loading: true, error: null });
    try {
      const member: GroupMember = await invoke('add_group_member', {
        groupId,
        peerId,
        displayName,
      });
      if (get().currentGroupId === groupId) {
        set((state) => ({
          currentGroupMembers: [...state.currentGroupMembers, member],
          loading: false,
        }));
      }
      // Refresh group info to update member count
      await get().loadGroupInfo(groupId);
      return member;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  removeMember: async (groupId: string, peerId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('remove_group_member', { groupId, peerId });
      if (get().currentGroupId === groupId) {
        set((state) => ({
          currentGroupMembers: state.currentGroupMembers.filter(
            (m) => m.peer_id !== peerId
          ),
          loading: false,
        }));
      }
      // Refresh group info to update member count
      await get().loadGroupInfo(groupId);
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  promoteToAdmin: async (groupId: string, peerId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('promote_to_admin', { groupId, peerId });
      if (get().currentGroupId === groupId) {
        set((state) => ({
          currentGroupMembers: state.currentGroupMembers.map((m) =>
            m.peer_id === peerId ? { ...m, role: 'admin' as const } : m
          ),
          loading: false,
        }));
      }
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  demoteFromAdmin: async (groupId: string, peerId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('demote_from_admin', { groupId, peerId });
      if (get().currentGroupId === groupId) {
        set((state) => ({
          currentGroupMembers: state.currentGroupMembers.map((m) =>
            m.peer_id === peerId ? { ...m, role: 'member' as const } : m
          ),
          loading: false,
        }));
      }
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  sendGroupMessage: async (groupId: string, body: string) => {
    try {
      const messageId: number = await invoke('send_group_message', {
        groupId,
        body,
      });
      return messageId;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  rotateGroupKeys: async (groupId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('rotate_group_keys', { groupId });
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  selectGroup: (groupId: string | null) => {
    set({
      currentGroupId: groupId,
      currentGroupMembers: groupId ? get().currentGroupMembers : [],
    });
    if (groupId) {
      get().loadGroupMembers(groupId);
    }
  },

  clearError: () => {
    set({ error: null });
  },
}));

// Helper functions
export function formatMemberCount(count: number): string {
  if (count === 1) return '1 member';
  return `${count} members`;
}

export function getRoleLabel(role: 'admin' | 'member'): string {
  return role === 'admin' ? 'Admin' : 'Member';
}

export function formatJoinedDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString();
}
