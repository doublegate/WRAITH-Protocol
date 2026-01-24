// WRAITH Recon - Settings Panel Component

import { useSettingsStore } from '../stores/settingsStore';
import { useNodeStore } from '../stores/nodeStore';

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
  const {
    theme, notificationsEnabled, refreshIntervalMs,
    setTheme, setNotificationsEnabled, setRefreshInterval, resetSettings,
  } = useSettingsStore();

  const { status: nodeStatus, startNode, stopNode, loading: nodeLoading } = useNodeStore();

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content w-[450px]" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="p-4 border-b border-border-primary flex justify-between items-center">
          <h2 className="text-lg font-semibold text-text-primary">Settings</h2>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-bg-hover transition-colors"
          >
            <svg className="w-5 h-5 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="p-4 space-y-6">
          {/* Appearance Section */}
          <section>
            <h3 className="text-sm font-medium text-text-secondary mb-3">Appearance</h3>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-text-primary mb-2">Theme</label>
                <select
                  value={theme}
                  onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
                  className="input"
                >
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                  <option value="system">System</option>
                </select>
              </div>
            </div>
          </section>

          {/* Notifications Section */}
          <section>
            <h3 className="text-sm font-medium text-text-secondary mb-3">Notifications</h3>
            <label className="flex items-center justify-between cursor-pointer">
              <span className="text-sm text-text-primary">Enable Notifications</span>
              <div className="relative">
                <input
                  type="checkbox"
                  checked={notificationsEnabled}
                  onChange={(e) => setNotificationsEnabled(e.target.checked)}
                  className="sr-only peer"
                />
                <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-primary-500 transition-colors"></div>
                <div className="absolute left-1 top-1 w-4 h-4 bg-white rounded-full transition-transform peer-checked:translate-x-5"></div>
              </div>
            </label>
          </section>

          {/* Refresh Section */}
          <section>
            <h3 className="text-sm font-medium text-text-secondary mb-3">Data Refresh</h3>
            <div>
              <label className="block text-sm text-text-primary mb-2">
                Refresh Interval: {refreshIntervalMs / 1000}s
              </label>
              <input
                type="range"
                min={500}
                max={5000}
                step={500}
                value={refreshIntervalMs}
                onChange={(e) => setRefreshInterval(parseInt(e.target.value))}
                className="w-full h-2 rounded-lg appearance-none cursor-pointer bg-bg-tertiary"
              />
              <div className="flex justify-between text-xs text-text-muted mt-1">
                <span>0.5s</span>
                <span>5s</span>
              </div>
            </div>
          </section>

          {/* Network Section */}
          <section>
            <h3 className="text-sm font-medium text-text-secondary mb-3">WRAITH Node</h3>
            <div className="p-3 rounded-lg bg-bg-tertiary">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  <div className={`w-2.5 h-2.5 rounded-full ${nodeStatus?.running ? 'bg-green-500' : 'bg-gray-500'}`} />
                  <span className="text-sm text-text-primary">
                    {nodeStatus?.running ? 'Node Running' : 'Node Stopped'}
                  </span>
                </div>
                {nodeStatus?.running ? (
                  <button
                    onClick={stopNode}
                    disabled={nodeLoading}
                    className="btn btn-secondary text-sm"
                  >
                    Stop Node
                  </button>
                ) : (
                  <button
                    onClick={startNode}
                    disabled={nodeLoading}
                    className="btn btn-primary text-sm"
                  >
                    Start Node
                  </button>
                )}
              </div>
              {nodeStatus?.peer_id && (
                <div className="text-xs">
                  <span className="text-text-muted">Peer ID: </span>
                  <span className="text-text-secondary font-mono">
                    {nodeStatus.peer_id.slice(0, 16)}...
                  </span>
                </div>
              )}
              {nodeStatus?.active_routes !== undefined && (
                <div className="text-xs mt-1">
                  <span className="text-text-muted">Active Routes: </span>
                  <span className="text-text-secondary">{nodeStatus.active_routes}</span>
                </div>
              )}
            </div>
          </section>

          {/* Reset Section */}
          <section className="pt-4 border-t border-border-primary">
            <button
              onClick={resetSettings}
              className="btn btn-secondary w-full"
            >
              Reset to Defaults
            </button>
          </section>
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-border-primary text-center">
          <p className="text-xs text-text-muted">
            WRAITH Recon v2.1.1 - Security Assessment Platform
          </p>
        </div>
      </div>
    </div>
  );
}
