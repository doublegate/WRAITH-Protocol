// Voice Call Component - Sprint 17.5

import { useEffect, useState, useCallback } from 'react';
import {
  useCallStore,
  formatCallDuration,
  getCallStateText,
  type CallInfo,
} from '../stores/callStore';

interface VoiceCallProps {
  /** The peer to call or the active call info */
  peerId?: string;
  /** Callback when call ends */
  onCallEnd?: () => void;
}

export default function VoiceCall({ peerId, onCallEnd }: VoiceCallProps) {
  const {
    activeCall,
    incomingCall,
    loading,
    error,
    startCall,
    answerCall,
    rejectCall,
    endCall,
    toggleMute,
    toggleSpeaker,
    refreshCallInfo,
    loadAudioDevices,
    inputDevices,
    outputDevices,
    selectedInputDevice,
    selectedOutputDevice,
    setInputDevice,
    setOutputDevice,
  } = useCallStore();

  const [callDuration, setCallDuration] = useState(0);
  const [showSettings, setShowSettings] = useState(false);

  // Load audio devices on mount
  useEffect(() => {
    loadAudioDevices();
  }, [loadAudioDevices]);

  // Start call if peerId is provided and no active call
  useEffect(() => {
    if (peerId && !activeCall && !loading) {
      startCall(peerId).catch(console.error);
    }
  }, [peerId, activeCall, loading, startCall]);

  // Update call duration timer
  useEffect(() => {
    if (!activeCall || activeCall.state !== 'connected') {
      setCallDuration(0);
      return;
    }

    const interval = setInterval(() => {
      if (activeCall.connected_at) {
        const now = Math.floor(Date.now() / 1000);
        setCallDuration(now - activeCall.connected_at);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [activeCall]);

  // Refresh call info periodically
  useEffect(() => {
    if (!activeCall) return;

    const interval = setInterval(() => {
      refreshCallInfo(activeCall.call_id);
    }, 5000);

    return () => clearInterval(interval);
  }, [activeCall, refreshCallInfo]);

  // Handle call end callback
  useEffect(() => {
    if (activeCall?.state === 'ended' && onCallEnd) {
      onCallEnd();
    }
  }, [activeCall?.state, onCallEnd]);

  const handleEndCall = useCallback(async () => {
    if (activeCall) {
      await endCall(activeCall.call_id);
      onCallEnd?.();
    }
  }, [activeCall, endCall, onCallEnd]);

  const handleToggleMute = useCallback(async () => {
    if (activeCall) {
      await toggleMute(activeCall.call_id);
    }
  }, [activeCall, toggleMute]);

  const handleToggleSpeaker = useCallback(async () => {
    if (activeCall) {
      await toggleSpeaker(activeCall.call_id);
    }
  }, [activeCall, toggleSpeaker]);

  // Render incoming call UI
  if (incomingCall) {
    return (
      <IncomingCallView
        call={incomingCall}
        onAnswer={() => answerCall(incomingCall.call_id)}
        onReject={() => rejectCall(incomingCall.call_id)}
      />
    );
  }

  // Render active call UI
  if (activeCall) {
    return (
      <div className="fixed inset-0 bg-wraith-darker/95 flex flex-col items-center justify-center z-50">
        {/* Caller Info */}
        <div className="text-center mb-8">
          <div className="w-24 h-24 rounded-full bg-wraith-primary flex items-center justify-center text-4xl font-bold mx-auto mb-4">
            {activeCall.peer_id.substring(0, 2).toUpperCase()}
          </div>
          <h2 className="text-2xl font-semibold">
            {activeCall.peer_id.substring(0, 16)}...
          </h2>
          <p className="text-gray-400 mt-2">
            {activeCall.state === 'connected'
              ? formatCallDuration(callDuration)
              : getCallStateText(activeCall.state)}
          </p>
        </div>

        {/* Call Stats (when connected) */}
        {activeCall.state === 'connected' && (
          <div className="text-sm text-gray-500 mb-8">
            <span className="mr-4">Latency: {activeCall.stats.avg_latency_ms.toFixed(0)}ms</span>
            <span>Quality: {getCallQuality(activeCall.stats.packets_lost, activeCall.stats.packets_received)}</span>
          </div>
        )}

        {/* Error Display */}
        {error && (
          <div className="bg-red-500/20 text-red-400 px-4 py-2 rounded mb-4">
            {error}
          </div>
        )}

        {/* Call Controls */}
        <div className="flex items-center gap-6">
          {/* Mute Button */}
          <button
            onClick={handleToggleMute}
            className={`w-14 h-14 rounded-full flex items-center justify-center transition ${
              activeCall.muted
                ? 'bg-red-500 text-white'
                : 'bg-gray-700 text-white hover:bg-gray-600'
            }`}
            title={activeCall.muted ? 'Unmute' : 'Mute'}
          >
            {activeCall.muted ? (
              <MicOffIcon className="w-6 h-6" />
            ) : (
              <MicIcon className="w-6 h-6" />
            )}
          </button>

          {/* End Call Button */}
          <button
            onClick={handleEndCall}
            className="w-16 h-16 rounded-full bg-red-500 hover:bg-red-600 text-white flex items-center justify-center transition"
            title="End Call"
          >
            <PhoneOffIcon className="w-8 h-8" />
          </button>

          {/* Speaker Button */}
          <button
            onClick={handleToggleSpeaker}
            className={`w-14 h-14 rounded-full flex items-center justify-center transition ${
              activeCall.speaker_on
                ? 'bg-wraith-primary text-white'
                : 'bg-gray-700 text-white hover:bg-gray-600'
            }`}
            title={activeCall.speaker_on ? 'Use Earpiece' : 'Use Speaker'}
          >
            {activeCall.speaker_on ? (
              <SpeakerHighIcon className="w-6 h-6" />
            ) : (
              <SpeakerIcon className="w-6 h-6" />
            )}
          </button>

          {/* Settings Button */}
          <button
            onClick={() => setShowSettings(!showSettings)}
            className="w-14 h-14 rounded-full bg-gray-700 text-white hover:bg-gray-600 flex items-center justify-center transition"
            title="Audio Settings"
          >
            <SettingsIcon className="w-6 h-6" />
          </button>
        </div>

        {/* Audio Settings Panel */}
        {showSettings && (
          <div className="mt-8 bg-wraith-dark p-6 rounded-lg w-80">
            <h3 className="font-semibold mb-4">Audio Settings</h3>

            {/* Input Device */}
            <label className="block mb-4">
              <span className="text-sm text-gray-400">Microphone</span>
              <select
                value={selectedInputDevice || ''}
                onChange={(e) => setInputDevice(e.target.value || null)}
                className="w-full mt-1 p-2 bg-wraith-darker border border-gray-600 rounded"
              >
                <option value="">System Default</option>
                {inputDevices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} {device.is_default && '(Default)'}
                  </option>
                ))}
              </select>
            </label>

            {/* Output Device */}
            <label className="block">
              <span className="text-sm text-gray-400">Speaker</span>
              <select
                value={selectedOutputDevice || ''}
                onChange={(e) => setOutputDevice(e.target.value || null)}
                className="w-full mt-1 p-2 bg-wraith-darker border border-gray-600 rounded"
              >
                <option value="">System Default</option>
                {outputDevices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} {device.is_default && '(Default)'}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}
      </div>
    );
  }

  // No active call
  return null;
}

// Incoming Call View
interface IncomingCallViewProps {
  call: CallInfo;
  onAnswer: () => void;
  onReject: () => void;
}

function IncomingCallView({ call, onAnswer, onReject }: IncomingCallViewProps) {
  return (
    <div className="fixed inset-0 bg-wraith-darker/95 flex flex-col items-center justify-center z-50">
      <div className="text-center mb-8 animate-pulse">
        <div className="w-24 h-24 rounded-full bg-wraith-primary flex items-center justify-center text-4xl font-bold mx-auto mb-4">
          {call.peer_id.substring(0, 2).toUpperCase()}
        </div>
        <h2 className="text-2xl font-semibold">Incoming Call</h2>
        <p className="text-gray-400 mt-2">
          {call.peer_id.substring(0, 16)}...
        </p>
      </div>

      <div className="flex items-center gap-8">
        {/* Reject Button */}
        <button
          onClick={onReject}
          className="w-16 h-16 rounded-full bg-red-500 hover:bg-red-600 text-white flex items-center justify-center transition"
          title="Decline"
        >
          <PhoneOffIcon className="w-8 h-8" />
        </button>

        {/* Answer Button */}
        <button
          onClick={onAnswer}
          className="w-16 h-16 rounded-full bg-green-500 hover:bg-green-600 text-white flex items-center justify-center transition"
          title="Answer"
        >
          <PhoneIcon className="w-8 h-8" />
        </button>
      </div>
    </div>
  );
}

// Call button for chat header
interface CallButtonProps {
  peerId: string;
  onCallStart?: () => void;
}

export function CallButton({ peerId, onCallStart }: CallButtonProps) {
  const { startCall, activeCall, loading } = useCallStore();

  const handleClick = async () => {
    if (!activeCall && !loading) {
      await startCall(peerId);
      onCallStart?.();
    }
  };

  return (
    <button
      onClick={handleClick}
      disabled={!!activeCall || loading}
      className="p-2 rounded hover:bg-gray-700 transition disabled:opacity-50 disabled:cursor-not-allowed"
      title="Start Voice Call"
    >
      <PhoneIcon className="w-5 h-5" />
    </button>
  );
}

// Helper function to determine call quality
function getCallQuality(packetsLost: number, packetsReceived: number): string {
  if (packetsReceived === 0) return 'Unknown';
  const lossRate = packetsLost / (packetsReceived + packetsLost);
  if (lossRate < 0.01) return 'Excellent';
  if (lossRate < 0.05) return 'Good';
  if (lossRate < 0.1) return 'Fair';
  return 'Poor';
}

// Icons (simple SVG components)
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

function PhoneIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z" />
    </svg>
  );
}

function PhoneOffIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.516l2.257-1.13a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
    </svg>
  );
}

function SpeakerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
    </svg>
  );
}

function SpeakerHighIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.536 8.464a5 5 0 010 7.072M18.364 5.636a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
    </svg>
  );
}

function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
  );
}
