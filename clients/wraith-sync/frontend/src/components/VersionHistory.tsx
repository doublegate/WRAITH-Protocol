// VersionHistory Component - Browse and restore previous file versions

import { useState } from 'react';
import { useSyncStore } from '../stores/syncStore';
import { useVersionStore } from '../stores/versionStore';
import type { VersionInfo } from '../types';

export default function VersionHistory() {
  const { folders, folderFiles, selectFolder } = useSyncStore();
  const { versions, selectedFile, selectFile, clearSelection, restoreVersion, loading } =
    useVersionStore();

  const [browseFolderId, setBrowseFolderId] = useState<number | null>(null);

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  const handleFolderSelect = (folderId: number) => {
    setBrowseFolderId(folderId);
    selectFolder(folderId);
    clearSelection();
  };

  const handleFileSelect = (folderId: number, relativePath: string) => {
    selectFile(folderId, relativePath);
  };

  const handleRestore = async (versionId: number) => {
    if (!selectedFile) return;

    if (
      confirm(
        `Are you sure you want to restore this version? The current file will be replaced.`
      )
    ) {
      await restoreVersion(
        selectedFile.folderId,
        selectedFile.relativePath,
        versionId
      );
    }
  };

  const getFileVersions = (): VersionInfo[] => {
    if (!selectedFile) return [];
    const key = `${selectedFile.folderId}:${selectedFile.relativePath}`;
    return versions.get(key) || [];
  };

  const fileVersions = getFileVersions();

  return (
    <div className="flex h-full">
      {/* Folder selector */}
      <div className="w-64 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h3 className="text-sm font-semibold text-gray-400 uppercase">
            Folders
          </h3>
        </div>
        <div className="flex-1 overflow-y-auto">
          {folders.map((folder) => (
            <button
              key={folder.id}
              onClick={() => handleFolderSelect(folder.id)}
              className={`w-full px-4 py-3 text-left flex items-center gap-2 hover:bg-wraith-dark transition-colors ${
                browseFolderId === folder.id ? 'bg-wraith-dark' : ''
              }`}
            >
              <span className="text-gray-400">...</span>
              <span className="truncate">
                {folder.local_path.split(/[/\\]/).pop()}
              </span>
            </button>
          ))}
          {folders.length === 0 && (
            <div className="p-4 text-center text-gray-400 text-sm">
              No folders synced
            </div>
          )}
        </div>
      </div>

      {/* File selector */}
      <div className="w-80 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h3 className="text-sm font-semibold text-gray-400 uppercase">
            Files
          </h3>
        </div>
        <div className="flex-1 overflow-y-auto">
          {browseFolderId === null ? (
            <div className="p-4 text-center text-gray-400 text-sm">
              Select a folder to view files
            </div>
          ) : folderFiles.length === 0 ? (
            <div className="p-4 text-center text-gray-400 text-sm">
              No files in this folder
            </div>
          ) : (
            folderFiles.map((file) => (
              <button
                key={file.relative_path}
                onClick={() =>
                  handleFileSelect(browseFolderId, file.relative_path)
                }
                className={`w-full px-4 py-3 text-left hover:bg-wraith-dark transition-colors ${
                  selectedFile?.relativePath === file.relative_path
                    ? 'bg-wraith-dark'
                    : ''
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-gray-400">...</span>
                  <span className="truncate flex-1">
                    {file.relative_path.split(/[/\\]/).pop()}
                  </span>
                  {file.versions.length > 0 && (
                    <span className="text-xs text-gray-500">
                      {file.versions.length}v
                    </span>
                  )}
                </div>
                <p className="text-xs text-gray-500 mt-1 truncate pl-6">
                  {file.relative_path}
                </p>
              </button>
            ))
          )}
        </div>
      </div>

      {/* Version history */}
      <div className="flex-1 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h3 className="text-sm font-semibold text-gray-400 uppercase">
            Version History
          </h3>
          {selectedFile && (
            <p className="text-sm text-white mt-1">
              {selectedFile.relativePath}
            </p>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {!selectedFile ? (
            <div className="flex flex-col items-center justify-center h-full text-gray-400">
              <svg
                className="w-12 h-12 mb-3"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z"
                  clipRule="evenodd"
                />
              </svg>
              <p className="text-sm">Select a file to view its version history</p>
            </div>
          ) : loading ? (
            <div className="flex items-center justify-center h-full">
              <div className="animate-spin h-8 w-8 border-2 border-wraith-primary border-t-transparent rounded-full" />
            </div>
          ) : fileVersions.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-gray-400">
              <p className="text-sm">No version history available</p>
              <p className="text-xs mt-1">
                Versions are created when files are modified
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {fileVersions.map((version, index) => (
                <div
                  key={version.id}
                  className={`p-4 rounded-lg border ${
                    index === 0
                      ? 'border-green-500/50 bg-green-500/5'
                      : 'border-gray-700 bg-wraith-dark'
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-medium">
                          Version {version.version_number}
                        </span>
                        {index === 0 && (
                          <span className="px-2 py-0.5 text-xs rounded-full bg-green-500 text-white">
                            Current
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-gray-400 mt-1">
                        {formatDate(version.modified_at)}
                      </p>
                      <p className="text-xs text-gray-500 mt-1">
                        {formatSize(version.size)}
                      </p>
                    </div>
                    {index !== 0 && (
                      <button
                        onClick={() => handleRestore(version.id)}
                        className="px-3 py-1.5 text-sm rounded border border-wraith-primary text-wraith-primary hover:bg-wraith-primary hover:text-white transition-colors"
                      >
                        Restore
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
