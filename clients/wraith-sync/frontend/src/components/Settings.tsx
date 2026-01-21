// Settings Component - Application configuration

import { useEffect, useState } from 'react';
import { useConfigStore } from '../stores/configStore';
import type { AppSettings } from '../types';

export default function Settings() {
  const {
    settings,
    devices,
    globalPatterns,
    loadSettings,
    updateSettings,
    loadDevices,
    removeDevice,
    loadIgnoredPatterns,
    addIgnoredPattern,
    loading,
    error,
  } = useConfigStore();

  // Use settings as initial value when available, allowing local edits
  const [localSettings, setLocalSettings] = useState<AppSettings | null>(null);
  const [newPattern, setNewPattern] = useState('');
  const [activeSection, setActiveSection] = useState<
    'general' | 'sync' | 'devices' | 'patterns'
  >('general');

  useEffect(() => {
    loadSettings();
    loadDevices();
    loadIgnoredPatterns();
  }, [loadSettings, loadDevices, loadIgnoredPatterns]);

  // Initialize local settings when store settings first load
  const settingsInitialized = localSettings !== null;
  useEffect(() => {
    if (settings && !settingsInitialized) {
      setLocalSettings(settings);
    }
  }, [settings, settingsInitialized]);

  const handleSave = async () => {
    if (localSettings) {
      await updateSettings(localSettings);
    }
  };

  const handleAddPattern = async () => {
    if (newPattern.trim()) {
      await addIgnoredPattern(newPattern.trim());
      setNewPattern('');
    }
  };

  const handleRemoveDevice = async (deviceId: string) => {
    if (confirm('Are you sure you want to remove this device?')) {
      await removeDevice(deviceId);
    }
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  if (!localSettings) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin h-8 w-8 border-2 border-wraith-primary border-t-transparent rounded-full" />
      </div>
    );
  }

  const sections = [
    { id: 'general' as const, label: 'General', icon: 'cog' },
    { id: 'sync' as const, label: 'Sync', icon: 'sync' },
    { id: 'devices' as const, label: 'Devices', icon: 'devices' },
    { id: 'patterns' as const, label: 'Ignored Patterns', icon: 'ban' },
  ];

  return (
    <div className="flex h-full">
      {/* Sidebar navigation */}
      <div className="w-64 border-r border-gray-700 p-4">
        <nav className="space-y-1">
          {sections.map((section) => (
            <button
              key={section.id}
              onClick={() => setActiveSection(section.id)}
              className={`w-full px-4 py-2 text-left rounded transition-colors ${
                activeSection === section.id
                  ? 'bg-wraith-primary text-white'
                  : 'hover:bg-wraith-dark text-gray-300'
              }`}
            >
              {section.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Settings content */}
      <div className="flex-1 overflow-y-auto p-6">
        {error && (
          <div className="mb-4 p-3 bg-red-500/20 border border-red-500 rounded text-sm text-red-400">
            {error}
          </div>
        )}

        {/* General Settings */}
        {activeSection === 'general' && (
          <div className="space-y-6">
            <h2 className="text-xl font-semibold mb-4">General Settings</h2>

            {/* Device Name */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Device Name
              </label>
              <input
                type="text"
                value={localSettings.device_name}
                onChange={(e) =>
                  setLocalSettings({ ...localSettings, device_name: e.target.value })
                }
                className="w-full max-w-md px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
              />
              <p className="text-xs text-gray-500 mt-1">
                This name will be shown to other devices
              </p>
            </div>

            {/* Theme */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Theme
              </label>
              <select
                value={localSettings.theme}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    theme: e.target.value as 'light' | 'dark' | 'system',
                  })
                }
                className="w-full max-w-md px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
              >
                <option value="system">System</option>
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </div>

            {/* Auto Start */}
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="auto_start"
                checked={localSettings.auto_start}
                onChange={(e) =>
                  setLocalSettings({ ...localSettings, auto_start: e.target.checked })
                }
                className="w-4 h-4 rounded border-gray-700 bg-wraith-darker"
              />
              <label htmlFor="auto_start" className="text-sm text-gray-300">
                Start syncing automatically when app opens
              </label>
            </div>

            {/* Notifications */}
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="notifications"
                checked={localSettings.notifications_enabled}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    notifications_enabled: e.target.checked,
                  })
                }
                className="w-4 h-4 rounded border-gray-700 bg-wraith-darker"
              />
              <label htmlFor="notifications" className="text-sm text-gray-300">
                Show desktop notifications
              </label>
            </div>

            <button
              onClick={handleSave}
              disabled={loading}
              className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded transition-colors disabled:opacity-50"
            >
              {loading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        )}

        {/* Sync Settings */}
        {activeSection === 'sync' && (
          <div className="space-y-6">
            <h2 className="text-xl font-semibold mb-4">Sync Settings</h2>

            {/* Conflict Strategy */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Conflict Resolution
              </label>
              <select
                value={localSettings.conflict_strategy}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    conflict_strategy: e.target.value as
                      | 'last_writer_wins'
                      | 'keep_both'
                      | 'manual',
                  })
                }
                className="w-full max-w-md px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
              >
                <option value="last_writer_wins">
                  Last writer wins (automatic)
                </option>
                <option value="keep_both">Keep both versions</option>
                <option value="manual">Ask me each time</option>
              </select>
              <p className="text-xs text-gray-500 mt-1">
                How to handle files modified on multiple devices
              </p>
            </div>

            {/* Delta Sync */}
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="delta_sync"
                checked={localSettings.enable_delta_sync}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    enable_delta_sync: e.target.checked,
                  })
                }
                className="w-4 h-4 rounded border-gray-700 bg-wraith-darker"
              />
              <label htmlFor="delta_sync" className="text-sm text-gray-300">
                Enable delta sync (only transfer changes)
              </label>
            </div>

            {/* Bandwidth Limits */}
            <div className="grid grid-cols-2 gap-4 max-w-md">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Upload Limit (MB/s)
                </label>
                <input
                  type="number"
                  min="0"
                  step="0.1"
                  value={localSettings.upload_limit / (1024 * 1024)}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      upload_limit: parseFloat(e.target.value) * 1024 * 1024,
                    })
                  }
                  className="w-full px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                  placeholder="0 = unlimited"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Download Limit (MB/s)
                </label>
                <input
                  type="number"
                  min="0"
                  step="0.1"
                  value={localSettings.download_limit / (1024 * 1024)}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      download_limit: parseFloat(e.target.value) * 1024 * 1024,
                    })
                  }
                  className="w-full px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                  placeholder="0 = unlimited"
                />
              </div>
            </div>

            {/* Version History */}
            <div className="grid grid-cols-2 gap-4 max-w-md">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Max Versions to Keep
                </label>
                <input
                  type="number"
                  min="1"
                  max="100"
                  value={localSettings.max_versions}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      max_versions: parseInt(e.target.value),
                    })
                  }
                  className="w-full px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Version Retention (days)
                </label>
                <input
                  type="number"
                  min="1"
                  max="365"
                  value={localSettings.version_retention_days}
                  onChange={(e) =>
                    setLocalSettings({
                      ...localSettings,
                      version_retention_days: parseInt(e.target.value),
                    })
                  }
                  className="w-full px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                />
              </div>
            </div>

            {/* Debounce */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Debounce Interval (ms)
              </label>
              <input
                type="number"
                min="50"
                max="5000"
                step="50"
                value={localSettings.debounce_ms}
                onChange={(e) =>
                  setLocalSettings({
                    ...localSettings,
                    debounce_ms: parseInt(e.target.value),
                  })
                }
                className="w-full max-w-md px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
              />
              <p className="text-xs text-gray-500 mt-1">
                Wait time before syncing after file changes (lower = faster, but
                more CPU usage)
              </p>
            </div>

            <button
              onClick={handleSave}
              disabled={loading}
              className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded transition-colors disabled:opacity-50"
            >
              {loading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        )}

        {/* Devices */}
        {activeSection === 'devices' && (
          <div className="space-y-6">
            <h2 className="text-xl font-semibold mb-4">Connected Devices</h2>

            {devices.length === 0 ? (
              <div className="text-center text-gray-400 py-8">
                <p>No devices connected</p>
                <p className="text-sm mt-1">
                  Other devices will appear here when they connect
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                {devices.map((device) => (
                  <div
                    key={device.device_id}
                    className={`p-4 rounded-lg border ${
                      device.is_self
                        ? 'border-wraith-primary bg-wraith-primary/5'
                        : 'border-gray-700 bg-wraith-dark'
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div
                          className={`p-2 rounded ${
                            device.is_self ? 'bg-wraith-primary/20' : 'bg-gray-700'
                          }`}
                        >
                          <svg
                            className="w-5 h-5"
                            fill="currentColor"
                            viewBox="0 0 20 20"
                          >
                            <path
                              fillRule="evenodd"
                              d="M3 5a2 2 0 012-2h10a2 2 0 012 2v8a2 2 0 01-2 2h-2.22l.123.489.804.321A1 1 0 0113.323 17H6.677a1 1 0 01-.386-1.923l.804-.32.123-.49H5a2 2 0 01-2-2V5zm14.5 0a.5.5 0 00-.5-.5H3a.5.5 0 00-.5.5v8a.5.5 0 00.5.5h14a.5.5 0 00.5-.5V5z"
                              clipRule="evenodd"
                            />
                          </svg>
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-medium">{device.device_name}</span>
                            {device.is_self && (
                              <span className="px-2 py-0.5 text-xs rounded-full bg-wraith-primary text-white">
                                This device
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-gray-500 mt-1">
                            Last seen: {formatDate(device.last_seen)}
                          </p>
                        </div>
                      </div>
                      {!device.is_self && (
                        <button
                          onClick={() => handleRemoveDevice(device.device_id)}
                          className="p-2 rounded hover:bg-red-500/20 text-red-400 transition-colors"
                          title="Remove device"
                        >
                          <svg
                            className="w-5 h-5"
                            fill="currentColor"
                            viewBox="0 0 20 20"
                          >
                            <path
                              fillRule="evenodd"
                              d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                              clipRule="evenodd"
                            />
                          </svg>
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Ignored Patterns */}
        {activeSection === 'patterns' && (
          <div className="space-y-6">
            <h2 className="text-xl font-semibold mb-4">Ignored Patterns</h2>
            <p className="text-sm text-gray-400">
              Files matching these patterns will not be synced. Use glob syntax
              (e.g., *.tmp, node_modules/*, .git/**)
            </p>

            {/* Add new pattern */}
            <div className="flex gap-2 max-w-md">
              <input
                type="text"
                value={newPattern}
                onChange={(e) => setNewPattern(e.target.value)}
                placeholder="e.g., *.log, .DS_Store"
                className="flex-1 px-3 py-2 bg-wraith-darker border border-gray-700 rounded focus:border-wraith-primary focus:outline-none"
                onKeyDown={(e) => e.key === 'Enter' && handleAddPattern()}
              />
              <button
                onClick={handleAddPattern}
                disabled={!newPattern.trim()}
                className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded transition-colors disabled:opacity-50"
              >
                Add
              </button>
            </div>

            {/* Pattern list */}
            <div className="space-y-2 max-w-md">
              {globalPatterns.map((pattern, index) => (
                <div
                  key={index}
                  className="flex items-center justify-between px-3 py-2 bg-wraith-dark border border-gray-700 rounded"
                >
                  <code className="text-sm text-gray-300">{pattern}</code>
                  {/* Note: Remove functionality would need backend support */}
                </div>
              ))}
              {globalPatterns.length === 0 && (
                <p className="text-sm text-gray-500 py-2">
                  No custom patterns defined. Default patterns (.git, node_modules,
                  etc.) are always applied.
                </p>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
