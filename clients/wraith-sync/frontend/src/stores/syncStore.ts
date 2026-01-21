// Sync Store (Zustand) - Main sync state management

import { create } from 'zustand';
import type { OverallStatus, FolderInfo, ConflictInfo, FileInfo } from '../types';
import * as tauri from '../lib/tauri';

interface SyncState {
  // Status
  status: OverallStatus | null;
  loading: boolean;
  error: string | null;

  // Folders
  folders: FolderInfo[];
  selectedFolderId: number | null;
  folderFiles: FileInfo[];

  // Conflicts
  conflicts: ConflictInfo[];

  // Actions - Status
  refreshStatus: () => Promise<void>;
  pauseSync: () => Promise<void>;
  resumeSync: () => Promise<void>;

  // Actions - Folders
  loadFolders: () => Promise<void>;
  addFolder: (localPath: string, remotePath: string) => Promise<void>;
  removeFolder: (folderId: number) => Promise<void>;
  selectFolder: (folderId: number | null) => void;
  pauseFolder: (folderId: number) => Promise<void>;
  resumeFolder: (folderId: number) => Promise<void>;
  forceSyncFolder: (folderId: number) => Promise<void>;
  loadFolderFiles: (folderId: number) => Promise<void>;

  // Actions - Conflicts
  loadConflicts: () => Promise<void>;
  resolveConflict: (
    conflictId: number,
    resolution: 'local' | 'remote' | 'keep_both'
  ) => Promise<void>;
}

export const useSyncStore = create<SyncState>((set, get) => ({
  // Initial state
  status: null,
  loading: false,
  error: null,
  folders: [],
  selectedFolderId: null,
  folderFiles: [],
  conflicts: [],

  // Status actions
  refreshStatus: async () => {
    set({ loading: true, error: null });
    try {
      const status = await tauri.getStatus();
      set({ status, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  pauseSync: async () => {
    try {
      await tauri.pauseSync();
      await get().refreshStatus();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  resumeSync: async () => {
    try {
      await tauri.resumeSync();
      await get().refreshStatus();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  // Folder actions
  loadFolders: async () => {
    set({ loading: true, error: null });
    try {
      const folders = await tauri.listFolders();
      set({ folders, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  addFolder: async (localPath: string, remotePath: string) => {
    try {
      await tauri.addFolder(localPath, remotePath);
      await get().loadFolders();
      await get().refreshStatus();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  removeFolder: async (folderId: number) => {
    try {
      await tauri.removeFolder(folderId);
      await get().loadFolders();
      await get().refreshStatus();
      // Clear selection if we removed the selected folder
      if (get().selectedFolderId === folderId) {
        set({ selectedFolderId: null, folderFiles: [] });
      }
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  selectFolder: (folderId: number | null) => {
    set({ selectedFolderId: folderId, folderFiles: [] });
    if (folderId !== null) {
      get().loadFolderFiles(folderId);
    }
  },

  pauseFolder: async (folderId: number) => {
    try {
      await tauri.pauseFolder(folderId);
      await get().loadFolders();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  resumeFolder: async (folderId: number) => {
    try {
      await tauri.resumeFolder(folderId);
      await get().loadFolders();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  forceSyncFolder: async (folderId: number) => {
    try {
      await tauri.forceSyncFolder(folderId);
      await get().loadFolders();
      await get().refreshStatus();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  loadFolderFiles: async (folderId: number) => {
    try {
      const folderFiles = await tauri.listFolderFiles(folderId);
      set({ folderFiles });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  // Conflict actions
  loadConflicts: async () => {
    try {
      const conflicts = await tauri.listConflicts();
      set({ conflicts });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  resolveConflict: async (
    conflictId: number,
    resolution: 'local' | 'remote' | 'keep_both'
  ) => {
    try {
      await tauri.resolveConflict(conflictId, resolution);
      await get().loadConflicts();
      await get().refreshStatus();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },
}));
