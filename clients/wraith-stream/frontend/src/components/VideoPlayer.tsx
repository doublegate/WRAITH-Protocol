import { useEffect, useRef, useCallback } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useStreamStore } from '../stores/streamStore';
import QualitySelector from './QualitySelector';

// SVG Icons
const PlayIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
    <polygon points="5,3 19,12 5,21" />
  </svg>
);

const PauseIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
    <rect x="6" y="4" width="4" height="16" />
    <rect x="14" y="4" width="4" height="16" />
  </svg>
);

const VolumeIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" />
    <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07" />
  </svg>
);

const MuteIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" />
    <line x1="23" y1="9" x2="17" y2="15" />
    <line x1="17" y1="9" x2="23" y2="15" />
  </svg>
);

const FullscreenIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="15,3 21,3 21,9" />
    <polyline points="9,21 3,21 3,15" />
    <line x1="21" y1="3" x2="14" y2="10" />
    <line x1="3" y1="21" x2="10" y2="14" />
  </svg>
);

const BackIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="19" y1="12" x2="5" y2="12" />
    <polyline points="12,19 5,12 12,5" />
  </svg>
);

const SettingsIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const formatTime = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
};

export default function VideoPlayer() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const { selectedStream, setCurrentView, clearSelectedStream } = useStreamStore();
  const {
    player,
    playbackInfo,
    setVideoRef,
    loadStream,
    togglePlay,
    seek,
    setVolume,
    toggleMute,
    toggleFullscreen,
    updateCurrentTime,
    setDuration,
    setBuffering,
    reset,
  } = usePlayerStore();

  const [showQualitySelector, setShowQualitySelector] = React.useState(false);

  // Load stream when selected
  useEffect(() => {
    if (selectedStream) {
      loadStream(selectedStream.id);
    }
    return () => {
      reset();
    };
  }, [selectedStream, loadStream, reset]);

  // Set video ref
  useEffect(() => {
    setVideoRef(videoRef.current);
  }, [setVideoRef]);

  const handleBack = useCallback(() => {
    clearSelectedStream();
    setCurrentView('browse');
  }, [clearSelectedStream, setCurrentView]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;

      switch (e.key) {
        case ' ':
        case 'k':
          e.preventDefault();
          togglePlay();
          break;
        case 'f':
          toggleFullscreen();
          break;
        case 'm':
          toggleMute();
          break;
        case 'ArrowLeft':
          seek(Math.max(0, player.current_time - 10));
          break;
        case 'ArrowRight':
          seek(Math.min(player.duration, player.current_time + 10));
          break;
        case 'ArrowUp':
          setVolume(Math.min(1, player.volume + 0.1));
          break;
        case 'ArrowDown':
          setVolume(Math.max(0, player.volume - 0.1));
          break;
        case 'Escape':
          handleBack();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [togglePlay, toggleFullscreen, toggleMute, seek, setVolume, player, handleBack]);

  const handleSeekBarClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = (e.clientX - rect.left) / rect.width;
    seek(percent * player.duration);
  };

  const handleVolumeChange = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = (e.clientX - rect.left) / rect.width;
    setVolume(Math.max(0, Math.min(1, percent)));
  };

  if (!selectedStream || !playbackInfo) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-[var(--color-text-muted)]">Loading stream...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full gap-6">
      {/* Back button and title */}
      <div className="flex items-center gap-4">
        <button
          onClick={handleBack}
          className="player-button hover:bg-[var(--color-bg-hover)] text-[var(--color-text-primary)]"
        >
          <BackIcon />
        </button>
        <div>
          <h1 className="text-xl font-semibold text-[var(--color-text-primary)]">
            {selectedStream.title}
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            {selectedStream.view_count.toLocaleString()} views - {selectedStream.created_by}
          </p>
        </div>
      </div>

      {/* Video container */}
      <div
        ref={containerRef}
        className="video-container flex-1 relative"
      >
        <video
          ref={videoRef}
          className="w-full h-full"
          onTimeUpdate={() => {
            if (videoRef.current) {
              updateCurrentTime(videoRef.current.currentTime);
            }
          }}
          onDurationChange={() => {
            if (videoRef.current) {
              setDuration(videoRef.current.duration);
            }
          }}
          onWaiting={() => setBuffering(true)}
          onCanPlay={() => setBuffering(false)}
          onClick={togglePlay}
        >
          {/* In a real implementation, we would set the src to HLS manifest */}
          <source src={playbackInfo.manifest_url} type="application/x-mpegURL" />
        </video>

        {/* Buffering indicator */}
        {player.is_buffering && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/50">
            <div className="w-12 h-12 border-4 border-[var(--color-primary-500)] border-t-transparent rounded-full animate-spin" />
          </div>
        )}

        {/* Player controls */}
        <div className="player-controls">
          {/* Progress bar */}
          <div
            className="seek-bar"
            onClick={handleSeekBarClick}
          >
            <div
              className="seek-bar-fill"
              style={{ width: `${(player.current_time / player.duration) * 100}%` }}
            />
            <div
              className="seek-bar-thumb"
              style={{ left: `${(player.current_time / player.duration) * 100}%` }}
            />
          </div>

          {/* Controls row */}
          <div className="player-controls-row">
            {/* Play/Pause */}
            <button className="player-button" onClick={togglePlay}>
              {player.is_playing ? <PauseIcon /> : <PlayIcon />}
            </button>

            {/* Volume */}
            <button className="player-button" onClick={toggleMute}>
              {player.is_muted || player.volume === 0 ? <MuteIcon /> : <VolumeIcon />}
            </button>
            <div
              className="volume-slider"
              onClick={handleVolumeChange}
            >
              <div
                className="volume-slider-fill"
                style={{ width: `${player.is_muted ? 0 : player.volume * 100}%` }}
              />
            </div>

            {/* Time */}
            <span className="player-time">
              {formatTime(player.current_time)} / {formatTime(player.duration)}
            </span>

            {/* Spacer */}
            <div className="flex-1" />

            {/* Quality selector */}
            <div className="relative">
              <button
                className="player-button"
                onClick={() => setShowQualitySelector(!showQualitySelector)}
              >
                <SettingsIcon />
              </button>
              {showQualitySelector && (
                <QualitySelector onClose={() => setShowQualitySelector(false)} />
              )}
            </div>

            {/* Fullscreen */}
            <button className="player-button" onClick={toggleFullscreen}>
              <FullscreenIcon />
            </button>
          </div>
        </div>
      </div>

      {/* Stream info */}
      <div className="card p-4">
        <h2 className="font-semibold text-[var(--color-text-primary)] mb-2">
          {selectedStream.title}
        </h2>
        {selectedStream.description && (
          <p className="text-sm text-[var(--color-text-secondary)] mb-4">
            {selectedStream.description}
          </p>
        )}
        <div className="flex items-center gap-4 text-sm text-[var(--color-text-muted)]">
          {selectedStream.category && (
            <span className="category-pill">{selectedStream.category}</span>
          )}
          {selectedStream.tags && (
            <div className="flex gap-2">
              {selectedStream.tags.split(',').map((tag, i) => (
                <span key={i} className="text-[var(--color-primary-400)]">#{tag.trim()}</span>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// Import React for useState
import React from 'react';
