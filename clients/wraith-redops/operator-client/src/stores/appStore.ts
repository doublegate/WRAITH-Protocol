import { create } from 'zustand';
import type { Implant, Campaign, Listener, StreamEvent } from '../types';
import * as ipc from '../lib/ipc';
import { useToastStore } from './toastStore';

interface AppState {
  // Navigation
  activeTab: string;
  setActiveTab: (tab: string) => void;

  // Connection
  serverAddress: string;
  setServerAddress: (addr: string) => void;
  serverStatus: string;
  connect: (address?: string) => Promise<void>;

  // Data
  implants: Implant[];
  campaigns: Campaign[];
  listeners: Listener[];
  events: StreamEvent[];
  addEvent: (event: StreamEvent) => void;

  // Interaction
  interactingImplantId: string | null;
  setInteractingImplantId: (id: string | null) => void;

  // Campaign creation
  showCreateCampaign: boolean;
  setShowCreateCampaign: (show: boolean) => void;

  // Auth
  authToken: string;
  setAuthToken: (token: string) => void;
  doRefreshToken: () => Promise<void>;

  // Settings
  autoRefreshInterval: number;
  setAutoRefreshInterval: (ms: number) => void;

  // Theme
  theme: 'dark' | 'light';
  toggleTheme: () => void;

  // Refresh
  refreshAll: () => Promise<void>;
  refreshImplants: () => Promise<void>;
  refreshCampaigns: () => Promise<void>;
  refreshListeners: () => Promise<void>;
}

const toast = () => useToastStore.getState().addToast;

export const useAppStore = create<AppState>((set, get) => ({
  // Navigation
  activeTab: 'dashboard',
  setActiveTab: (tab) => set({ activeTab: tab }),

  // Connection
  serverAddress: '127.0.0.1:50051',
  setServerAddress: (addr) => set({ serverAddress: addr }),
  serverStatus: 'Disconnected',

  connect: async (address?: string) => {
    const addr = address ?? get().serverAddress;
    try {
      set({ serverStatus: 'Connecting...' });
      await ipc.connectToServer(addr);
      set({ serverStatus: 'Connected' });
      get().refreshAll();
      // Start event stream
      ipc.streamEvents().catch(() => {
        // Event stream ended or failed silently
      });
    } catch (e) {
      set({ serverStatus: 'Error: ' + e });
    }
  },

  // Data
  implants: [],
  campaigns: [],
  listeners: [],
  events: [],
  addEvent: (event) =>
    set((state) => ({
      events: [event, ...state.events].slice(0, 500),
    })),

  // Interaction
  interactingImplantId: null,
  setInteractingImplantId: (id) => set({ interactingImplantId: id }),

  // Campaign creation
  showCreateCampaign: false,
  setShowCreateCampaign: (show) => set({ showCreateCampaign: show }),

  // Auth
  authToken: '',
  setAuthToken: (token) => set({ authToken: token }),
  doRefreshToken: async () => {
    const { authToken } = get();
    if (!authToken) return;
    try {
      const newToken = await ipc.refreshToken(authToken);
      set({ authToken: newToken });
    } catch (e) {
      toast()('error', 'Token refresh failed: ' + e);
    }
  },

  // Settings
  autoRefreshInterval: 5000,
  setAutoRefreshInterval: (ms) => set({ autoRefreshInterval: ms }),

  // Theme
  theme: 'dark',
  toggleTheme: () => {
    const next = get().theme === 'dark' ? 'light' : 'dark';
    set({ theme: next });
    if (next === 'dark') document.documentElement.classList.add('dark');
    else document.documentElement.classList.remove('dark');
  },

  // Refresh
  refreshAll: async () => {
    const { refreshImplants, refreshCampaigns, refreshListeners } = get();
    await Promise.allSettled([refreshImplants(), refreshCampaigns(), refreshListeners()]);
  },

  refreshImplants: async () => {
    try {
      const implants = await ipc.listImplants();
      set({ implants });
    } catch {
      // Silently fail during polling
    }
  },

  refreshCampaigns: async () => {
    try {
      const campaigns = await ipc.listCampaigns();
      set({ campaigns });
    } catch {
      // Silently fail during polling
    }
  },

  refreshListeners: async () => {
    try {
      const listeners = await ipc.listListeners();
      set({ listeners });
    } catch {
      // Silently fail during polling
    }
  },
}));
