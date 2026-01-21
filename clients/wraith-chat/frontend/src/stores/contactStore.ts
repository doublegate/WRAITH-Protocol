// Contact Store (Zustand)

import { create } from "zustand";
import type { Contact } from "../types";
import * as tauri from "../lib/tauri";

interface ContactState {
  contacts: Contact[];
  loading: boolean;
  error: string | null;

  loadContacts: () => Promise<void>;
  addContact: (
    peerId: string,
    displayName: string | null,
    identityKey: number[],
  ) => Promise<number>;
  getContact: (peerId: string) => Promise<Contact | null>;
}

export const useContactStore = create<ContactState>((set, get) => ({
  contacts: [],
  loading: false,
  error: null,

  loadContacts: async () => {
    set({ loading: true, error: null });
    try {
      const contacts = await tauri.listContacts();
      set({ contacts, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  addContact: async (
    peerId: string,
    displayName: string | null,
    identityKey: number[],
  ) => {
    try {
      const id = await tauri.createContact(peerId, displayName, identityKey);
      await get().loadContacts();
      return id;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  getContact: async (peerId: string) => {
    try {
      return await tauri.getContact(peerId);
    } catch (error) {
      set({ error: (error as Error).message });
      return null;
    }
  },
}));
