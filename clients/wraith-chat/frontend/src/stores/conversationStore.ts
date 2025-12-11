// Conversation Store (Zustand)

import { create } from 'zustand';
import type { Conversation } from '../types';
import * as tauri from '../lib/tauri';

interface ConversationState {
  conversations: Conversation[];
  selectedConversationId: number | null;
  loading: boolean;
  error: string | null;

  loadConversations: () => Promise<void>;
  selectConversation: (id: number) => void;
  createConversation: (
    convType: string,
    peerId: string | null,
    groupId: string | null,
    displayName: string | null
  ) => Promise<number>;
}

export const useConversationStore = create<ConversationState>((set, get) => ({
  conversations: [],
  selectedConversationId: null,
  loading: false,
  error: null,

  loadConversations: async () => {
    set({ loading: true, error: null });
    try {
      const conversations = await tauri.listConversations();
      set({ conversations, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  selectConversation: (id: number) => {
    set({ selectedConversationId: id });
  },

  createConversation: async (
    convType: string,
    peerId: string | null,
    groupId: string | null,
    displayName: string | null
  ) => {
    try {
      const id = await tauri.createConversation(convType, peerId, groupId, displayName);
      await get().loadConversations();
      return id;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },
}));
