// Secret Store (Zustand) for WRAITH Vault

import { create } from "zustand";
import type { SecretInfo, SecretCreationResult, SecretType } from "../types";
import * as tauri from "../lib/tauri";

interface SecretState {
  secrets: SecretInfo[];
  selectedSecret: SecretInfo | null;
  loading: boolean;
  error: string | null;
  lastCreationResult: SecretCreationResult | null;

  // Actions
  loadSecrets: () => Promise<void>;
  loadSecretsByType: (type: SecretType) => Promise<void>;
  loadSecretsByTag: (tag: string) => Promise<void>;
  searchSecrets: (query: string) => Promise<void>;
  createSecret: (
    name: string,
    secretData: Uint8Array,
    secretType: SecretType,
    description: string | null,
    threshold: number,
    totalShares: number,
    tags: string[],
    password: string | null
  ) => Promise<SecretCreationResult>;
  getSecret: (secretId: string) => Promise<SecretInfo | null>;
  updateSecret: (
    secretId: string,
    name: string | null,
    description: string | null,
    tags: string[] | null
  ) => Promise<SecretInfo>;
  deleteSecret: (secretId: string) => Promise<void>;
  getSecretsNeedingRotation: (maxAgeDays: number) => Promise<SecretInfo[]>;
  selectSecret: (secret: SecretInfo | null) => void;
  clearError: () => void;
}

export const useSecretStore = create<SecretState>((set, get) => ({
  secrets: [],
  selectedSecret: null,
  loading: false,
  error: null,
  lastCreationResult: null,

  loadSecrets: async () => {
    set({ loading: true, error: null });
    try {
      const secrets = await tauri.listSecrets();
      set({ secrets, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  loadSecretsByType: async (type: SecretType) => {
    set({ loading: true, error: null });
    try {
      const secrets = await tauri.listSecretsByType(type);
      set({ secrets, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  loadSecretsByTag: async (tag: string) => {
    set({ loading: true, error: null });
    try {
      const secrets = await tauri.listSecretsByTag(tag);
      set({ secrets, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  searchSecrets: async (query: string) => {
    set({ loading: true, error: null });
    try {
      const secrets = await tauri.searchSecrets(query);
      set({ secrets, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  createSecret: async (
    name: string,
    secretData: Uint8Array,
    secretType: SecretType,
    description: string | null,
    threshold: number,
    totalShares: number,
    tags: string[],
    password: string | null
  ) => {
    set({ loading: true, error: null });
    try {
      const result = await tauri.createSecret(
        name,
        Array.from(secretData),
        secretType,
        description,
        threshold,
        totalShares,
        tags,
        password
      );
      set({ lastCreationResult: result, loading: false });
      await get().loadSecrets();
      return result;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  getSecret: async (secretId: string) => {
    try {
      return await tauri.getSecret(secretId);
    } catch (error) {
      set({ error: (error as Error).message });
      return null;
    }
  },

  updateSecret: async (
    secretId: string,
    name: string | null,
    description: string | null,
    tags: string[] | null
  ) => {
    set({ loading: true, error: null });
    try {
      const updated = await tauri.updateSecret(secretId, name, description, tags);
      await get().loadSecrets();
      set({ loading: false });
      return updated;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  deleteSecret: async (secretId: string) => {
    set({ loading: true, error: null });
    try {
      await tauri.deleteSecret(secretId);
      if (get().selectedSecret?.id === secretId) {
        set({ selectedSecret: null });
      }
      await get().loadSecrets();
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      throw error;
    }
  },

  getSecretsNeedingRotation: async (maxAgeDays: number) => {
    try {
      return await tauri.getSecretsNeedingRotation(maxAgeDays);
    } catch (error) {
      set({ error: (error as Error).message });
      return [];
    }
  },

  selectSecret: (secret: SecretInfo | null) => {
    set({ selectedSecret: secret });
  },

  clearError: () => {
    set({ error: null });
  },
}));
