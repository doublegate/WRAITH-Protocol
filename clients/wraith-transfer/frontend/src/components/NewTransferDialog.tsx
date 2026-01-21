// WRAITH Transfer - New Transfer Dialog Component

import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useTransferStore } from '../stores/transferStore';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export function NewTransferDialog({ isOpen, onClose }: Props) {
  const [peerId, setPeerId] = useState('');
  const [filePath, setFilePath] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);
  const { sendFile, loading, error, clearError } = useTransferStore();

  const validatePeerId = (id: string): boolean => {
    // Remove any whitespace
    const trimmedId = id.trim();

    // Check length (64 hex chars = 32 bytes)
    if (trimmedId.length !== 64) {
      setValidationError('Peer ID must be exactly 64 hexadecimal characters');
      return false;
    }

    // Check if valid hex
    if (!/^[0-9a-fA-F]{64}$/.test(trimmedId)) {
      setValidationError('Peer ID must contain only hexadecimal characters (0-9, a-f, A-F)');
      return false;
    }

    setValidationError(null);
    return true;
  };

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        title: 'Select file to send',
      });

      if (selected) {
        console.log('File selected:', selected);
        setFilePath(selected as string);
      } else {
        console.log('No file selected (user cancelled)');
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }
  };

  const handleSend = async () => {
    if (!peerId || !filePath) return;

    // Validate peer ID before sending
    if (!validatePeerId(peerId)) {
      return;
    }

    const transferId = await sendFile(peerId.trim(), filePath);
    if (transferId) {
      // Reset form and close
      setPeerId('');
      setFilePath('');
      setValidationError(null);
      onClose();
    }
  };

  const handlePeerIdChange = (value: string) => {
    setPeerId(value);
    if (validationError) {
      setValidationError(null);
    }
  };

  const handleClose = () => {
    setPeerId('');
    setFilePath('');
    setValidationError(null);
    clearError();
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleClose();
    } else if (e.key === 'Enter' && peerId && filePath && !loading) {
      handleSend();
    }
  };

  if (!isOpen) return null;

  const displayError = validationError || error;

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={handleClose}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-transfer-title"
    >
      <div
        className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-md p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="new-transfer-title" className="text-xl font-semibold text-white mb-4">
          New Transfer
        </h2>

        {displayError && (
          <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
            {displayError}
          </div>
        )}

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1">
              Peer ID
            </label>
            <input
              type="text"
              value={peerId}
              onChange={(e) => handlePeerIdChange(e.target.value)}
              onBlur={() => peerId && validatePeerId(peerId)}
              placeholder="Enter 64-character hex peer ID"
              className={`w-full bg-bg-primary border rounded-lg px-3 py-2 text-white placeholder-slate-500 focus:outline-none font-mono text-sm ${
                validationError ? 'border-red-500 focus:border-red-500' : 'border-slate-600 focus:border-wraith-primary'
              }`}
              aria-invalid={!!validationError}
              aria-describedby={validationError ? 'peer-id-error' : undefined}
            />
            {peerId.length > 0 && (
              <div className="mt-1 text-xs text-slate-500">
                {peerId.length}/64 characters
              </div>
            )}
          </div>

          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1">
              File
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={filePath}
                readOnly
                placeholder="Select a file..."
                className="flex-1 bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white placeholder-slate-500 text-sm"
              />
              <button
                onClick={handleSelectFile}
                className="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-white text-sm transition-colors"
              >
                Browse
              </button>
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-3 mt-6">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSend}
            disabled={!peerId || !filePath || loading || !!validationError}
            className={`px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors ${
              (!peerId || !filePath || loading || validationError) ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            {loading ? 'Sending...' : 'Send File'}
          </button>
        </div>
      </div>
    </div>
  );
}
