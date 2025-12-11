// Message Store (Zustand)

import { create } from 'zustand';
import type { Message } from '../types';
import * as tauri from '../lib/tauri';

interface MessageState {
  messages: Record<number, Message[]>; // conversationId -> messages
  loading: boolean;
  error: string | null;

  loadMessages: (conversationId: number, limit?: number, offset?: number) => Promise<void>;
  sendMessage: (conversationId: number, peerId: string, body: string) => Promise<void>;
  markAsRead: (conversationId: number) => Promise<void>;
}

export const useMessageStore = create<MessageState>((set, get) => ({
  messages: {},
  loading: false,
  error: null,

  loadMessages: async (conversationId: number, limit = 50, offset = 0) => {
    set({ loading: true, error: null });
    try {
      const messages = await tauri.getMessages(conversationId, limit, offset);
      set((state) => ({
        messages: {
          ...state.messages,
          [conversationId]: messages.reverse(), // Oldest first
        },
        loading: false,
      }));
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  sendMessage: async (conversationId: number, peerId: string, body: string) => {
    try {
      await tauri.sendMessage(conversationId, peerId, body);
      // Reload messages after sending
      await get().loadMessages(conversationId);
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  markAsRead: async (conversationId: number) => {
    try {
      await tauri.markAsRead(conversationId);
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },
}));
