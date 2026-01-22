// Header Component - WRAITH Mesh

import { useState } from 'react';
import { useNetworkStore } from '../stores/networkStore';
import { useUiStore } from '../stores/uiStore';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

export default function Header() {
  const { localPeerId, monitoringActive, setMonitoringActive, monitorInterval, setMonitorInterval } = useUiStore();
  const { snapshot, exportData, exportMetrics } = useNetworkStore();
  const [copied, setCopied] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const handleCopyPeerId = async () => {
    if (localPeerId) {
      try {
        await navigator.clipboard.writeText(localPeerId);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
      }
    }
  };

  const handleExport = async (format: 'json' | 'csv') => {
    try {
      const data = await exportData(format);
      const filePath = await save({
        defaultPath: `wraith-mesh-topology-${Date.now()}.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (filePath) {
        await writeTextFile(filePath, data);
      }
    } catch (err) {
      console.error('Export failed:', err);
    }
  };

  const handleExportMetrics = async () => {
    try {
      const data = await exportMetrics(3600);
      const filePath = await save({
        defaultPath: `wraith-mesh-metrics-${Date.now()}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (filePath) {
        await writeTextFile(filePath, data);
      }
    } catch (err) {
      console.error('Metrics export failed:', err);
    }
  };

  const nodeCount = snapshot?.nodes.length ?? 0;
  const linkCount = snapshot?.links.length ?? 0;
  const healthScore = snapshot?.health_score ?? 0;

  return (
    <header className="bg-bg-secondary border-b border-slate-700 px-4 py-3">
      <div className="flex items-center justify-between">
        {/* Left: Logo and Status */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <svg className="w-6 h-6 text-wraith-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <h1 className="text-lg font-bold text-white">WRAITH Mesh</h1>
          </div>

          {/* Network Health */}
          <div className="flex items-center gap-2 px-3 py-1 bg-slate-700/50 rounded-lg">
            <div
              className={`w-2 h-2 rounded-full ${
                healthScore >= 0.8
                  ? 'bg-green-500'
                  : healthScore >= 0.5
                  ? 'bg-yellow-500'
                  : 'bg-red-500'
              }`}
            />
            <span className="text-sm text-slate-300">
              Health: {(healthScore * 100).toFixed(0)}%
            </span>
          </div>

          {/* Node Count */}
          <div className="text-sm text-slate-400">
            <span className="text-white font-medium">{nodeCount}</span> nodes,{' '}
            <span className="text-white font-medium">{linkCount}</span> links
          </div>
        </div>

        {/* Right: Node ID and Actions */}
        <div className="flex items-center gap-4">
          {/* Node ID */}
          {localPeerId && (
            <div className="relative">
              <button
                onClick={handleCopyPeerId}
                className="text-sm text-slate-400 cursor-pointer hover:text-slate-200 transition-colors flex items-center gap-1"
                title={`${localPeerId}\nClick to copy`}
              >
                <span className="text-slate-500">Node:</span>
                <span className="font-mono">
                  {localPeerId.slice(0, 8)}...{localPeerId.slice(-4)}
                </span>
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
              </button>
              {copied && (
                <span className="absolute -top-8 left-1/2 -translate-x-1/2 bg-green-600 text-white text-xs px-2 py-1 rounded whitespace-nowrap">
                  Copied!
                </span>
              )}
            </div>
          )}

          {/* Monitoring Toggle */}
          <button
            onClick={() => setMonitoringActive(!monitoringActive)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
              monitoringActive
                ? 'bg-green-600/20 text-green-400 hover:bg-green-600/30'
                : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
            }`}
          >
            {monitoringActive ? (
              <>
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Pause
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Resume
              </>
            )}
          </button>

          {/* Export Menu */}
          <div className="relative group">
            <button
              className="p-2 text-slate-400 hover:text-white transition-colors rounded-lg hover:bg-slate-700"
              aria-label="Export data"
            >
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </button>
            <div className="absolute right-0 top-full mt-1 bg-bg-secondary border border-slate-700 rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
              <button
                onClick={() => handleExport('json')}
                className="w-full px-4 py-2 text-sm text-left text-slate-300 hover:bg-slate-700 hover:text-white transition-colors"
              >
                Export as JSON
              </button>
              <button
                onClick={() => handleExport('csv')}
                className="w-full px-4 py-2 text-sm text-left text-slate-300 hover:bg-slate-700 hover:text-white transition-colors"
              >
                Export as CSV
              </button>
              <div className="border-t border-slate-700" />
              <button
                onClick={handleExportMetrics}
                className="w-full px-4 py-2 text-sm text-left text-slate-300 hover:bg-slate-700 hover:text-white transition-colors"
              >
                Export Metrics
              </button>
            </div>
          </div>

          {/* Settings */}
          <button
            onClick={() => setShowSettings(true)}
            className="p-2 text-slate-400 hover:text-white transition-colors rounded-lg hover:bg-slate-700"
            aria-label="Settings"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>

      {/* Settings Modal */}
      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
          monitorInterval={monitorInterval}
          setMonitorInterval={setMonitorInterval}
        />
      )}
    </header>
  );
}

function SettingsModal({
  onClose,
  monitorInterval,
  setMonitorInterval,
}: {
  onClose: () => void;
  monitorInterval: number;
  setMonitorInterval: (interval: number) => void;
}) {
  const { showLabels, setShowLabels, showIndirectPeers, setShowIndirectPeers, graphLayout, setGraphLayout } = useUiStore();

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        className="bg-bg-secondary rounded-xl border border-slate-700 p-6 w-full max-w-md"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-6">
          <h2 id="settings-title" className="text-xl font-semibold text-white">
            Settings
          </h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white transition-colors"
            aria-label="Close settings"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="space-y-6">
          {/* Monitor Interval */}
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Update Interval
            </label>
            <select
              value={monitorInterval}
              onChange={(e) => setMonitorInterval(Number(e.target.value))}
              className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            >
              <option value={500}>500ms (Fast)</option>
              <option value={1000}>1 second</option>
              <option value={2000}>2 seconds</option>
              <option value={5000}>5 seconds</option>
              <option value={10000}>10 seconds</option>
            </select>
          </div>

          {/* Graph Layout */}
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Graph Layout
            </label>
            <select
              value={graphLayout}
              onChange={(e) => setGraphLayout(e.target.value as 'force' | 'radial' | 'tree')}
              className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            >
              <option value="force">Force-Directed</option>
              <option value="radial">Radial</option>
              <option value="tree">Tree</option>
            </select>
          </div>

          {/* Toggle Options */}
          <div className="space-y-4">
            <ToggleOption
              label="Show Node Labels"
              enabled={showLabels}
              onChange={setShowLabels}
            />
            <ToggleOption
              label="Show Indirect Peers"
              enabled={showIndirectPeers}
              onChange={setShowIndirectPeers}
            />
          </div>
        </div>

        <div className="mt-8 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg text-white font-medium transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

function ToggleOption({
  label,
  enabled,
  onChange,
}: {
  label: string;
  enabled: boolean;
  onChange: (enabled: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-slate-300">{label}</span>
      <button
        onClick={() => onChange(!enabled)}
        className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
          enabled ? 'bg-wraith-primary' : 'bg-slate-600'
        }`}
        role="switch"
        aria-checked={enabled}
        aria-label={label}
      >
        <span
          className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
            enabled ? 'translate-x-6' : 'translate-x-1'
          }`}
        />
      </button>
    </div>
  );
}
