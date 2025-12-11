// WRAITH Transfer - Settings Panel Component

import { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useSettingsStore, type Theme } from '../stores/settingsStore';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: Props) {
  const {
    theme,
    downloadDir,
    maxConcurrentTransfers,
    port,
    autoAcceptTransfers,
    setTheme,
    setDownloadDir,
    setMaxConcurrentTransfers,
    setPort,
    setAutoAcceptTransfers,
    resetToDefaults,
  } = useSettingsStore();

  const [localDownloadDir, setLocalDownloadDir] = useState(downloadDir);
  const [localMaxTransfers, setLocalMaxTransfers] = useState(maxConcurrentTransfers);
  const [localPort, setLocalPort] = useState(port);

  useEffect(() => {
    if (isOpen) {
      setLocalDownloadDir(downloadDir);
      setLocalMaxTransfers(maxConcurrentTransfers);
      setLocalPort(port);
    }
  }, [isOpen, downloadDir, maxConcurrentTransfers, port]);

  const handleSelectDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select download directory',
      });

      if (selected) {
        setLocalDownloadDir(selected as string);
      }
    } catch (error) {
      console.error('Error selecting directory:', error);
    }
  };

  const handleSave = () => {
    setDownloadDir(localDownloadDir);
    setMaxConcurrentTransfers(localMaxTransfers);
    setPort(localPort);
    onClose();
  };

  const handleReset = () => {
    resetToDefaults();
    setLocalDownloadDir('');
    setLocalMaxTransfers(3);
    setLocalPort(8337);
  };

  const handleClose = () => {
    // Revert to saved values
    setLocalDownloadDir(downloadDir);
    setLocalMaxTransfers(maxConcurrentTransfers);
    setLocalPort(port);
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={handleClose}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <div
        className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-2xl max-h-[90vh] overflow-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="sticky top-0 bg-bg-secondary border-b border-slate-700 px-6 py-4">
          <div className="flex items-center justify-between">
            <h2 id="settings-title" className="text-xl font-semibold text-white">
              Settings
            </h2>
            <button
              onClick={handleClose}
              className="text-slate-400 hover:text-white transition-colors"
              aria-label="Close settings"
            >
              ✕
            </button>
          </div>
        </div>

        <div className="p-6 space-y-6">
          {/* Appearance Section */}
          <section>
            <h3 className="text-lg font-medium text-white mb-4">Appearance</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Theme
                </label>
                <div className="flex gap-2">
                  {(['light', 'dark', 'system'] as Theme[]).map((t) => (
                    <button
                      key={t}
                      onClick={() => setTheme(t)}
                      className={`flex-1 px-4 py-2 rounded-lg border transition-colors ${
                        theme === t
                          ? 'bg-wraith-primary border-wraith-primary text-white'
                          : 'bg-bg-primary border-slate-600 text-slate-400 hover:border-slate-500'
                      }`}
                    >
                      {t.charAt(0).toUpperCase() + t.slice(1)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </section>

          {/* General Section */}
          <section>
            <h3 className="text-lg font-medium text-white mb-4">General</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Download Directory
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={localDownloadDir}
                    onChange={(e) => setLocalDownloadDir(e.target.value)}
                    placeholder="Default: ~/Downloads"
                    className="flex-1 bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary text-sm"
                  />
                  <button
                    onClick={handleSelectDirectory}
                    className="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-white text-sm transition-colors"
                  >
                    Browse
                  </button>
                </div>
                <p className="text-xs text-slate-500 mt-1">
                  Where received files will be saved
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Max Concurrent Transfers
                </label>
                <input
                  type="number"
                  min="1"
                  max="10"
                  value={localMaxTransfers}
                  onChange={(e) => setLocalMaxTransfers(parseInt(e.target.value) || 1)}
                  className="w-full bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-wraith-primary"
                />
                <p className="text-xs text-slate-500 mt-1">
                  Maximum number of simultaneous file transfers (1-10)
                </p>
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm font-medium text-slate-300">
                    Auto-accept transfers
                  </label>
                  <p className="text-xs text-slate-500 mt-1">
                    Automatically accept incoming file transfers
                  </p>
                </div>
                <button
                  onClick={() => setAutoAcceptTransfers(!autoAcceptTransfers)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    autoAcceptTransfers ? 'bg-wraith-primary' : 'bg-slate-600'
                  }`}
                  role="switch"
                  aria-checked={autoAcceptTransfers}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      autoAcceptTransfers ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>
            </div>
          </section>

          {/* Network Section */}
          <section>
            <h3 className="text-lg font-medium text-white mb-4">Network</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Port
                </label>
                <input
                  type="number"
                  min="1024"
                  max="65535"
                  value={localPort}
                  onChange={(e) => setLocalPort(parseInt(e.target.value) || 8337)}
                  className="w-full bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-wraith-primary"
                />
                <p className="text-xs text-slate-500 mt-1">
                  Network port for WRAITH protocol (1024-65535)
                </p>
              </div>
            </div>
          </section>
        </div>

        <div className="sticky bottom-0 bg-bg-secondary border-t border-slate-700 px-6 py-4">
          <div className="flex justify-between">
            <button
              onClick={handleReset}
              className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
            >
              Reset to Defaults
            </button>
            <div className="flex gap-3">
              <button
                onClick={handleClose}
                className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSave}
                className="px-6 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
