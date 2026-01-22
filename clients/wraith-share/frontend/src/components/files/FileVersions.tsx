// FileVersions Component - Display and manage file versions

import { useEffect } from 'react';
import { useFileStore } from '../../stores/fileStore';
import { useUiStore } from '../../stores/uiStore';
import Button from '../ui/Button';
import { formatBytes, formatRelativeTime, truncatePeerId } from '../../types';

export default function FileVersions() {
  const { selectedFileId, files, versions, fetchVersions, restoreVersion } = useFileStore();
  const { addToast } = useUiStore();

  const selectedFile = files.find((f) => f.id === selectedFileId);

  useEffect(() => {
    if (selectedFileId) {
      fetchVersions(selectedFileId);
    }
  }, [selectedFileId, fetchVersions]);

  if (!selectedFile) {
    return (
      <div className="p-4 text-center text-slate-400">
        <p>Select a file to view its versions</p>
      </div>
    );
  }

  const handleRestore = async (version: number) => {
    if (!selectedFileId) return;
    if (!confirm(`Restore "${selectedFile.name}" to version ${version}?`)) return;

    try {
      await restoreVersion(selectedFileId, version);
      addToast('success', `Restored to version ${version}`);
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  return (
    <div className="p-4 space-y-4">
      {/* Current file info */}
      <div className="p-3 bg-slate-800 rounded-lg">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-violet-600/20 rounded-lg flex items-center justify-center">
            <svg className="w-5 h-5 text-violet-400" fill="currentColor" viewBox="0 0 20 20">
              <path
                fillRule="evenodd"
                d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"
                clipRule="evenodd"
              />
            </svg>
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-medium text-white truncate">{selectedFile.name}</h3>
            <p className="text-xs text-slate-400">
              Current version: {selectedFile.version} | {formatBytes(selectedFile.size)}
            </p>
          </div>
        </div>
      </div>

      {/* Version history */}
      <div>
        <h4 className="text-sm font-medium text-slate-300 mb-2">Version History</h4>
        {versions.length === 0 ? (
          <p className="text-sm text-slate-500 text-center py-4">
            No version history available
          </p>
        ) : (
          <div className="space-y-2">
            {versions.map((version) => {
              const isCurrent = version.version === selectedFile.version;

              return (
                <div
                  key={version.version}
                  className={`p-3 rounded-lg border ${
                    isCurrent
                      ? 'bg-violet-600/10 border-violet-500'
                      : 'bg-slate-800 border-slate-700'
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-white">
                          Version {version.version}
                        </span>
                        {isCurrent && (
                          <span className="px-1.5 py-0.5 bg-violet-500 text-white text-xs rounded">
                            Current
                          </span>
                        )}
                      </div>
                      <p className="text-xs text-slate-400 mt-1">
                        {formatBytes(version.size)} | {formatRelativeTime(version.uploaded_at)}
                      </p>
                      <p className="text-xs text-slate-500">
                        By {truncatePeerId(version.uploaded_by)}
                      </p>
                      {version.comment && (
                        <p className="text-sm text-slate-300 mt-2">{version.comment}</p>
                      )}
                    </div>
                    {!isCurrent && (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => handleRestore(version.version)}
                      >
                        Restore
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
