// Version Store (Zustand) - File version history management

import { create } from 'zustand';
import type { VersionInfo } from '../types';
import * as tauri from '../lib/tauri';

interface VersionState {
  // Version history
  versions: Map<string, VersionInfo[]>; // key: "folderId:relativePath"
  loading: boolean;
  error: string | null;

  // Selected file for version history
  selectedFile: { folderId: number; relativePath: string } | null;

  // Actions
  loadVersions: (folderId: number, relativePath: string) => Promise<void>;
  restoreVersion: (
    folderId: number,
    relativePath: string,
    versionId: number
  ) => Promise<void>;
  selectFile: (folderId: number, relativePath: string) => void;
  clearSelection: () => void;
}

// Helper to create version key
const versionKey = (folderId: number, relativePath: string): string =>
  `${folderId}:${relativePath}`;

export const useVersionStore = create<VersionState>((set, get) => ({
  // Initial state
  versions: new Map(),
  loading: false,
  error: null,
  selectedFile: null,

  // Actions
  loadVersions: async (folderId: number, relativePath: string) => {
    set({ loading: true, error: null });
    try {
      const fileVersions = await tauri.getFileVersions(folderId, relativePath);
      const versions = new Map(get().versions);
      versions.set(versionKey(folderId, relativePath), fileVersions);
      set({ versions, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  restoreVersion: async (
    folderId: number,
    relativePath: string,
    versionId: number
  ) => {
    set({ loading: true, error: null });
    try {
      await tauri.restoreVersion(folderId, relativePath, versionId);
      // Reload versions after restore
      await get().loadVersions(folderId, relativePath);
      set({ loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  selectFile: (folderId: number, relativePath: string) => {
    set({ selectedFile: { folderId, relativePath } });
    get().loadVersions(folderId, relativePath);
  },

  clearSelection: () => {
    set({ selectedFile: null });
  },
}));
