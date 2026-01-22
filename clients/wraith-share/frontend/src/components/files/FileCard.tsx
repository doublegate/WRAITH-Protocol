// FileCard Component - Individual file card for grid view

import { useFileStore } from '../../stores/fileStore';
import { useUiStore } from '../../stores/uiStore';
import { useGroupStore } from '../../stores/groupStore';
import type { SharedFile } from '../../types';
import { formatBytes, formatRelativeTime, getFileIcon } from '../../types';

interface FileCardProps {
  file: SharedFile;
  isSelected: boolean;
}

export default function FileCard({ file, isSelected }: FileCardProps) {
  const { selectFile, downloadFile, deleteFile } = useFileStore();
  const { openModal, addToast } = useUiStore();
  const { groupInfos, selectedGroupId } = useGroupStore();

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;
  const canDelete = groupInfo?.my_role === 'admin' || groupInfo?.my_role === 'write';

  const handleClick = () => {
    selectFile(file.id);
  };

  const handleDoubleClick = async () => {
    // Download on double-click
    try {
      await downloadFile(file.id, file.name);
      addToast('success', `Downloaded ${file.name}`);
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  const handleShare = (e: React.MouseEvent) => {
    e.stopPropagation();
    openModal('shareLink', file.id);
  };

  const handleDelete = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm(`Delete "${file.name}"?`)) return;
    try {
      await deleteFile(file.id);
      addToast('success', `Deleted ${file.name}`);
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  const iconType = getFileIcon(file.mime_type);

  return (
    <div
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      className={`group relative p-4 rounded-xl border transition-all cursor-pointer ${
        isSelected
          ? 'bg-violet-600/20 border-violet-500 ring-2 ring-violet-500/50'
          : 'bg-slate-800 border-slate-700 hover:border-slate-600'
      }`}
    >
      {/* Actions (visible on hover) */}
      <div className="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onClick={handleShare}
          className="p-1.5 bg-slate-700/80 hover:bg-slate-600 rounded text-slate-300 hover:text-white transition-colors"
          aria-label="Share file"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
            />
          </svg>
        </button>
        {canDelete && (
          <button
            onClick={handleDelete}
            className="p-1.5 bg-slate-700/80 hover:bg-red-600 rounded text-slate-300 hover:text-white transition-colors"
            aria-label="Delete file"
          >
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path
                fillRule="evenodd"
                d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                clipRule="evenodd"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Icon */}
      <div className="flex items-center justify-center h-20 mb-3">
        <FileTypeIcon type={iconType} />
      </div>

      {/* File info */}
      <div className="space-y-1">
        <h3 className="font-medium text-white truncate" title={file.name}>
          {file.name}
        </h3>
        <div className="flex items-center justify-between text-xs text-slate-400">
          <span>{formatBytes(file.size)}</span>
          <span>{formatRelativeTime(file.modified_at)}</span>
        </div>
      </div>

      {/* Version badge */}
      {file.version > 1 && (
        <div className="absolute bottom-2 right-2">
          <span className="px-1.5 py-0.5 bg-cyan-500/20 text-cyan-400 text-xs rounded">
            v{file.version}
          </span>
        </div>
      )}
    </div>
  );
}

function FileTypeIcon({ type }: { type: string }) {
  const colors: Record<string, string> = {
    image: 'text-pink-400',
    video: 'text-purple-400',
    audio: 'text-green-400',
    pdf: 'text-red-400',
    archive: 'text-yellow-400',
    document: 'text-blue-400',
    spreadsheet: 'text-emerald-400',
    presentation: 'text-orange-400',
    text: 'text-slate-300',
    file: 'text-slate-400',
  };

  const icons: Record<string, JSX.Element> = {
    image: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M4 3a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V5a2 2 0 00-2-2H4zm12 12H4l4-8 3 6 2-4 3 6z" clipRule="evenodd" />
      </svg>
    ),
    video: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path d="M2 6a2 2 0 012-2h6a2 2 0 012 2v8a2 2 0 01-2 2H4a2 2 0 01-2-2V6zM14.553 7.106A1 1 0 0014 8v4a1 1 0 00.553.894l2 1A1 1 0 0018 13V7a1 1 0 00-1.447-.894l-2 1z" />
      </svg>
    ),
    audio: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z" />
      </svg>
    ),
    pdf: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z" clipRule="evenodd" />
      </svg>
    ),
    archive: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path d="M4 3a2 2 0 100 4h12a2 2 0 100-4H4z" />
        <path fillRule="evenodd" d="M3 8h14v7a2 2 0 01-2 2H5a2 2 0 01-2-2V8zm5 3a1 1 0 011-1h2a1 1 0 110 2H9a1 1 0 01-1-1z" clipRule="evenodd" />
      </svg>
    ),
    file: (
      <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z" clipRule="evenodd" />
      </svg>
    ),
  };

  return (
    <div className={colors[type] || colors.file}>
      {icons[type] || icons.file}
    </div>
  );
}
