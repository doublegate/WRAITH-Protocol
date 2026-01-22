import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { PlayerState, PlaybackInfo, SubtitleCue, SubtitleInfo } from '../types';

interface PlayerStore {
  // Player state
  player: PlayerState;
  playbackInfo: PlaybackInfo | null;
  subtitles: SubtitleCue[];
  availableSubtitles: SubtitleInfo[];
  currentSubtitle: string | null;

  // Video element reference
  videoRef: HTMLVideoElement | null;
  setVideoRef: (ref: HTMLVideoElement | null) => void;

  // Playback actions
  loadStream: (streamId: string) => Promise<void>;
  play: () => void;
  pause: () => void;
  togglePlay: () => void;
  seek: (time: number) => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  setQuality: (quality: string) => Promise<void>;
  setPlaybackRate: (rate: number) => void;
  toggleFullscreen: () => void;

  // Time updates
  updateCurrentTime: (time: number) => void;
  setDuration: (duration: number) => void;
  setBuffering: (isBuffering: boolean) => void;

  // Subtitles
  loadSubtitles: (streamId: string, language: string) => Promise<void>;
  setSubtitleLanguage: (language: string | null) => void;
  fetchAvailableSubtitles: (streamId: string) => Promise<void>;

  // Cleanup
  reset: () => void;
}

const initialPlayerState: PlayerState = {
  stream_id: null,
  is_playing: false,
  is_buffering: false,
  current_time: 0,
  duration: 0,
  volume: 1,
  is_muted: false,
  quality: 'auto',
  available_qualities: [],
  is_fullscreen: false,
  playback_rate: 1,
};

export const usePlayerStore = create<PlayerStore>((set, get) => ({
  player: initialPlayerState,
  playbackInfo: null,
  subtitles: [],
  availableSubtitles: [],
  currentSubtitle: null,
  videoRef: null,

  setVideoRef: (ref) => set({ videoRef: ref }),

  loadStream: async (streamId) => {
    try {
      // Get playback info from backend
      const playbackInfo = await invoke<PlaybackInfo>('get_playback_info', { streamId });

      set({
        playbackInfo,
        player: {
          ...initialPlayerState,
          stream_id: streamId,
          available_qualities: playbackInfo.qualities.map(q => q.name),
          quality: playbackInfo.qualities.length > 0 ? playbackInfo.qualities[playbackInfo.qualities.length - 1].name : 'auto',
        },
      });

      // Fetch available subtitles
      await get().fetchAvailableSubtitles(streamId);

    } catch (error) {
      console.error('Failed to load stream:', error);
      throw error;
    }
  },

  play: () => {
    const { videoRef } = get();
    if (videoRef) {
      videoRef.play();
      set((state) => ({ player: { ...state.player, is_playing: true } }));
    }
  },

  pause: () => {
    const { videoRef } = get();
    if (videoRef) {
      videoRef.pause();
      set((state) => ({ player: { ...state.player, is_playing: false } }));
    }
  },

  togglePlay: () => {
    const { player } = get();
    if (player.is_playing) {
      get().pause();
    } else {
      get().play();
    }
  },

  seek: (time) => {
    const { videoRef } = get();
    if (videoRef) {
      videoRef.currentTime = time;
      set((state) => ({ player: { ...state.player, current_time: time } }));
    }
  },

  setVolume: (volume) => {
    const { videoRef } = get();
    if (videoRef) {
      videoRef.volume = volume;
      set((state) => ({
        player: {
          ...state.player,
          volume,
          is_muted: volume === 0,
        },
      }));
    }
  },

  toggleMute: () => {
    const { videoRef, player } = get();
    if (videoRef) {
      const newMuted = !player.is_muted;
      videoRef.muted = newMuted;
      set((state) => ({ player: { ...state.player, is_muted: newMuted } }));
    }
  },

  setQuality: async (quality) => {
    const { player } = get();
    if (player.stream_id) {
      try {
        await invoke('set_quality', { streamId: player.stream_id, quality });
        set((state) => ({ player: { ...state.player, quality } }));
      } catch (error) {
        console.error('Failed to set quality:', error);
      }
    }
  },

  setPlaybackRate: (rate) => {
    const { videoRef } = get();
    if (videoRef) {
      videoRef.playbackRate = rate;
      set((state) => ({ player: { ...state.player, playback_rate: rate } }));
    }
  },

  toggleFullscreen: () => {
    const { videoRef, player } = get();
    if (!videoRef) return;

    const container = videoRef.parentElement;
    if (!container) return;

    if (!player.is_fullscreen) {
      if (container.requestFullscreen) {
        container.requestFullscreen();
      }
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
      }
    }
    set((state) => ({ player: { ...state.player, is_fullscreen: !state.player.is_fullscreen } }));
  },

  updateCurrentTime: (time) => {
    set((state) => ({ player: { ...state.player, current_time: time } }));
  },

  setDuration: (duration) => {
    set((state) => ({ player: { ...state.player, duration } }));
  },

  setBuffering: (isBuffering) => {
    set((state) => ({ player: { ...state.player, is_buffering: isBuffering } }));
  },

  loadSubtitles: async (streamId, language) => {
    try {
      const subtitles = await invoke<SubtitleCue[]>('load_subtitles', { streamId, language });
      set({ subtitles, currentSubtitle: language });
    } catch (error) {
      console.error('Failed to load subtitles:', error);
      set({ subtitles: [], currentSubtitle: null });
    }
  },

  setSubtitleLanguage: (language) => {
    const { player } = get();
    if (language && player.stream_id) {
      get().loadSubtitles(player.stream_id, language);
    } else {
      set({ subtitles: [], currentSubtitle: null });
    }
  },

  fetchAvailableSubtitles: async (streamId) => {
    try {
      const availableSubtitles = await invoke<SubtitleInfo[]>('list_subtitle_languages', { streamId });
      set({ availableSubtitles });
    } catch (error) {
      console.error('Failed to fetch subtitles:', error);
      set({ availableSubtitles: [] });
    }
  },

  reset: () => {
    set({
      player: initialPlayerState,
      playbackInfo: null,
      subtitles: [],
      availableSubtitles: [],
      currentSubtitle: null,
    });
  },
}));
