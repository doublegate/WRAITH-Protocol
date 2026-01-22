import { useRef, useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useStreamStore } from '../stores/streamStore';

// SVG Icons
const UploadIcon = () => (
  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="17,8 12,3 7,8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </svg>
);

const FileVideoIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
    <polyline points="14,2 14,8 20,8" />
    <polygon points="10,11 10,17 15,14" />
  </svg>
);

const categories = [
  'Gaming',
  'Music',
  'Education',
  'Technology',
  'Entertainment',
  'Sports',
  'News',
  'Creative',
  'Other',
];

export default function UploadPanel() {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const {
    upload,
    setUploadFile,
    setUploadTitle,
    setUploadDescription,
    setUploadCategory,
    setUploadTags,
    startUpload,
    cancelUpload,
    resetUpload,
    pollTranscodeProgress,
    setCurrentView,
  } = useStreamStore();

  const [dragOver, setDragOver] = useState(false);

  // Poll for transcode progress when transcoding
  useEffect(() => {
    let interval: number | undefined;
    if (upload.status === 'transcoding' && upload.streamId) {
      interval = window.setInterval(() => {
        pollTranscodeProgress(upload.streamId!);
      }, 1000);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [upload.status, upload.streamId, pollTranscodeProgress]);

  const handleFileSelect = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Video',
          extensions: ['mp4', 'mkv', 'avi', 'mov', 'webm', 'flv', 'm4v'],
        }],
      });

      if (selected && typeof selected === 'string') {
        // Create a File-like object with the path
        const fileName = selected.split('/').pop() || selected.split('\\').pop() || 'video';
        const file = new File([], fileName) as File & { path: string };
        (file as File & { path: string }).path = selected;
        setUploadFile(file);
      }
    } catch (error) {
      console.error('Failed to select file:', error);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      const file = files[0];
      if (file.type.startsWith('video/') || /\.(mp4|mkv|avi|mov|webm|flv|m4v)$/i.test(file.name)) {
        setUploadFile(file);
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!upload.file || !upload.title) return;
    await startUpload();
  };

  const handleComplete = () => {
    resetUpload();
    setCurrentView('my-streams');
  };

  // Upload complete state
  if (upload.status === 'complete') {
    return (
      <div className="max-w-2xl mx-auto">
        <div className="card p-8 text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-[var(--color-success)]/20 flex items-center justify-center">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="20,6 9,17 4,12" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-[var(--color-text-primary)] mb-2">
            Upload Complete!
          </h2>
          <p className="text-[var(--color-text-secondary)] mb-6">
            Your video has been uploaded and is ready to watch.
          </p>
          <button onClick={handleComplete} className="btn btn-primary">
            View My Streams
          </button>
        </div>
      </div>
    );
  }

  // Upload error state
  if (upload.status === 'error') {
    return (
      <div className="max-w-2xl mx-auto">
        <div className="card p-8 text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-[var(--color-error)]/20 flex items-center justify-center">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-[var(--color-text-primary)] mb-2">
            Upload Failed
          </h2>
          <p className="text-[var(--color-error)] mb-6">
            {upload.error || 'An unknown error occurred'}
          </p>
          <button onClick={resetUpload} className="btn btn-secondary">
            Try Again
          </button>
        </div>
      </div>
    );
  }

  // Uploading/Transcoding state
  if (upload.status === 'uploading' || upload.status === 'transcoding') {
    return (
      <div className="max-w-2xl mx-auto">
        <div className="card p-8">
          <h2 className="text-xl font-semibold text-[var(--color-text-primary)] mb-6 text-center">
            {upload.status === 'uploading' ? 'Uploading...' : 'Transcoding...'}
          </h2>

          <div className="transcode-progress mb-6">
            <div className="transcode-progress-header">
              <span className="transcode-progress-label">
                {upload.status === 'transcoding' ? 'Processing video' : 'Uploading file'}
              </span>
              <span className="transcode-progress-value">
                {Math.round(upload.progress)}%
              </span>
            </div>
            <div className="progress-bar mt-2">
              <div
                className="progress-bar-fill"
                style={{ width: `${upload.progress}%` }}
              />
            </div>
          </div>

          <p className="text-sm text-[var(--color-text-muted)] text-center mb-6">
            {upload.status === 'transcoding'
              ? 'Your video is being transcoded to multiple quality levels. This may take a few minutes.'
              : 'Uploading your video file...'}
          </p>

          <div className="text-center">
            <button onClick={cancelUpload} className="btn btn-secondary">
              Cancel
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto">
      <h1 className="text-2xl font-semibold text-[var(--color-text-primary)] mb-6">
        Upload Video
      </h1>

      <form onSubmit={handleSubmit}>
        {/* File upload zone */}
        {!upload.file ? (
          <div
            className={`upload-zone mb-6 ${dragOver ? 'drag-over' : ''}`}
            onClick={handleFileSelect}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
          >
            <div className="text-[var(--color-text-muted)]">
              <UploadIcon />
            </div>
            <div className="text-center">
              <p className="text-[var(--color-text-primary)] font-medium mb-1">
                Click to select or drag and drop
              </p>
              <p className="text-sm text-[var(--color-text-muted)]">
                MP4, MKV, AVI, MOV, WebM (max 10GB)
              </p>
            </div>
            <input
              ref={fileInputRef}
              type="file"
              accept="video/*"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) setUploadFile(file);
              }}
            />
          </div>
        ) : (
          <div className="card p-4 mb-6">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-lg bg-[var(--color-bg-tertiary)] flex items-center justify-center text-[var(--color-primary-500)]">
                <FileVideoIcon />
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-medium text-[var(--color-text-primary)] truncate">
                  {upload.file.name}
                </p>
                <p className="text-sm text-[var(--color-text-muted)]">
                  {(upload.file as File & { path?: string }).path || 'Ready to upload'}
                </p>
              </div>
              <button
                type="button"
                onClick={() => setUploadFile(null)}
                className="text-[var(--color-text-muted)] hover:text-[var(--color-error)]"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          </div>
        )}

        {/* Metadata form */}
        <div className="card p-6 space-y-4">
          {/* Title */}
          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Title <span className="text-[var(--color-error)]">*</span>
            </label>
            <input
              type="text"
              className="input"
              placeholder="Enter video title"
              value={upload.title}
              onChange={(e) => setUploadTitle(e.target.value)}
              required
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Description
            </label>
            <textarea
              className="input min-h-[100px] resize-y"
              placeholder="Describe your video..."
              value={upload.description}
              onChange={(e) => setUploadDescription(e.target.value)}
            />
          </div>

          {/* Category */}
          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Category
            </label>
            <select
              className="input"
              value={upload.category}
              onChange={(e) => setUploadCategory(e.target.value)}
            >
              <option value="">Select a category</option>
              {categories.map((cat) => (
                <option key={cat} value={cat}>{cat}</option>
              ))}
            </select>
          </div>

          {/* Tags */}
          <div>
            <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
              Tags
            </label>
            <input
              type="text"
              className="input"
              placeholder="Enter tags separated by commas"
              value={upload.tags}
              onChange={(e) => setUploadTags(e.target.value)}
            />
            <p className="mt-1 text-xs text-[var(--color-text-muted)]">
              Separate multiple tags with commas (e.g., tutorial, coding, rust)
            </p>
          </div>
        </div>

        {/* Submit button */}
        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => {
              resetUpload();
              setCurrentView('browse');
            }}
          >
            Cancel
          </button>
          <button
            type="submit"
            className="btn btn-primary"
            disabled={!upload.file || !upload.title}
          >
            Upload Video
          </button>
        </div>
      </form>
    </div>
  );
}
