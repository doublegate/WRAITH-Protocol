// UI Store (Zustand) - UI state management

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Toast, ViewMode } from '../types';
import * as tauri from '../lib/tauri';

interface UiState {
  // Identity
  peerId: string | null;
  displayName: string;

  // UI State
  viewMode: ViewMode;
  sidebarCollapsed: boolean;
  showActivityPanel: boolean;
  activeModal: string | null;
  modalData: unknown;
  toasts: Toast[];

  // Actions - Identity
  fetchIdentity: () => Promise<void>;
  setDisplayName: (name: string) => Promise<void>;

  // Actions - UI
  setViewMode: (mode: ViewMode) => void;
  toggleSidebar: () => void;
  toggleActivityPanel: () => void;
  openModal: (modalId: string, data?: unknown) => void;
  closeModal: () => void;

  // Actions - Toasts
  addToast: (
    type: Toast['type'],
    message: string,
    duration?: number
  ) => void;
  removeToast: (id: string) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set, get) => ({
      // Initial state
      peerId: null,
      displayName: 'Anonymous',
      viewMode: 'grid',
      sidebarCollapsed: false,
      showActivityPanel: false,
      activeModal: null,
      modalData: null,
      toasts: [],

      // Identity actions
      fetchIdentity: async () => {
        try {
          const [peerId, displayName] = await Promise.all([
            tauri.getPeerId(),
            tauri.getDisplayName(),
          ]);
          set({ peerId, displayName });
        } catch (error) {
          console.error('Failed to fetch identity:', error);
        }
      },

      setDisplayName: async (name: string) => {
        try {
          await tauri.setDisplayName(name);
          set({ displayName: name });
        } catch (error) {
          console.error('Failed to set display name:', error);
          throw error;
        }
      },

      // UI actions
      setViewMode: (mode: ViewMode) => set({ viewMode: mode }),

      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      toggleActivityPanel: () =>
        set((state) => ({ showActivityPanel: !state.showActivityPanel })),

      openModal: (modalId: string, data?: unknown) =>
        set({ activeModal: modalId, modalData: data }),

      closeModal: () => set({ activeModal: null, modalData: null }),

      // Toast actions
      addToast: (
        type: Toast['type'],
        message: string,
        duration: number = 5000
      ) => {
        const id = crypto.randomUUID();
        const toast: Toast = { id, type, message, duration };

        set((state) => ({
          toasts: [...state.toasts, toast],
        }));

        // Auto-remove after duration
        if (duration > 0) {
          setTimeout(() => {
            get().removeToast(id);
          }, duration);
        }
      },

      removeToast: (id: string) =>
        set((state) => ({
          toasts: state.toasts.filter((t) => t.id !== id),
        })),
    }),
    {
      name: 'wraith-share-ui',
      partialize: (state) => ({
        viewMode: state.viewMode,
        sidebarCollapsed: state.sidebarCollapsed,
      }),
    }
  )
);
