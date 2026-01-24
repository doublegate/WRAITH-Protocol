// WRAITH Recon - Engagement Controls Component

import { useState } from 'react';
import { useEngagementStore } from '../stores/engagementStore';
import { KillSwitchButton } from './KillSwitchButton';

export function EngagementControls() {
  const {
    status, timingInfo, operatorId,
    startEngagement, stopEngagement, pauseEngagement, resumeEngagement,
    setOperatorId, fetchTimingInfo,
    loading, error,
  } = useEngagementStore();

  const [stopReason, setStopReason] = useState('');
  const [showStopDialog, setShowStopDialog] = useState(false);
  const [newOperatorId, setNewOperatorId] = useState(operatorId);

  const canStart = status === 'Ready';
  const canPause = status === 'Active';
  const canResume = status === 'Paused';
  const canStop = status === 'Active' || status === 'Paused';
  const isTerminated = status === 'Terminated';

  const handleStart = async () => {
    try {
      await startEngagement();
      await fetchTimingInfo();
    } catch (e) {
      console.error('Failed to start engagement:', e);
    }
  };

  const handleStop = async () => {
    if (!stopReason.trim()) return;
    try {
      await stopEngagement(stopReason);
      setStopReason('');
      setShowStopDialog(false);
    } catch (e) {
      console.error('Failed to stop engagement:', e);
    }
  };

  const handleSetOperator = async () => {
    if (newOperatorId.trim()) {
      await setOperatorId(newOperatorId.trim());
    }
  };

  const formatTimeRemaining = (seconds: number | null) => {
    if (seconds === null) return '--:--:--';
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hrs.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="card">
      <div className="p-4 border-b border-border-primary">
        <h2 className="text-lg font-semibold text-text-primary flex items-center gap-2">
          <svg className="w-5 h-5 text-primary-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
          Engagement Controls
        </h2>
      </div>

      <div className="p-4 space-y-4">
        {/* Error Display */}
        {error && (
          <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
            {error}
          </div>
        )}

        {/* Operator ID */}
        <div>
          <label className="block text-sm font-medium text-text-secondary mb-1">Operator ID</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={newOperatorId}
              onChange={(e) => setNewOperatorId(e.target.value)}
              placeholder="Enter operator identifier"
              className="input flex-1"
              disabled={status === 'Active' || isTerminated}
            />
            {newOperatorId !== operatorId && (
              <button
                onClick={handleSetOperator}
                disabled={loading || !newOperatorId.trim()}
                className="btn btn-secondary text-sm"
              >
                Set
              </button>
            )}
          </div>
        </div>

        {/* Timing Info */}
        {timingInfo && (
          <div className="p-3 rounded-lg bg-bg-tertiary">
            <div className="flex justify-between items-center mb-2">
              <span className="text-sm font-medium text-text-secondary">Time Window</span>
              <span className={`text-xs px-2 py-0.5 rounded-full ${
                timingInfo.status === 'Active' ? 'bg-green-500/20 text-green-400' :
                timingInfo.status === 'Expired' ? 'bg-red-500/20 text-red-400' :
                timingInfo.status === 'NotStarted' ? 'bg-yellow-500/20 text-yellow-400' :
                'bg-gray-500/20 text-gray-400'
              }`}>
                {timingInfo.status}
              </span>
            </div>
            {timingInfo.time_remaining_secs !== null && (
              <div className="text-2xl font-mono text-center text-primary-400">
                {formatTimeRemaining(timingInfo.time_remaining_secs)}
              </div>
            )}
            {timingInfo.start_time && timingInfo.end_time && (
              <div className="mt-2 text-xs text-text-muted text-center">
                {new Date(timingInfo.start_time).toLocaleString()} - {new Date(timingInfo.end_time).toLocaleString()}
              </div>
            )}
          </div>
        )}

        {/* Control Buttons */}
        {!isTerminated && (
          <div className="grid grid-cols-2 gap-2">
            {/* Start Button */}
            {canStart && (
              <button
                onClick={handleStart}
                disabled={loading || !operatorId}
                className="btn btn-primary col-span-2"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Start Engagement
              </button>
            )}

            {/* Pause/Resume */}
            {canPause && (
              <button
                onClick={pauseEngagement}
                disabled={loading}
                className="btn btn-secondary"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Pause
              </button>
            )}

            {canResume && (
              <button
                onClick={resumeEngagement}
                disabled={loading}
                className="btn btn-primary"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Resume
              </button>
            )}

            {/* Stop Button */}
            {canStop && (
              <button
                onClick={() => setShowStopDialog(true)}
                disabled={loading}
                className="btn btn-secondary"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
                </svg>
                Stop
              </button>
            )}
          </div>
        )}

        {/* Kill Switch */}
        <div className="pt-4 border-t border-border-primary">
          <KillSwitchButton />
        </div>
      </div>

      {/* Stop Dialog */}
      {showStopDialog && (
        <div className="modal-overlay" onClick={() => setShowStopDialog(false)}>
          <div className="modal-content w-[400px] p-6" onClick={(e) => e.stopPropagation()}>
            <h2 className="text-xl font-bold text-text-primary mb-4">Stop Engagement</h2>

            <p className="text-text-secondary mb-4">
              Please provide a reason for stopping the engagement. This will be recorded in the audit log.
            </p>

            <div className="mb-4">
              <label className="block text-sm font-medium text-text-secondary mb-2">
                Reason
              </label>
              <textarea
                value={stopReason}
                onChange={(e) => setStopReason(e.target.value)}
                placeholder="Enter reason for stopping..."
                className="input h-24 resize-none"
                autoFocus
              />
            </div>

            <div className="flex gap-3">
              <button
                onClick={() => setShowStopDialog(false)}
                className="btn btn-secondary flex-1"
              >
                Cancel
              </button>
              <button
                onClick={handleStop}
                disabled={!stopReason.trim() || loading}
                className="btn btn-primary flex-1"
              >
                {loading ? 'Stopping...' : 'Stop Engagement'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
