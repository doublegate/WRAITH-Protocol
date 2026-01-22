import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { EditorMode, ViewMode } from '../types';

interface UIState {
  // View state
  viewMode: ViewMode;
  editorMode: EditorMode;
  sidebarCollapsed: boolean;
  showPublishModal: boolean;
  showSettingsModal: boolean;

  // Identity
  peerId: string | null;
  displayName: string;

  // Notifications
  notification: NotificationData | null;

  // Actions
  setViewMode: (mode: ViewMode) => void;
  setEditorMode: (mode: EditorMode) => void;
  toggleSidebar: () => void;
  setShowPublishModal: (show: boolean) => void;
  setShowSettingsModal: (show: boolean) => void;
  setPeerId: (id: string | null) => void;
  setDisplayName: (name: string) => void;
  showNotification: (notification: NotificationData) => void;
  clearNotification: () => void;
}

interface NotificationData {
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
  duration?: number;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      // Initial state
      viewMode: 'list',
      editorMode: 'split',
      sidebarCollapsed: false,
      showPublishModal: false,
      showSettingsModal: false,
      peerId: null,
      displayName: 'Anonymous',
      notification: null,

      // Actions
      setViewMode: (mode) => set({ viewMode: mode }),
      setEditorMode: (mode) => set({ editorMode: mode }),
      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setShowPublishModal: (show) => set({ showPublishModal: show }),
      setShowSettingsModal: (show) => set({ showSettingsModal: show }),
      setPeerId: (id) => set({ peerId: id }),
      setDisplayName: (name) => set({ displayName: name }),
      showNotification: (notification) => set({ notification }),
      clearNotification: () => set({ notification: null }),
    }),
    {
      name: 'wraith-publish-ui',
      partialize: (state) => ({
        editorMode: state.editorMode,
        sidebarCollapsed: state.sidebarCollapsed,
      }),
    }
  )
);

// Auto-clear notifications
useUIStore.subscribe((state) => {
  if (state.notification) {
    const duration = state.notification.duration ?? 3000;
    setTimeout(() => {
      useUIStore.getState().clearNotification();
    }, duration);
  }
});
