// WRAITH Recon - Kill Switch Button Component

import { useState } from 'react';
import { useEngagementStore } from '../stores/engagementStore';

export function KillSwitchButton() {
  const { status, killSwitchState, activateKillSwitch, loading } = useEngagementStore();
  const [showConfirm, setShowConfirm] = useState(false);
  const [reason, setReason] = useState('');

  const isKilled = status === 'Terminated' || killSwitchState?.activated;
  const canKill = status === 'Active' || status === 'Paused';

  const handleKillSwitch = async () => {
    if (!reason.trim()) return;

    try {
      await activateKillSwitch(reason);
      setShowConfirm(false);
      setReason('');
    } catch (e) {
      console.error('Kill switch activation failed:', e);
    }
  };

  if (isKilled) {
    return (
      <div className="p-4 bg-red-900/30 border border-red-500 rounded-lg">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-red-500 flex items-center justify-center animate-pulse">
            <svg className="w-6 h-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div>
            <h3 className="text-lg font-bold text-red-400">ENGAGEMENT TERMINATED</h3>
            {killSwitchState?.reason && (
              <p className="text-sm text-red-300">Reason: {killSwitchState.reason}</p>
            )}
            {killSwitchState?.activated_at && (
              <p className="text-xs text-red-400 mt-1">
                At: {new Date(killSwitchState.activated_at).toLocaleString()}
              </p>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <>
      {/* Kill Switch Button */}
      <button
        onClick={() => setShowConfirm(true)}
        disabled={!canKill || loading}
        className={`
          w-full py-3 px-4 rounded-lg font-bold text-lg
          transition-all duration-200
          ${canKill
            ? 'bg-red-600 hover:bg-red-500 text-white shadow-lg shadow-red-500/30'
            : 'bg-gray-700 text-gray-400 cursor-not-allowed'
          }
        `}
      >
        <div className="flex items-center justify-center gap-2">
          <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
          </svg>
          KILL SWITCH
        </div>
      </button>

      {/* Confirmation Modal */}
      {showConfirm && (
        <div className="modal-overlay" onClick={() => setShowConfirm(false)}>
          <div className="modal-content w-[400px] p-6" onClick={(e) => e.stopPropagation()}>
            <h2 className="text-xl font-bold text-red-400 mb-4 flex items-center gap-2">
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              Confirm Kill Switch
            </h2>

            <p className="text-text-secondary mb-4">
              This will immediately terminate all active operations, close all channels,
              and log a critical audit entry. This action cannot be undone.
            </p>

            <div className="mb-4">
              <label className="block text-sm font-medium text-text-secondary mb-2">
                Reason for termination (required)
              </label>
              <textarea
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                placeholder="Enter reason for emergency termination..."
                className="input h-24 resize-none"
                autoFocus
              />
            </div>

            <div className="flex gap-3">
              <button
                onClick={() => setShowConfirm(false)}
                className="btn btn-secondary flex-1"
              >
                Cancel
              </button>
              <button
                onClick={handleKillSwitch}
                disabled={!reason.trim() || loading}
                className="btn btn-danger flex-1"
              >
                {loading ? 'Terminating...' : 'TERMINATE'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
