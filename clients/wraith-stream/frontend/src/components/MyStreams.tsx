import { useEffect, useState } from 'react';
import { useStreamStore } from '../stores/streamStore';
import type { StreamInfo } from '../types';

// SVG Icons
const EditIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
  </svg>
);

const TrashIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="3,6 5,6 21,6" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <line x1="10" y1="11" x2="10" y2="17" />
    <line x1="14" y1="11" x2="14" y2="17" />
  </svg>
);

const PlayIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
    <polygon points="5,3 19,12 5,21" />
  </svg>
);

const formatDuration = (seconds: number | null): string => {
  if (seconds === null) return '--:--';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
};

const formatDate = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
};

export default function MyStreams() {
  const {
    myStreams,
    fetchMyStreams,
    deleteStream,
    selectStream,
    setCurrentView,
    isLoading,
  } = useStreamStore();

  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [editStream, setEditStream] = useState<StreamInfo | null>(null);

  useEffect(() => {
    fetchMyStreams();
  }, [fetchMyStreams]);

  const handleDelete = async (streamId: string) => {
    try {
      await deleteStream(streamId);
      setDeleteConfirm(null);
    } catch (error) {
      console.error('Failed to delete stream:', error);
    }
  };

  const handlePlay = (streamId: string) => {
    selectStream(streamId);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex flex-col items-center gap-4">
          <div className="w-10 h-10 border-4 border-[var(--color-primary-500)] border-t-transparent rounded-full animate-spin" />
          <p className="text-[var(--color-text-secondary)]">Loading your streams...</p>
        </div>
      </div>
    );
  }

  if (myStreams.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
            <line x1="7" y1="2" x2="7" y2="22" />
            <line x1="17" y1="2" x2="17" y2="22" />
          </svg>
        </div>
        <p className="empty-state-title">No streams yet</p>
        <p className="empty-state-description">
          Upload your first video to get started!
        </p>
        <button
          className="btn btn-primary mt-4"
          onClick={() => setCurrentView('upload')}
        >
          Upload Video
        </button>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-semibold text-[var(--color-text-primary)]">
          My Streams
        </h1>
        <button
          className="btn btn-primary"
          onClick={() => setCurrentView('upload')}
        >
          Upload New
        </button>
      </div>

      {/* Stream list */}
      <div className="space-y-4">
        {myStreams.map((stream) => (
          <div key={stream.id} className="card p-4">
            <div className="flex gap-4">
              {/* Thumbnail */}
              <div className="w-48 h-27 flex-shrink-0 rounded-lg overflow-hidden bg-[var(--color-bg-tertiary)]">
                {stream.thumbnail_url ? (
                  <img
                    src={stream.thumbnail_url}
                    alt={stream.title}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="w-full h-full flex items-center justify-center text-[var(--color-text-muted)]">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                      <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
                      <line x1="7" y1="2" x2="7" y2="22" />
                      <line x1="17" y1="2" x2="17" y2="22" />
                    </svg>
                  </div>
                )}
              </div>

              {/* Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <h3 className="font-semibold text-[var(--color-text-primary)] truncate">
                      {stream.title}
                    </h3>
                    {stream.description && (
                      <p className="text-sm text-[var(--color-text-secondary)] line-clamp-2 mt-1">
                        {stream.description}
                      </p>
                    )}
                  </div>

                  {/* Status badge */}
                  <StatusBadge status={stream.status} isLive={stream.is_live} />
                </div>

                {/* Meta info */}
                <div className="flex items-center gap-4 mt-3 text-sm text-[var(--color-text-muted)]">
                  <span>{formatDuration(stream.duration)}</span>
                  <span>-</span>
                  <span>{stream.view_count.toLocaleString()} views</span>
                  <span>-</span>
                  <span>{formatDate(stream.created_at)}</span>
                  {stream.category && (
                    <>
                      <span>-</span>
                      <span className="category-pill">{stream.category}</span>
                    </>
                  )}
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 mt-4">
                  {stream.status === 'ready' && (
                    <button
                      className="btn btn-primary text-sm py-1.5"
                      onClick={() => handlePlay(stream.id)}
                    >
                      <PlayIcon />
                      Watch
                    </button>
                  )}
                  <button
                    className="btn btn-secondary text-sm py-1.5"
                    onClick={() => setEditStream(stream)}
                  >
                    <EditIcon />
                    Edit
                  </button>
                  <button
                    className="btn btn-secondary text-sm py-1.5 hover:bg-[var(--color-error)]/10 hover:text-[var(--color-error)] hover:border-[var(--color-error)]"
                    onClick={() => setDeleteConfirm(stream.id)}
                  >
                    <TrashIcon />
                    Delete
                  </button>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Delete confirmation modal */}
      {deleteConfirm && (
        <div className="modal-overlay" onClick={() => setDeleteConfirm(null)}>
          <div className="modal-content p-6 max-w-sm" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
              Delete Stream?
            </h3>
            <p className="text-[var(--color-text-secondary)] mb-6">
              This action cannot be undone. All segments and data will be permanently deleted.
            </p>
            <div className="flex justify-end gap-3">
              <button
                className="btn btn-secondary"
                onClick={() => setDeleteConfirm(null)}
              >
                Cancel
              </button>
              <button
                className="btn btn-danger"
                onClick={() => handleDelete(deleteConfirm)}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit modal */}
      {editStream && (
        <EditStreamModal
          stream={editStream}
          onClose={() => setEditStream(null)}
        />
      )}
    </div>
  );
}

// Status badge component
function StatusBadge({ status, isLive }: { status: string; isLive: boolean }) {
  if (isLive) {
    return <span className="live-badge">LIVE</span>;
  }

  const statusConfig: Record<string, { bg: string; text: string; label: string }> = {
    processing: { bg: 'var(--color-warning)/20', text: 'var(--color-warning)', label: 'Processing' },
    ready: { bg: 'var(--color-success)/20', text: 'var(--color-success)', label: 'Ready' },
    failed: { bg: 'var(--color-error)/20', text: 'var(--color-error)', label: 'Failed' },
  };

  const config = statusConfig[status] || statusConfig.processing;

  return (
    <span
      className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium"
      style={{
        backgroundColor: config.bg,
        color: config.text,
      }}
    >
      {config.label}
    </span>
  );
}

// Edit stream modal
function EditStreamModal({ stream, onClose }: { stream: StreamInfo; onClose: () => void }) {
  const { updateStream } = useStreamStore();
  const [title, setTitle] = useState(stream.title);
  const [description, setDescription] = useState(stream.description || '');
  const [category, setCategory] = useState(stream.category || '');
  const [tags, setTags] = useState(stream.tags || '');
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await updateStream(stream.id, { title, description, category, tags });
      onClose();
    } catch (error) {
      console.error('Failed to update stream:', error);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content p-6 w-full max-w-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-lg font-semibold text-[var(--color-text-primary)] mb-4">
          Edit Stream
        </h3>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Title
            </label>
            <input
              type="text"
              className="input"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Description
            </label>
            <textarea
              className="input min-h-[80px] resize-y"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Category
            </label>
            <input
              type="text"
              className="input"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Tags
            </label>
            <input
              type="text"
              className="input"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="Separate with commas"
            />
          </div>
        </div>

        <div className="flex justify-end gap-3 mt-6">
          <button className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={isSaving}
          >
            {isSaving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>
    </div>
  );
}
