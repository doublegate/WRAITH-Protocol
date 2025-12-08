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
  const { sendFile, loading, error, clearError } = useTransferStore();

  const handleSelectFile = async () => {
    const selected = await open({
      multiple: false,
      title: 'Select file to send',
    });

    if (selected) {
      setFilePath(selected as string);
    }
  };

  const handleSend = async () => {
    if (!peerId || !filePath) return;

    const transferId = await sendFile(peerId, filePath);
    if (transferId) {
      // Reset form and close
      setPeerId('');
      setFilePath('');
      onClose();
    }
  };

  const handleClose = () => {
    setPeerId('');
    setFilePath('');
    clearError();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-md p-6">
        <h2 className="text-xl font-semibold text-white mb-4">New Transfer</h2>

        {error && (
          <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
            {error}
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
              onChange={(e) => setPeerId(e.target.value)}
              placeholder="Enter 64-character hex peer ID"
              className="w-full bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary font-mono text-sm"
            />
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
            disabled={!peerId || !filePath || loading}
            className={`px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors ${
              (!peerId || !filePath || loading) ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            {loading ? 'Sending...' : 'Send File'}
          </button>
        </div>
      </div>
    </div>
  );
}
