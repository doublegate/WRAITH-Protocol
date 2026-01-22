// FileBrowser Component - Main file browsing interface

import { useEffect } from 'react';
import { useFileStore } from '../../stores/fileStore';
import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';
import FileCard from './FileCard';
import Button from '../ui/Button';

export default function FileBrowser() {
  const { selectedGroupId, groupInfos } = useGroupStore();
  const {
    fetchFiles,
    getSortedFiles,
    sortBy,
    sortOrder,
    setSortBy,
    setSortOrder,
    loading,
    selectedFileId,
  } = useFileStore();
  const { viewMode, openModal } = useUiStore();

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;
  const canUpload = groupInfo?.my_role === 'admin' || groupInfo?.my_role === 'write';

  useEffect(() => {
    if (selectedGroupId) {
      fetchFiles(selectedGroupId);
    }
  }, [selectedGroupId, fetchFiles]);

  const files = getSortedFiles();

  if (!selectedGroupId) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center p-8">
          <svg
            className="w-16 h-16 mx-auto text-slate-600 mb-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
            />
          </svg>
          <h3 className="text-lg font-medium text-white mb-2">No group selected</h3>
          <p className="text-slate-400">
            Select a group from the sidebar to view its files
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between p-4 border-b border-slate-700">
        <div className="flex items-center gap-2">
          <span className="text-sm text-slate-400">Sort by:</span>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
            className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-white"
            aria-label="Sort by"
          >
            <option value="name">Name</option>
            <option value="size">Size</option>
            <option value="date">Date</option>
            <option value="type">Type</option>
          </select>
          <button
            onClick={() => setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')}
            className="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded transition-colors"
            aria-label={`Sort ${sortOrder === 'asc' ? 'descending' : 'ascending'}`}
          >
            <svg
              className={`w-4 h-4 transition-transform ${sortOrder === 'desc' ? 'rotate-180' : ''}`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"
              />
            </svg>
          </button>
        </div>

        {canUpload && (
          <Button size="sm" onClick={() => openModal('fileUpload')}>
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                />
              </svg>
              Upload
            </span>
          </Button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {loading && files.length === 0 ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin w-8 h-8 border-2 border-violet-500 border-t-transparent rounded-full" />
          </div>
        ) : files.length === 0 ? (
          <EmptyState canUpload={canUpload} onUpload={() => openModal('fileUpload')} />
        ) : viewMode === 'grid' ? (
          <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
            {files.map((file) => (
              <FileCard
                key={file.id}
                file={file}
                isSelected={selectedFileId === file.id}
              />
            ))}
          </div>
        ) : (
          <FileListView files={files} />
        )}
      </div>
    </div>
  );
}

function EmptyState({
  canUpload,
  onUpload,
}: {
  canUpload: boolean;
  onUpload: () => void;
}) {
  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-center">
        <svg
          className="w-16 h-16 mx-auto text-slate-600 mb-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <h3 className="text-lg font-medium text-white mb-2">No files yet</h3>
        <p className="text-slate-400 mb-4">
          {canUpload
            ? 'Upload files to share them with the group'
            : 'No files have been shared in this group yet'}
        </p>
        {canUpload && (
          <Button onClick={onUpload}>
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                />
              </svg>
              Upload Files
            </span>
          </Button>
        )}
      </div>
    </div>
  );
}

function FileListView({ files }: { files: ReturnType<typeof useFileStore.getState>['files'] }) {
  const { selectFile, selectedFileId, downloadFile, deleteFile } = useFileStore();
  const { addToast, openModal } = useUiStore();
  const { groupInfos, selectedGroupId } = useGroupStore();

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;
  const canDelete = groupInfo?.my_role === 'admin' || groupInfo?.my_role === 'write';

  const handleDownload = async (file: typeof files[0]) => {
    try {
      await downloadFile(file.id, file.name);
      addToast('success', `Downloaded ${file.name}`);
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  const handleDelete = async (file: typeof files[0]) => {
    if (!confirm(`Delete "${file.name}"?`)) return;
    try {
      await deleteFile(file.id);
      addToast('success', `Deleted ${file.name}`);
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  return (
    <div className="space-y-1">
      {/* Header */}
      <div className="flex items-center gap-4 p-2 text-xs text-slate-500 font-medium uppercase">
        <div className="flex-1">Name</div>
        <div className="w-24 text-right">Size</div>
        <div className="w-32 text-right">Modified</div>
        <div className="w-24"></div>
      </div>

      {/* Files */}
      {files.map((file) => (
        <div
          key={file.id}
          onClick={() => selectFile(file.id)}
          className={`flex items-center gap-4 p-2 rounded-lg cursor-pointer transition-colors ${
            selectedFileId === file.id
              ? 'bg-violet-600/20 border border-violet-500'
              : 'hover:bg-slate-800 border border-transparent'
          }`}
        >
          <div className="flex-1 flex items-center gap-3 min-w-0">
            <FileIcon mimeType={file.mime_type} />
            <span className="truncate text-white">{file.name}</span>
          </div>
          <div className="w-24 text-right text-sm text-slate-400">
            {formatBytes(file.size)}
          </div>
          <div className="w-32 text-right text-sm text-slate-400">
            {formatDate(file.modified_at)}
          </div>
          <div className="w-24 flex justify-end gap-1">
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleDownload(file);
              }}
              className="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded"
              aria-label="Download"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                openModal('shareLink', file.id);
              }}
              className="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded"
              aria-label="Share"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
              </svg>
            </button>
            {canDelete && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDelete(file);
                }}
                className="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-400/10 rounded"
                aria-label="Delete"
              >
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd" />
                </svg>
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

function FileIcon({ mimeType }: { mimeType: string | null }) {
  const getIconColor = () => {
    if (!mimeType) return 'text-slate-400';
    if (mimeType.startsWith('image/')) return 'text-pink-400';
    if (mimeType.startsWith('video/')) return 'text-purple-400';
    if (mimeType.startsWith('audio/')) return 'text-green-400';
    if (mimeType.includes('pdf')) return 'text-red-400';
    if (mimeType.includes('zip') || mimeType.includes('archive')) return 'text-yellow-400';
    return 'text-blue-400';
  };

  return (
    <svg className={`w-5 h-5 ${getIconColor()}`} fill="currentColor" viewBox="0 0 20 20">
      <path fillRule="evenodd" d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z" clipRule="evenodd" />
    </svg>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString();
}
