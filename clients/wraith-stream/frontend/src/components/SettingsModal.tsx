import { useState } from 'react';
import { useAppStore } from '../stores/appStore';

const CloseIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

export default function SettingsModal() {
  const { peerId, displayName, setDisplayName, setSettingsOpen } = useAppStore();
  const [nameInput, setNameInput] = useState(displayName);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!nameInput.trim()) return;

    setIsSaving(true);
    setSaveError(null);

    try {
      await setDisplayName(nameInput.trim());
      setSettingsOpen(false);
    } catch (error) {
      setSaveError(String(error));
    } finally {
      setIsSaving(false);
    }
  };

  const handleClose = () => {
    setSettingsOpen(false);
  };

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div
        className="modal-content w-full max-w-md"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[var(--color-border-primary)]">
          <h2 className="text-lg font-semibold text-[var(--color-text-primary)]">
            Settings
          </h2>
          <button
            onClick={handleClose}
            className="player-button hover:bg-[var(--color-bg-hover)] text-[var(--color-text-secondary)]"
          >
            <CloseIcon />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-6">
          {/* Profile section */}
          <section>
            <h3 className="text-sm font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-3">
              Profile
            </h3>

            <div className="space-y-4">
              {/* Display name */}
              <div>
                <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
                  Display Name
                </label>
                <input
                  type="text"
                  className="input"
                  value={nameInput}
                  onChange={(e) => setNameInput(e.target.value)}
                  placeholder="Enter your display name"
                />
              </div>

              {/* Peer ID */}
              <div>
                <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
                  Peer ID
                </label>
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    className="input font-mono text-xs"
                    value={peerId || 'Not initialized'}
                    readOnly
                  />
                  <button
                    className="btn btn-secondary px-3"
                    onClick={() => {
                      if (peerId) {
                        navigator.clipboard.writeText(peerId);
                      }
                    }}
                  >
                    Copy
                  </button>
                </div>
                <p className="mt-1 text-xs text-[var(--color-text-muted)]">
                  Your unique identifier on the WRAITH network
                </p>
              </div>
            </div>
          </section>

          {/* Video section */}
          <section>
            <h3 className="text-sm font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-3">
              Video
            </h3>

            <div className="space-y-4">
              {/* Default quality */}
              <div>
                <label className="block text-sm font-medium text-[var(--color-text-primary)] mb-1.5">
                  Default Quality
                </label>
                <select className="input">
                  <option value="auto">Auto</option>
                  <option value="1080p">1080p</option>
                  <option value="720p">720p</option>
                  <option value="480p">480p</option>
                  <option value="240p">240p</option>
                </select>
              </div>

              {/* Autoplay */}
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-[var(--color-text-primary)]">
                    Autoplay videos
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    Automatically start playback when loading a stream
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" defaultChecked />
                  <div className="w-11 h-6 bg-[var(--color-bg-tertiary)] peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-[var(--color-primary-600)]"></div>
                </label>
              </div>
            </div>
          </section>

          {/* About section */}
          <section>
            <h3 className="text-sm font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-3">
              About
            </h3>

            <div className="card p-4 bg-[var(--color-bg-tertiary)]">
              <div className="flex items-center gap-3 mb-3">
                <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-[var(--color-primary-500)] to-[var(--color-accent-500)] flex items-center justify-center">
                  <svg width="20" height="20" viewBox="0 0 32 32" fill="white">
                    <path d="M8 10L16 6L24 10V22L16 26L8 22V10Z" fillOpacity="0.2" />
                    <path d="M16 6L24 10L16 14L8 10L16 6Z" />
                    <path d="M16 14V26L8 22V10L16 14Z" fillOpacity="0.7" />
                    <path d="M16 14V26L24 22V10L16 14Z" fillOpacity="0.4" />
                  </svg>
                </div>
                <div>
                  <p className="font-semibold text-[var(--color-text-primary)]">
                    WRAITH Stream
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    Version 1.7.2
                  </p>
                </div>
              </div>
              <p className="text-sm text-[var(--color-text-secondary)]">
                Encrypted peer-to-peer media streaming using the WRAITH protocol.
                All content is end-to-end encrypted and distributed across the network.
              </p>
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t border-[var(--color-border-primary)]">
          {saveError && (
            <p className="text-sm text-[var(--color-error)]">{saveError}</p>
          )}
          <div className="flex items-center gap-3 ml-auto">
            <button className="btn btn-secondary" onClick={handleClose}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              onClick={handleSave}
              disabled={isSaving || !nameInput.trim()}
            >
              {isSaving ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
