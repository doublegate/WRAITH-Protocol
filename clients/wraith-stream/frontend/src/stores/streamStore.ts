import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { StreamInfo, TranscodeProgress, SearchResults, StreamSummary, ViewState, UploadState } from '../types';

interface StreamState {
  // View state
  currentView: ViewState;
  setCurrentView: (view: ViewState) => void;

  // Streams
  streams: StreamInfo[];
  myStreams: StreamInfo[];
  trendingStreams: StreamSummary[];
  searchResults: SearchResults | null;
  selectedStream: StreamInfo | null;
  isLoading: boolean;
  error: string | null;

  // Upload state
  upload: UploadState;

  // Actions
  fetchStreams: (limit?: number, offset?: number) => Promise<void>;
  fetchMyStreams: () => Promise<void>;
  fetchTrendingStreams: (limit?: number) => Promise<void>;
  searchStreams: (query: string, limit?: number) => Promise<void>;
  selectStream: (streamId: string) => Promise<void>;
  clearSelectedStream: () => void;

  // Upload actions
  setUploadFile: (file: File | null) => void;
  setUploadTitle: (title: string) => void;
  setUploadDescription: (description: string) => void;
  setUploadCategory: (category: string) => void;
  setUploadTags: (tags: string) => void;
  startUpload: () => Promise<void>;
  cancelUpload: () => void;
  resetUpload: () => void;

  // Stream management
  createStream: (title: string, description?: string, category?: string, tags?: string) => Promise<StreamInfo>;
  deleteStream: (streamId: string) => Promise<void>;
  updateStream: (streamId: string, updates: Partial<Pick<StreamInfo, 'title' | 'description' | 'category' | 'tags'>>) => Promise<void>;

  // Transcode progress
  transcodeProgress: TranscodeProgress | null;
  pollTranscodeProgress: (streamId: string) => Promise<void>;
}

const initialUploadState: UploadState = {
  file: null,
  title: '',
  description: '',
  category: '',
  tags: '',
  status: 'idle',
  progress: 0,
  streamId: null,
  error: null,
};

export const useStreamStore = create<StreamState>((set, get) => ({
  // Initial state
  currentView: 'browse',
  streams: [],
  myStreams: [],
  trendingStreams: [],
  searchResults: null,
  selectedStream: null,
  isLoading: false,
  error: null,
  upload: initialUploadState,
  transcodeProgress: null,

  setCurrentView: (view) => set({ currentView: view }),

  fetchStreams: async (limit = 50, offset = 0) => {
    set({ isLoading: true, error: null });
    try {
      const streams = await invoke<StreamInfo[]>('list_streams', { limit, offset });
      set({ streams, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchMyStreams: async () => {
    set({ isLoading: true, error: null });
    try {
      const myStreams = await invoke<StreamInfo[]>('get_my_streams');
      set({ myStreams, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchTrendingStreams: async (limit = 10) => {
    try {
      const trendingStreams = await invoke<StreamSummary[]>('get_trending_streams', { limit });
      set({ trendingStreams });
    } catch (error) {
      console.error('Failed to fetch trending streams:', error);
    }
  },

  searchStreams: async (query, limit = 20) => {
    set({ isLoading: true, error: null });
    try {
      const searchResults = await invoke<SearchResults>('search_streams', { query, limit });
      set({ searchResults, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  selectStream: async (streamId) => {
    set({ isLoading: true, error: null });
    try {
      const selectedStream = await invoke<StreamInfo | null>('get_stream', { streamId });
      if (selectedStream) {
        // Record view
        await invoke('record_view', { streamId });
        set({ selectedStream, isLoading: false, currentView: 'player' });
      } else {
        set({ error: 'Stream not found', isLoading: false });
      }
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  clearSelectedStream: () => set({ selectedStream: null }),

  // Upload actions
  setUploadFile: (file) => set((state) => ({
    upload: { ...state.upload, file, title: file?.name.replace(/\.[^/.]+$/, '') || '' }
  })),

  setUploadTitle: (title) => set((state) => ({
    upload: { ...state.upload, title }
  })),

  setUploadDescription: (description) => set((state) => ({
    upload: { ...state.upload, description }
  })),

  setUploadCategory: (category) => set((state) => ({
    upload: { ...state.upload, category }
  })),

  setUploadTags: (tags) => set((state) => ({
    upload: { ...state.upload, tags }
  })),

  startUpload: async () => {
    const { upload } = get();
    if (!upload.file) return;

    set((state) => ({
      upload: { ...state.upload, status: 'uploading', progress: 0, error: null }
    }));

    try {
      // Create the stream first
      const stream = await get().createStream(
        upload.title,
        upload.description || undefined,
        upload.category || undefined,
        upload.tags || undefined
      );

      set((state) => ({
        upload: { ...state.upload, streamId: stream.id, status: 'transcoding' }
      }));

      // Start transcoding by uploading the video
      // Note: In a real implementation, we would need to handle file path properly
      // For Tauri, we'd use the file dialog to get the actual file path
      const filePath = (upload.file as File & { path?: string }).path || upload.file.name;

      await invoke('upload_video', {
        streamId: stream.id,
        filePath,
      });

      set((state) => ({
        upload: { ...state.upload, status: 'complete', progress: 100 }
      }));

      // Refresh streams
      await get().fetchMyStreams();

    } catch (error) {
      set((state) => ({
        upload: { ...state.upload, status: 'error', error: String(error) }
      }));
    }
  },

  cancelUpload: () => {
    const { upload } = get();
    if (upload.streamId) {
      invoke('cancel_transcode', { streamId: upload.streamId }).catch(console.error);
    }
    set({ upload: initialUploadState });
  },

  resetUpload: () => set({ upload: initialUploadState }),

  // Stream management
  createStream: async (title, description, category, tags) => {
    const stream = await invoke<StreamInfo>('create_stream', {
      title,
      description: description || null,
      category: category || null,
      tags: tags || null,
    });
    return stream as unknown as StreamInfo;
  },

  deleteStream: async (streamId) => {
    await invoke('delete_stream', { streamId });
    set((state) => ({
      streams: state.streams.filter((s) => s.id !== streamId),
      myStreams: state.myStreams.filter((s) => s.id !== streamId),
    }));
  },

  updateStream: async (streamId, updates) => {
    await invoke('update_stream', {
      streamId,
      title: updates.title || null,
      description: updates.description || null,
      category: updates.category || null,
      tags: updates.tags || null,
    });

    // Refresh the stream
    const updatedStream = await invoke<StreamInfo | null>('get_stream', { streamId });
    if (updatedStream) {
      set((state) => ({
        streams: state.streams.map((s) => s.id === streamId ? updatedStream : s),
        myStreams: state.myStreams.map((s) => s.id === streamId ? updatedStream : s),
        selectedStream: state.selectedStream?.id === streamId ? updatedStream : state.selectedStream,
      }));
    }
  },

  // Transcode progress polling
  pollTranscodeProgress: async (streamId) => {
    try {
      const progress = await invoke<TranscodeProgress | null>('get_transcode_progress', { streamId });
      set({ transcodeProgress: progress });

      if (progress) {
        set((state) => ({
          upload: { ...state.upload, progress: progress.progress * 100 }
        }));
      }
    } catch (error) {
      console.error('Failed to poll transcode progress:', error);
    }
  },
}));
