// SyncStatus Component - Overall sync status display

import { useSyncStore } from '../stores/syncStore';

export default function SyncStatus() {
  const { status, pauseSync, resumeSync } = useSyncStore();

  if (!status) {
    return (
      <div className="flex items-center gap-2 text-gray-400">
        <div className="animate-spin h-4 w-4 border-2 border-wraith-primary border-t-transparent rounded-full" />
        <span>Loading...</span>
      </div>
    );
  }

  const getStatusColor = () => {
    switch (status.status) {
      case 'syncing':
        return 'text-blue-400';
      case 'paused':
        return 'text-yellow-400';
      case 'error':
        return 'text-red-400';
      case 'offline':
        return 'text-gray-400';
      default:
        return 'text-green-400';
    }
  };

  const getStatusIcon = () => {
    switch (status.status) {
      case 'syncing':
        return (
          <div className="animate-spin h-4 w-4 border-2 border-blue-400 border-t-transparent rounded-full" />
        );
      case 'paused':
        return (
          <svg
            className="h-4 w-4"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
        );
      case 'error':
        return (
          <svg
            className="h-4 w-4"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
        );
      case 'offline':
        return (
          <svg
            className="h-4 w-4"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
              clipRule="evenodd"
            />
          </svg>
        );
      default:
        return (
          <svg
            className="h-4 w-4"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
              clipRule="evenodd"
            />
          </svg>
        );
    }
  };

  const getStatusText = () => {
    switch (status.status) {
      case 'syncing':
        return `Syncing ${status.syncing_folders} folder${status.syncing_folders !== 1 ? 's' : ''}`;
      case 'paused':
        return 'Paused';
      case 'error':
        return 'Error';
      case 'offline':
        return 'Offline';
      default:
        return 'Up to date';
    }
  };

  return (
    <div className="flex items-center gap-4">
      {/* Status indicator */}
      <div className={`flex items-center gap-2 ${getStatusColor()}`}>
        {getStatusIcon()}
        <span className="text-sm font-medium">{getStatusText()}</span>
      </div>

      {/* Pending operations */}
      {status.pending_operations > 0 && (
        <span className="text-xs text-gray-400">
          {status.pending_operations} pending
        </span>
      )}

      {/* Conflicts indicator */}
      {status.unresolved_conflicts > 0 && (
        <span className="px-2 py-0.5 text-xs rounded-full bg-red-500 text-white">
          {status.unresolved_conflicts} conflict
          {status.unresolved_conflicts !== 1 ? 's' : ''}
        </span>
      )}

      {/* Pause/Resume button */}
      <button
        onClick={status.is_paused ? resumeSync : pauseSync}
        className="px-3 py-1 text-sm rounded bg-wraith-primary hover:bg-wraith-secondary transition-colors"
      >
        {status.is_paused ? 'Resume' : 'Pause'}
      </button>
    </div>
  );
}
