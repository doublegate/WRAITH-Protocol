// Group Store (Zustand) - Group state management

import { create } from 'zustand';
import type {
  Group,
  GroupInfo,
  GroupMember,
  ExportedInvitation,
  MemberRole,
} from '../types';
import * as tauri from '../lib/tauri';

interface GroupState {
  // State
  groups: Group[];
  groupInfos: Map<string, GroupInfo>;
  selectedGroupId: string | null;
  members: GroupMember[];
  loading: boolean;
  error: string | null;

  // Actions - Groups
  fetchGroups: () => Promise<void>;
  fetchGroupInfo: (groupId: string) => Promise<GroupInfo | null>;
  createGroup: (name: string, description?: string) => Promise<Group>;
  deleteGroup: (groupId: string) => Promise<void>;
  selectGroup: (groupId: string | null) => void;

  // Actions - Members
  fetchMembers: (groupId: string) => Promise<void>;
  inviteMember: (
    groupId: string,
    peerId: string | null,
    role: MemberRole
  ) => Promise<ExportedInvitation>;
  acceptInvitation: (invitation: ExportedInvitation) => Promise<void>;
  removeMember: (groupId: string, peerId: string) => Promise<void>;
  setMemberRole: (
    groupId: string,
    peerId: string,
    role: MemberRole
  ) => Promise<void>;

  // Utility
  clearError: () => void;
}

export const useGroupStore = create<GroupState>((set, get) => ({
  // Initial state
  groups: [],
  groupInfos: new Map(),
  selectedGroupId: null,
  members: [],
  loading: false,
  error: null,

  // Group actions
  fetchGroups: async () => {
    set({ loading: true, error: null });
    try {
      const groups = await tauri.listGroups();
      set({ groups, loading: false });

      // Fetch info for each group
      for (const group of groups) {
        get().fetchGroupInfo(group.id);
      }
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  fetchGroupInfo: async (groupId: string) => {
    try {
      const info = await tauri.getGroupInfo(groupId);
      if (info) {
        set((state) => {
          const newInfos = new Map(state.groupInfos);
          newInfos.set(groupId, info);
          return { groupInfos: newInfos };
        });
      }
      return info;
    } catch (error) {
      console.error('Failed to fetch group info:', error);
      return null;
    }
  },

  createGroup: async (name: string, description?: string) => {
    set({ loading: true, error: null });
    try {
      const group = await tauri.createGroup(name, description);
      set((state) => ({
        groups: [...state.groups, group],
        loading: false,
      }));
      // Fetch the info for the new group
      get().fetchGroupInfo(group.id);
      return group;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  deleteGroup: async (groupId: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.deleteGroup(groupId);
      set((state) => {
        const newInfos = new Map(state.groupInfos);
        newInfos.delete(groupId);
        return {
          groups: state.groups.filter((g) => g.id !== groupId),
          groupInfos: newInfos,
          selectedGroupId:
            state.selectedGroupId === groupId ? null : state.selectedGroupId,
          loading: false,
        };
      });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  selectGroup: (groupId: string | null) => {
    set({ selectedGroupId: groupId, members: [] });
    if (groupId) {
      get().fetchMembers(groupId);
    }
  },

  // Member actions
  fetchMembers: async (groupId: string) => {
    try {
      const members = await tauri.listMembers(groupId);
      set({ members });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  inviteMember: async (
    groupId: string,
    peerId: string | null,
    role: MemberRole
  ) => {
    try {
      const invitation = await tauri.inviteMember(groupId, peerId, role);
      // Refresh members if it was a direct invitation
      if (peerId) {
        await get().fetchMembers(groupId);
        await get().fetchGroupInfo(groupId);
      }
      return invitation;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  acceptInvitation: async (invitation: ExportedInvitation) => {
    try {
      await tauri.acceptInvitation(invitation);
      await get().fetchGroups();
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  removeMember: async (groupId: string, peerId: string) => {
    try {
      await tauri.removeMember(groupId, peerId);
      await get().fetchMembers(groupId);
      await get().fetchGroupInfo(groupId);
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  setMemberRole: async (groupId: string, peerId: string, role: MemberRole) => {
    try {
      await tauri.setMemberRole(groupId, peerId, role);
      await get().fetchMembers(groupId);
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  clearError: () => set({ error: null }),
}));
