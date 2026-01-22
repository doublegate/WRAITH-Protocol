// File Store (Zustand) - File state management

import { create } from 'zustand';
import type {
  SharedFile,
  VersionInfo,
  ShareLinkInfo,
  UploadProgress,
  SortBy,
  SortOrder,
} from '../types';
import * as tauri from '../lib/tauri';

interface FileState {
  // State
  files: SharedFile[];
  selectedFileId: string | null;
  versions: VersionInfo[];
  shareLinks: ShareLinkInfo[];
  uploads: UploadProgress[];
  searchQuery: string;
  currentPath: string;
  sortBy: SortBy;
  sortOrder: SortOrder;
  loading: boolean;
  error: string | null;

  // Actions - Files
  fetchFiles: (groupId: string) => Promise<void>;
  searchFiles: (groupId: string, query: string) => Promise<void>;
  uploadFile: (groupId: string, file: File) => Promise<void>;
  downloadFile: (fileId: string, fileName: string) => Promise<void>;
  deleteFile: (fileId: string) => Promise<void>;
  selectFile: (fileId: string | null) => void;

  // Actions - Versions
  fetchVersions: (fileId: string) => Promise<void>;
  restoreVersion: (fileId: string, version: number) => Promise<void>;

  // Actions - Share Links
  fetchShareLinks: (fileId: string) => Promise<void>;
  createShareLink: (
    fileId: string,
    expiresInHours?: number,
    password?: string,
    maxDownloads?: number
  ) => Promise<ShareLinkInfo>;
  revokeShareLink: (linkId: string) => Promise<void>;

  // Actions - UI State
  setSearchQuery: (query: string) => void;
  setCurrentPath: (path: string) => void;
  setSortBy: (sortBy: SortBy) => void;
  setSortOrder: (order: SortOrder) => void;
  removeUpload: (id: string) => void;

  // Utility
  clearError: () => void;
  getSortedFiles: () => SharedFile[];
}

export const useFileStore = create<FileState>((set, get) => ({
  // Initial state
  files: [],
  selectedFileId: null,
  versions: [],
  shareLinks: [],
  uploads: [],
  searchQuery: '',
  currentPath: '/',
  sortBy: 'name',
  sortOrder: 'asc',
  loading: false,
  error: null,

  // File actions
  fetchFiles: async (groupId: string) => {
    set({ loading: true, error: null });
    try {
      const files = await tauri.listFiles(groupId);
      set({ files, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  searchFiles: async (groupId: string, query: string) => {
    set({ loading: true, error: null, searchQuery: query });
    try {
      const files = query
        ? await tauri.searchFiles(groupId, query)
        : await tauri.listFiles(groupId);
      set({ files, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  uploadFile: async (groupId: string, file: File) => {
    const uploadId = crypto.randomUUID();
    const progress: UploadProgress = {
      id: uploadId,
      fileName: file.name,
      progress: 0,
      status: 'pending',
    };

    set((state) => ({
      uploads: [...state.uploads, progress],
    }));

    try {
      // Update to uploading status
      set((state) => ({
        uploads: state.uploads.map((u) =>
          u.id === uploadId ? { ...u, status: 'uploading' as const, progress: 10 } : u
        ),
      }));

      // Read file as bytes
      const arrayBuffer = await file.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);

      // Update progress
      set((state) => ({
        uploads: state.uploads.map((u) =>
          u.id === uploadId ? { ...u, progress: 50 } : u
        ),
      }));

      // Upload
      const uploadedFile = await tauri.uploadFile(groupId, file.name, data);

      // Update progress to completed
      set((state) => ({
        uploads: state.uploads.map((u) =>
          u.id === uploadId
            ? { ...u, status: 'completed' as const, progress: 100 }
            : u
        ),
        files: [...state.files, uploadedFile],
      }));

      // Remove from uploads after delay
      setTimeout(() => {
        get().removeUpload(uploadId);
      }, 3000);
    } catch (error) {
      set((state) => ({
        uploads: state.uploads.map((u) =>
          u.id === uploadId
            ? {
                ...u,
                status: 'failed' as const,
                error: (error as Error).message,
              }
            : u
        ),
      }));
    }
  },

  downloadFile: async (fileId: string, fileName: string) => {
    try {
      const data = await tauri.downloadFile(fileId);

      // Create blob and download
      const blob = new Blob([data]);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  deleteFile: async (fileId: string) => {
    try {
      await tauri.deleteFile(fileId);
      set((state) => ({
        files: state.files.filter((f) => f.id !== fileId),
        selectedFileId:
          state.selectedFileId === fileId ? null : state.selectedFileId,
      }));
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  selectFile: (fileId: string | null) => {
    set({ selectedFileId: fileId, versions: [], shareLinks: [] });
    if (fileId) {
      get().fetchVersions(fileId);
      get().fetchShareLinks(fileId);
    }
  },

  // Version actions
  fetchVersions: async (fileId: string) => {
    try {
      const versions = await tauri.getFileVersions(fileId);
      set({ versions });
    } catch (error) {
      console.error('Failed to fetch versions:', error);
    }
  },

  restoreVersion: async (fileId: string, version: number) => {
    try {
      const restoredFile = await tauri.restoreVersion(fileId, version);
      set((state) => ({
        files: state.files.map((f) => (f.id === fileId ? restoredFile : f)),
      }));
      await get().fetchVersions(fileId);
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  // Share link actions
  fetchShareLinks: async (fileId: string) => {
    try {
      const shareLinks = await tauri.listFileShareLinks(fileId);
      set({ shareLinks });
    } catch (error) {
      console.error('Failed to fetch share links:', error);
    }
  },

  createShareLink: async (
    fileId: string,
    expiresInHours?: number,
    password?: string,
    maxDownloads?: number
  ) => {
    try {
      const link = await tauri.createShareLink(
        fileId,
        expiresInHours,
        password,
        maxDownloads
      );
      set((state) => ({
        shareLinks: [...state.shareLinks, link],
      }));
      return link;
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  revokeShareLink: async (linkId: string) => {
    try {
      await tauri.revokeShareLink(linkId);
      set((state) => ({
        shareLinks: state.shareLinks.filter((l) => l.id !== linkId),
      }));
    } catch (error) {
      set({ error: (error as Error).message });
      throw error;
    }
  },

  // UI state actions
  setSearchQuery: (query: string) => set({ searchQuery: query }),
  setCurrentPath: (path: string) => set({ currentPath: path }),
  setSortBy: (sortBy: SortBy) => set({ sortBy }),
  setSortOrder: (order: SortOrder) => set({ sortOrder: order }),
  removeUpload: (id: string) =>
    set((state) => ({
      uploads: state.uploads.filter((u) => u.id !== id),
    })),

  clearError: () => set({ error: null }),

  getSortedFiles: () => {
    const { files, sortBy, sortOrder, searchQuery, currentPath } = get();

    // Filter by path and search
    let filtered = files.filter((f) => {
      const matchesPath =
        currentPath === '/' ||
        f.path.startsWith(currentPath) ||
        f.name.includes(currentPath.slice(1));
      const matchesSearch =
        !searchQuery ||
        f.name.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesPath && matchesSearch;
    });

    // Sort
    filtered = [...filtered].sort((a, b) => {
      let comparison = 0;

      switch (sortBy) {
        case 'name':
          comparison = a.name.localeCompare(b.name);
          break;
        case 'size':
          comparison = a.size - b.size;
          break;
        case 'date':
          comparison = a.modified_at - b.modified_at;
          break;
        case 'type':
          comparison = (a.mime_type || '').localeCompare(b.mime_type || '');
          break;
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return filtered;
  },
}));
