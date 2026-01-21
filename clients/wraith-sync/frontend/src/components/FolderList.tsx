// FolderList Component - List of synced folders with management

import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useSyncStore } from '../stores/syncStore';
import type { FolderInfo } from '../types';

export default function FolderList() {
  const {
    folders,
    selectedFolderId,
    folderFiles,
    addFolder,
    removeFolder,
    selectFolder,
    pauseFolder,
    resumeFolder,
    forceSyncFolder,
    error,
  } = useSyncStore();

  const [showAddDialog, setShowAddDialog] = useState(false);
  const [newRemotePath, setNewRemotePath] = useState('');
  const [pendingLocalPath, setPendingLocalPath] = useState('');

  const handleAddFolder = async () => {
    // Open folder picker
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select folder to sync',
    });

    if (selected) {
      const localPath = selected as string;
      setPendingLocalPath(localPath);
      // Use folder name as default remote path
      const folderName = localPath.split(/[/\\]/).pop() || 'sync';
      setNewRemotePath(`/${folderName}`);
      setShowAddDialog(true);
    }
  };

  const getStatusBadge = (folder: FolderInfo) => {
    const badges: Record<string, { bg: string; text: string }> = {
      syncing: { bg: 'bg-blue-500', text: 'Syncing' },
      paused: { bg: 'bg-yellow-500', text: 'Paused' },
      error: { bg: 'bg-red-500', text: 'Error' },
      offline: { bg: 'bg-gray-500', text: 'Offline' },
      idle: { bg: 'bg-green-500', text: 'Synced' },
    };

    const badge = badges[folder.status] || badges.idle;

    return (
      <span
        className={`px-2 py-0.5 text-xs rounded-full ${badge.bg} text-white`}
      >
        {badge.text}
      </span>
    );
  };

  const formatSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  return (
    <div className="flex h-full">
      {/* Folder list */}
      <div className="w-1/2 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Synced Folders</h2>
          <button
            onClick={handleAddFolder}
            className="px-3 py-1.5 text-sm rounded bg-wraith-primary hover:bg-wraith-secondary transition-colors"
          >
            + Add Folder
          </button>
        </div>

        {error && (
          <div className="mx-4 mt-4 p-3 bg-red-500/20 border border-red-500 rounded text-sm text-red-400">
            {error}
          </div>
        )}

        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {folders.length === 0 ? (
            <div className="text-center text-gray-400 py-8">
              <div className="text-4xl mb-2">...</div>
              <p>No folders synced yet</p>
              <p className="text-sm mt-1">
                Click "Add Folder" to start syncing
              </p>
            </div>
          ) : (
            folders.map((folder) => (
              <div
                key={folder.id}
                onClick={() => selectFolder(folder.id)}
                className={`p-4 rounded-lg border cursor-pointer transition-colors ${
                  selectedFolderId === folder.id
                    ? 'border-wraith-primary bg-wraith-primary/10'
                    : 'border-gray-700 hover:border-gray-600 bg-wraith-dark'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-lg">...</span>
                      <span className="font-medium truncate">
                        {folder.local_path.split(/[/\\]/).pop()}
                      </span>
                      {getStatusBadge(folder)}
                    </div>
                    <p className="text-sm text-gray-400 truncate mt-1">
                      {folder.local_path}
                    </p>
                    <p className="text-xs text-gray-500 mt-1">
                      {folder.synced_files} / {folder.total_files} files synced
                      {folder.pending_operations > 0 &&
                        ` - ${folder.pending_operations} pending`}
                    </p>
                  </div>
                  <div className="flex gap-1 ml-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (folder.paused) {
                          resumeFolder(folder.id);
                        } else {
                          pauseFolder(folder.id);
                        }
                      }}
                      className="p-1.5 rounded hover:bg-gray-700 transition-colors"
                      title={folder.paused ? 'Resume' : 'Pause'}
                    >
                      {folder.paused ? (
                        <svg
                          className="w-4 h-4"
                          fill="currentColor"
                          viewBox="0 0 20 20"
                        >
                          <path
                            fillRule="evenodd"
                            d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
                            clipRule="evenodd"
                          />
                        </svg>
                      ) : (
                        <svg
                          className="w-4 h-4"
                          fill="currentColor"
                          viewBox="0 0 20 20"
                        >
                          <path
                            fillRule="evenodd"
                            d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z"
                            clipRule="evenodd"
                          />
                        </svg>
                      )}
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        forceSyncFolder(folder.id);
                      }}
                      className="p-1.5 rounded hover:bg-gray-700 transition-colors"
                      title="Force Sync"
                    >
                      <svg
                        className="w-4 h-4"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                      >
                        <path
                          fillRule="evenodd"
                          d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z"
                          clipRule="evenodd"
                        />
                      </svg>
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (
                          confirm(
                            'Are you sure you want to remove this folder from sync?'
                          )
                        ) {
                          removeFolder(folder.id);
                        }
                      }}
                      className="p-1.5 rounded hover:bg-red-700 transition-colors text-red-400"
                      title="Remove"
                    >
                      <svg
                        className="w-4 h-4"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                      >
                        <path
                          fillRule="evenodd"
                          d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                          clipRule="evenodd"
                        />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* File list for selected folder */}
      <div className="w-1/2 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h2 className="text-lg font-semibold">
            {selectedFolderId !== null
              ? `Files in ${folders.find((f) => f.id === selectedFolderId)?.local_path.split(/[/\\]/).pop() || 'folder'}`
              : 'Select a folder'}
          </h2>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {selectedFolderId === null ? (
            <div className="text-center text-gray-400 py-8">
              <p>Select a folder to view its files</p>
            </div>
          ) : folderFiles.length === 0 ? (
            <div className="text-center text-gray-400 py-8">
              <p>No files in this folder</p>
            </div>
          ) : (
            <div className="space-y-2">
              {folderFiles.map((file) => (
                <div
                  key={file.relative_path}
                  className="p-3 rounded border border-gray-700 bg-wraith-dark"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-gray-400">...</span>
                      <span className="truncate">{file.relative_path}</span>
                      {file.synced ? (
                        <span className="text-green-400 text-xs">Synced</span>
                      ) : (
                        <span className="text-yellow-400 text-xs">Pending</span>
                      )}
                    </div>
                    <div className="text-xs text-gray-500 whitespace-nowrap ml-2">
                      {formatSize(file.size)}
                      {file.versions.length > 0 && (
                        <span className="ml-2">
                          {file.versions.length} version
                          {file.versions.length !== 1 ? 's' : ''}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Add folder dialog - simplified inline modal */}
      {showAddDialog && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-wraith-dark border border-gray-700 rounded-lg p-6 w-96">
            <h3 className="text-lg font-semibold mb-4">Add Sync Folder</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Remote Path
                </label>
                <input
                  type="text"
                  value={newRemotePath}
                  onChange={(e) => setNewRemotePath(e.target.value)}
                  className="w-full px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                  placeholder="/my-folder"
                />
              </div>
              <div className="flex justify-end gap-2">
                <button
                  onClick={() => setShowAddDialog(false)}
                  className="px-4 py-2 text-sm rounded border border-gray-700 hover:bg-gray-700 transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={async () => {
                    if (pendingLocalPath && newRemotePath) {
                      await addFolder(pendingLocalPath, newRemotePath);
                      setPendingLocalPath('');
                      setNewRemotePath('');
                      setShowAddDialog(false);
                    }
                  }}
                  className="px-4 py-2 text-sm rounded bg-wraith-primary hover:bg-wraith-secondary transition-colors"
                >
                  Add Folder
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
