// ConflictResolver Component - UI for resolving file conflicts

import { useSyncStore } from '../stores/syncStore';
import type { ConflictInfo, ConflictResolution } from '../types';

export default function ConflictResolver() {
  const { conflicts, resolveConflict } = useSyncStore();

  const handleResolve = async (
    conflictId: number,
    resolution: ConflictResolution
  ) => {
    await resolveConflict(conflictId, resolution);
  };

  const handleResolveAll = async (resolution: ConflictResolution) => {
    for (const conflict of conflicts) {
      await resolveConflict(conflict.id, resolution);
    }
  };

  if (conflicts.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-gray-400">
        <svg
          className="w-16 h-16 mb-4"
          fill="currentColor"
          viewBox="0 0 20 20"
        >
          <path
            fillRule="evenodd"
            d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
            clipRule="evenodd"
          />
        </svg>
        <h2 className="text-xl font-semibold mb-2">No Conflicts</h2>
        <p className="text-sm">All your files are in sync</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header with bulk actions */}
      <div className="p-4 border-b border-gray-700 flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            {conflicts.length} Conflict{conflicts.length !== 1 ? 's' : ''} to
            Resolve
          </h2>
          <p className="text-sm text-gray-400 mt-1">
            Files modified on multiple devices need your attention
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => handleResolveAll('local')}
            className="px-3 py-1.5 text-sm rounded border border-gray-700 hover:bg-gray-700 transition-colors"
          >
            Keep All Local
          </button>
          <button
            onClick={() => handleResolveAll('remote')}
            className="px-3 py-1.5 text-sm rounded border border-gray-700 hover:bg-gray-700 transition-colors"
          >
            Keep All Remote
          </button>
          <button
            onClick={() => handleResolveAll('keep_both')}
            className="px-3 py-1.5 text-sm rounded bg-wraith-primary hover:bg-wraith-secondary transition-colors"
          >
            Keep All Both
          </button>
        </div>
      </div>

      {/* Conflict list */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {conflicts.map((conflict) => (
          <ConflictCard
            key={conflict.id}
            conflict={conflict}
            onResolve={handleResolve}
          />
        ))}
      </div>
    </div>
  );
}

interface ConflictCardProps {
  conflict: ConflictInfo;
  onResolve: (conflictId: number, resolution: ConflictResolution) => void;
}

function ConflictCard({ conflict, onResolve }: ConflictCardProps) {
  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getFileName = (path: string): string => {
    return path.split(/[/\\]/).pop() || path;
  };

  return (
    <div className="border border-yellow-500/50 bg-yellow-500/5 rounded-lg p-4">
      {/* File info */}
      <div className="flex items-start gap-3 mb-4">
        <div className="p-2 bg-yellow-500/20 rounded">
          <svg
            className="w-6 h-6 text-yellow-400"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-medium truncate">
            {getFileName(conflict.file_path)}
          </h3>
          <p className="text-sm text-gray-400 truncate">{conflict.file_path}</p>
          <p className="text-xs text-gray-500 mt-1">
            In folder: {conflict.folder_path}
          </p>
        </div>
      </div>

      {/* Version comparison */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        {/* Local version */}
        <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded">
          <div className="flex items-center gap-2 mb-2">
            <svg
              className="w-4 h-4 text-blue-400"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M3 5a2 2 0 012-2h10a2 2 0 012 2v8a2 2 0 01-2 2h-2.22l.123.489.804.321A1 1 0 0113.323 17H6.677a1 1 0 01-.386-1.923l.804-.32.123-.49H5a2 2 0 01-2-2V5zm14.5 0a.5.5 0 00-.5-.5H3a.5.5 0 00-.5.5v8a.5.5 0 00.5.5h14a.5.5 0 00.5-.5V5z"
                clipRule="evenodd"
              />
            </svg>
            <span className="text-sm font-medium text-blue-400">
              Local Version
            </span>
          </div>
          <p className="text-xs text-gray-400">
            Modified: {formatDate(conflict.local_modified_at)}
          </p>
          <p className="text-xs text-gray-500">This device</p>
        </div>

        {/* Remote version */}
        <div className="p-3 bg-purple-500/10 border border-purple-500/30 rounded">
          <div className="flex items-center gap-2 mb-2">
            <svg
              className="w-4 h-4 text-purple-400"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path d="M5.5 16a3.5 3.5 0 01-.369-6.98 4 4 0 117.753-1.977A4.5 4.5 0 1113.5 16h-8z" />
            </svg>
            <span className="text-sm font-medium text-purple-400">
              Remote Version
            </span>
          </div>
          <p className="text-xs text-gray-400">
            Modified: {formatDate(conflict.remote_modified_at)}
          </p>
          <p className="text-xs text-gray-500">From: {conflict.remote_device}</p>
        </div>
      </div>

      {/* Resolution buttons */}
      <div className="flex gap-2">
        <button
          onClick={() => onResolve(conflict.id, 'local')}
          className="flex-1 px-3 py-2 text-sm rounded border border-blue-500 text-blue-400 hover:bg-blue-500/20 transition-colors"
        >
          Keep Local
        </button>
        <button
          onClick={() => onResolve(conflict.id, 'remote')}
          className="flex-1 px-3 py-2 text-sm rounded border border-purple-500 text-purple-400 hover:bg-purple-500/20 transition-colors"
        >
          Keep Remote
        </button>
        <button
          onClick={() => onResolve(conflict.id, 'keep_both')}
          className="flex-1 px-3 py-2 text-sm rounded bg-wraith-primary hover:bg-wraith-secondary transition-colors"
        >
          Keep Both
        </button>
      </div>
    </div>
  );
}
