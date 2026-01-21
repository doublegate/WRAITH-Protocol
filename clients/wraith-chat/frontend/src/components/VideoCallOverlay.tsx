// WRAITH Chat - Video Call Overlay Component

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface VideoCallInfo {
  call_id: string;
  peer_id: string;
  state: VideoCallState;
  direction: 'outgoing' | 'incoming';
  started_at: number;
  connected_at?: number;
  muted: boolean;
  video_enabled: boolean;
  screen_sharing: boolean;
  quality: 'low' | 'medium' | 'high' | 'auto';
  stats: VideoCallStats;
}

type VideoCallState =
  | 'initiating'
  | 'ringing'
  | 'incoming'
  | 'connected'
  | 'reconnecting'
  | 'ended';

interface VideoCallStats {
  duration_secs: number;
  avg_latency_ms: number;
  video_bitrate: number;
  audio_bitrate: number;
  frame_rate: number;
  resolution: string;
}

interface VideoCallOverlayProps {
  peerId?: string;
  incomingCall?: VideoCallInfo;
  onCallEnd?: () => void;
}

export default function VideoCallOverlay({
  peerId,
  incomingCall,
  onCallEnd,
}: VideoCallOverlayProps) {
  const [activeCall, setActiveCall] = useState<VideoCallInfo | null>(null);
  const [callDuration, setCallDuration] = useState(0);
  const [showSettings, setShowSettings] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Start call if peerId is provided
  useEffect(() => {
    if (peerId && !activeCall && !loading) {
      startVideoCall(peerId);
    }
  }, [peerId, activeCall, loading]);

  // Update duration timer
  const isConnected = activeCall?.state === 'connected';
  const connectedAt = activeCall?.connected_at;
  useEffect(() => {
    if (!isConnected || !connectedAt) return;

    const updateDuration = () => {
      const now = Math.floor(Date.now() / 1000);
      setCallDuration(now - connectedAt);
    };
    updateDuration();

    const interval = setInterval(updateDuration, 1000);
    return () => {
      clearInterval(interval);
      setCallDuration(0);
    };
  }, [isConnected, connectedAt]);

  // Handle call end callback
  useEffect(() => {
    if (activeCall?.state === 'ended' && onCallEnd) {
      onCallEnd();
    }
  }, [activeCall?.state, onCallEnd]);

  const startVideoCall = async (targetPeerId: string) => {
    setLoading(true);
    setError(null);
    try {
      const call: VideoCallInfo = await invoke('start_video_call', {
        peerId: targetPeerId,
      });
      setActiveCall(call);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  };

  const handleAnswerCall = useCallback(async () => {
    if (!incomingCall) return;
    setLoading(true);
    try {
      const call: VideoCallInfo = await invoke('accept_incoming_video_call', {
        callId: incomingCall.call_id,
      });
      setActiveCall(call);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }, [incomingCall]);

  const handleRejectCall = useCallback(async () => {
    if (!incomingCall) return;
    try {
      await invoke('reject_incoming_video_call', {
        callId: incomingCall.call_id,
      });
      onCallEnd?.();
    } catch (err) {
      setError((err as Error).message);
    }
  }, [incomingCall, onCallEnd]);

  const handleEndCall = useCallback(async () => {
    if (!activeCall) return;
    try {
      await invoke('end_video_call', { callId: activeCall.call_id });
      setActiveCall(null);
      onCallEnd?.();
    } catch (err) {
      setError((err as Error).message);
    }
  }, [activeCall, onCallEnd]);

  const handleToggleMute = useCallback(async () => {
    if (!activeCall) return;
    try {
      await invoke('toggle_mute', { callId: activeCall.call_id });
      setActiveCall((prev) => (prev ? { ...prev, muted: !prev.muted } : null));
    } catch (err) {
      setError((err as Error).message);
    }
  }, [activeCall]);

  const handleToggleVideo = useCallback(async () => {
    if (!activeCall) return;
    try {
      await invoke('toggle_video', { callId: activeCall.call_id });
      setActiveCall((prev) =>
        prev ? { ...prev, video_enabled: !prev.video_enabled } : null
      );
    } catch (err) {
      setError((err as Error).message);
    }
  }, [activeCall]);

  const handleToggleScreenShare = useCallback(async () => {
    if (!activeCall) return;
    try {
      await invoke('toggle_screen_share', { callId: activeCall.call_id });
      setActiveCall((prev) =>
        prev ? { ...prev, screen_sharing: !prev.screen_sharing } : null
      );
    } catch (err) {
      setError((err as Error).message);
    }
  }, [activeCall]);

  const handleSetQuality = useCallback(
    async (quality: 'low' | 'medium' | 'high' | 'auto') => {
      if (!activeCall) return;
      try {
        await invoke('set_video_quality', {
          callId: activeCall.call_id,
          quality,
        });
        setActiveCall((prev) => (prev ? { ...prev, quality } : null));
      } catch (err) {
        setError((err as Error).message);
      }
    },
    [activeCall]
  );

  // Incoming call UI
  if (incomingCall && !activeCall) {
    return (
      <div className="fixed inset-0 bg-bg-primary/95 flex flex-col items-center justify-center z-50">
        <div className="text-center mb-8 animate-pulse">
          <div className="w-24 h-24 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-4xl font-bold mx-auto mb-4">
            {incomingCall.peer_id.substring(0, 2).toUpperCase()}
          </div>
          <h2 className="text-2xl font-semibold text-white">Incoming Video Call</h2>
          <p className="text-slate-400 mt-2">
            {incomingCall.peer_id.substring(0, 16)}...
          </p>
        </div>

        {error && (
          <div className="bg-red-500/20 text-red-400 px-4 py-2 rounded mb-4">
            {error}
          </div>
        )}

        <div className="flex items-center gap-8">
          <button
            onClick={handleRejectCall}
            className="w-16 h-16 rounded-full bg-red-500 hover:bg-red-600 text-white flex items-center justify-center transition-colors"
            title="Decline"
          >
            <PhoneOffIcon className="w-8 h-8" />
          </button>
          <button
            onClick={handleAnswerCall}
            disabled={loading}
            className="w-16 h-16 rounded-full bg-green-500 hover:bg-green-600 text-white flex items-center justify-center transition-colors disabled:opacity-50"
            title="Answer with video"
          >
            <VideoIcon className="w-8 h-8" />
          </button>
        </div>
      </div>
    );
  }

  // Active call UI
  if (activeCall) {
    return (
      <div className="fixed inset-0 bg-bg-primary z-50 flex flex-col">
        {/* Main Video Area */}
        <div className="flex-1 relative bg-slate-900">
          {/* Remote Video (placeholder) */}
          <div className="absolute inset-0 flex items-center justify-center">
            {activeCall.video_enabled ? (
              <div className="w-full h-full bg-slate-800 flex items-center justify-center">
                <span className="text-slate-500">Remote video feed</span>
              </div>
            ) : (
              <div className="text-center">
                <div className="w-32 h-32 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-6xl font-bold mx-auto mb-4">
                  {activeCall.peer_id.substring(0, 2).toUpperCase()}
                </div>
                <p className="text-slate-400">Camera is off</p>
              </div>
            )}
          </div>

          {/* Local Video (Picture-in-Picture) */}
          <div className="absolute bottom-4 right-4 w-48 h-36 bg-slate-800 rounded-lg border border-slate-700 overflow-hidden">
            <div className="w-full h-full flex items-center justify-center">
              {activeCall.video_enabled ? (
                <span className="text-xs text-slate-500">Your video</span>
              ) : (
                <VideoOffIcon className="w-8 h-8 text-slate-600" />
              )}
            </div>
          </div>

          {/* Call Info Overlay */}
          <div className="absolute top-4 left-4 flex items-center gap-3">
            <div className="px-3 py-1.5 bg-black/50 rounded-full backdrop-blur">
              <span className="text-white text-sm font-medium">
                {activeCall.peer_id.substring(0, 16)}...
              </span>
            </div>
            {activeCall.state === 'connected' && (
              <div className="px-3 py-1.5 bg-black/50 rounded-full backdrop-blur">
                <span className="text-white text-sm">{formatDuration(callDuration)}</span>
              </div>
            )}
            {activeCall.state !== 'connected' && (
              <div className="px-3 py-1.5 bg-black/50 rounded-full backdrop-blur">
                <span className="text-yellow-400 text-sm">{getStateText(activeCall.state)}</span>
              </div>
            )}
          </div>

          {/* Quality Indicator */}
          {activeCall.state === 'connected' && (
            <div className="absolute top-4 right-4">
              <button
                onClick={() => setShowSettings(!showSettings)}
                className="px-3 py-1.5 bg-black/50 rounded-full backdrop-blur text-white text-sm hover:bg-black/70 transition-colors"
              >
                {activeCall.stats.resolution} @ {activeCall.stats.frame_rate}fps
              </button>
            </div>
          )}

          {/* Settings Panel */}
          {showSettings && (
            <div className="absolute top-14 right-4 w-64 bg-bg-secondary rounded-lg border border-slate-700 p-4">
              <h3 className="text-sm font-medium text-white mb-3">Video Quality</h3>
              <div className="space-y-2">
                {(['auto', 'low', 'medium', 'high'] as const).map((q) => (
                  <button
                    key={q}
                    onClick={() => handleSetQuality(q)}
                    className={`w-full text-left px-3 py-2 rounded text-sm transition-colors ${
                      activeCall.quality === q
                        ? 'bg-wraith-primary text-white'
                        : 'text-slate-300 hover:bg-bg-tertiary'
                    }`}
                  >
                    {q.charAt(0).toUpperCase() + q.slice(1)}
                    {q === 'auto' && ' (Recommended)'}
                  </button>
                ))}
              </div>
              <div className="mt-4 pt-4 border-t border-slate-700 text-xs text-slate-500">
                <p>Latency: {activeCall.stats.avg_latency_ms}ms</p>
                <p>Video: {Math.round(activeCall.stats.video_bitrate / 1000)}kbps</p>
                <p>Audio: {Math.round(activeCall.stats.audio_bitrate / 1000)}kbps</p>
              </div>
            </div>
          )}
        </div>

        {/* Error Display */}
        {error && (
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-red-500/20 text-red-400 px-4 py-2 rounded">
            {error}
          </div>
        )}

        {/* Controls Bar */}
        <div className="bg-bg-secondary border-t border-slate-700 px-6 py-4">
          <div className="flex items-center justify-center gap-4">
            {/* Mute Button */}
            <button
              onClick={handleToggleMute}
              className={`w-14 h-14 rounded-full flex items-center justify-center transition-colors ${
                activeCall.muted
                  ? 'bg-red-500 text-white'
                  : 'bg-bg-tertiary text-white hover:bg-slate-600'
              }`}
              title={activeCall.muted ? 'Unmute' : 'Mute'}
            >
              {activeCall.muted ? (
                <MicOffIcon className="w-6 h-6" />
              ) : (
                <MicIcon className="w-6 h-6" />
              )}
            </button>

            {/* Video Toggle Button */}
            <button
              onClick={handleToggleVideo}
              className={`w-14 h-14 rounded-full flex items-center justify-center transition-colors ${
                !activeCall.video_enabled
                  ? 'bg-red-500 text-white'
                  : 'bg-bg-tertiary text-white hover:bg-slate-600'
              }`}
              title={activeCall.video_enabled ? 'Turn off camera' : 'Turn on camera'}
            >
              {activeCall.video_enabled ? (
                <VideoIcon className="w-6 h-6" />
              ) : (
                <VideoOffIcon className="w-6 h-6" />
              )}
            </button>

            {/* Screen Share Button */}
            <button
              onClick={handleToggleScreenShare}
              className={`w-14 h-14 rounded-full flex items-center justify-center transition-colors ${
                activeCall.screen_sharing
                  ? 'bg-wraith-primary text-white'
                  : 'bg-bg-tertiary text-white hover:bg-slate-600'
              }`}
              title={activeCall.screen_sharing ? 'Stop sharing' : 'Share screen'}
            >
              <ScreenShareIcon className="w-6 h-6" />
            </button>

            {/* End Call Button */}
            <button
              onClick={handleEndCall}
              className="w-16 h-16 rounded-full bg-red-500 hover:bg-red-600 text-white flex items-center justify-center transition-colors"
              title="End call"
            >
              <PhoneOffIcon className="w-8 h-8" />
            </button>
          </div>
        </div>
      </div>
    );
  }

  return null;
}

// Helper functions
function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function getStateText(state: VideoCallState): string {
  switch (state) {
    case 'initiating':
      return 'Connecting...';
    case 'ringing':
      return 'Ringing...';
    case 'incoming':
      return 'Incoming call';
    case 'connected':
      return 'Connected';
    case 'reconnecting':
      return 'Reconnecting...';
    case 'ended':
      return 'Call ended';
    default:
      return state;
  }
}

// Icons
function PhoneOffIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.516l2.257-1.13a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
    </svg>
  );
}

function VideoIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
    </svg>
  );
}

function VideoOffIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 3l18 18" />
    </svg>
  );
}

function MicIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
    </svg>
  );
}

function MicOffIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
    </svg>
  );
}

function ScreenShareIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
    </svg>
  );
}
