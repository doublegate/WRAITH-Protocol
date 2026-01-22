// FileUpload Component - File upload with drag and drop

import { useCallback, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import Modal from '../ui/Modal';
import Button from '../ui/Button';
import { useFileStore } from '../../stores/fileStore';
import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';
import { formatBytes } from '../../types';

export default function FileUploadModal() {
  const { activeModal, closeModal, addToast } = useUiStore();
  const { selectedGroupId } = useGroupStore();
  const { uploadFile, uploads } = useFileStore();

  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [uploading, setUploading] = useState(false);

  const isOpen = activeModal === 'fileUpload';

  const onDrop = useCallback((acceptedFiles: File[]) => {
    setSelectedFiles((prev) => [...prev, ...acceptedFiles]);
  }, []);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    multiple: true,
  });

  const handleClose = () => {
    if (!uploading) {
      setSelectedFiles([]);
      closeModal();
    }
  };

  const handleRemoveFile = (index: number) => {
    setSelectedFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const handleUpload = async () => {
    if (!selectedGroupId || selectedFiles.length === 0) return;

    setUploading(true);

    try {
      for (const file of selectedFiles) {
        await uploadFile(selectedGroupId, file);
      }
      addToast('success', `Uploaded ${selectedFiles.length} file(s)`);
      setSelectedFiles([]);
      closeModal();
    } catch (err) {
      addToast('error', (err as Error).message);
    } finally {
      setUploading(false);
    }
  };

  const totalSize = selectedFiles.reduce((acc, f) => acc + f.size, 0);

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      title="Upload Files"
      size="lg"
    >
      <div className="space-y-4">
        {/* Drop zone */}
        <div
          {...getRootProps()}
          className={`border-2 border-dashed rounded-xl p-8 text-center transition-colors cursor-pointer ${
            isDragActive
              ? 'border-violet-500 bg-violet-500/10'
              : 'border-slate-600 hover:border-slate-500 hover:bg-slate-800/50'
          }`}
        >
          <input {...getInputProps()} />
          <svg
            className="w-12 h-12 mx-auto text-slate-500 mb-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          {isDragActive ? (
            <p className="text-violet-400">Drop files here...</p>
          ) : (
            <>
              <p className="text-white mb-1">
                Drag and drop files here, or click to browse
              </p>
              <p className="text-sm text-slate-400">
                All files are encrypted end-to-end
              </p>
            </>
          )}
        </div>

        {/* Selected files */}
        {selectedFiles.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-slate-400">
                {selectedFiles.length} file(s) selected
              </span>
              <span className="text-slate-400">{formatBytes(totalSize)}</span>
            </div>

            <div className="max-h-48 overflow-y-auto space-y-2">
              {selectedFiles.map((file, index) => (
                <div
                  key={`${file.name}-${index}`}
                  className="flex items-center gap-3 p-2 bg-slate-800 rounded-lg"
                >
                  <FileIcon name={file.name} />
                  <div className="flex-1 min-w-0">
                    <p className="text-white truncate">{file.name}</p>
                    <p className="text-xs text-slate-500">{formatBytes(file.size)}</p>
                  </div>
                  <button
                    onClick={() => handleRemoveFile(index)}
                    disabled={uploading}
                    className="p-1 text-slate-400 hover:text-red-400 transition-colors"
                    aria-label={`Remove ${file.name}`}
                  >
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fillRule="evenodd"
                        d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Upload progress */}
        {uploads.length > 0 && (
          <div className="space-y-2">
            <p className="text-sm text-slate-400">Uploading...</p>
            {uploads.map((upload) => (
              <div
                key={upload.id}
                className="p-2 bg-slate-800 rounded-lg"
              >
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm text-white truncate">
                    {upload.fileName}
                  </span>
                  <span className="text-xs text-slate-400">
                    {upload.status === 'completed'
                      ? 'Done'
                      : upload.status === 'failed'
                      ? 'Failed'
                      : `${upload.progress}%`}
                  </span>
                </div>
                <div className="h-1.5 bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className={`h-full transition-all ${
                      upload.status === 'completed'
                        ? 'bg-green-500'
                        : upload.status === 'failed'
                        ? 'bg-red-500'
                        : 'bg-violet-500'
                    }`}
                    style={{ width: `${upload.progress}%` }}
                  />
                </div>
                {upload.error && (
                  <p className="text-xs text-red-400 mt-1">{upload.error}</p>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3 pt-4">
          <Button variant="ghost" onClick={handleClose} disabled={uploading}>
            Cancel
          </Button>
          <Button
            onClick={handleUpload}
            loading={uploading}
            disabled={selectedFiles.length === 0}
          >
            Upload {selectedFiles.length > 0 && `(${selectedFiles.length})`}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function FileIcon({ name }: { name: string }) {
  const ext = name.split('.').pop()?.toLowerCase();

  const getColor = () => {
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'].includes(ext || ''))
      return 'text-pink-400';
    if (['mp4', 'webm', 'mov', 'avi'].includes(ext || '')) return 'text-purple-400';
    if (['mp3', 'wav', 'flac', 'ogg'].includes(ext || '')) return 'text-green-400';
    if (ext === 'pdf') return 'text-red-400';
    if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext || ''))
      return 'text-yellow-400';
    return 'text-blue-400';
  };

  return (
    <svg className={`w-8 h-8 ${getColor()}`} fill="currentColor" viewBox="0 0 20 20">
      <path
        fillRule="evenodd"
        d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"
        clipRule="evenodd"
      />
    </svg>
  );
}
