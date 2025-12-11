// WRAITH Transfer - Header Component

import { useState } from 'react';
import { useNodeStore } from '../stores/nodeStore';
import { useSettingsStore } from '../stores/settingsStore';

interface Props {
  onOpenSettings: () => void;
}

export function Header({ onOpenSettings }: Props) {
  const { status, loading, startNode, stopNode } = useNodeStore();
  const { theme, setTheme } = useSettingsStore();
  const [copied, setCopied] = useState(false);

  const isRunning = status?.running ?? false;

  const toggleTheme = () => {
    if (theme === 'light') {
      setTheme('dark');
    } else if (theme === 'dark') {
      setTheme('system');
    } else {
      setTheme('light');
    }
  };

  const getThemeIcon = () => {
    switch (theme) {
      case 'light':
        return '☀';
      case 'dark':
        return '☾';
      case 'system':
        return '⚙';
    }
  };

  const handleCopyNodeId = async () => {
    if (!status?.node_id) return;

    try {
      await navigator.clipboard.writeText(status.node_id);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy node ID:', err);
    }
  };

  return (
    <header className="bg-bg-secondary border-b border-slate-700 px-6 py-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold text-white">WRAITH Transfer</h1>
          <div className="flex items-center gap-2">
            <div
              className={`w-2 h-2 rounded-full ${
                isRunning ? 'bg-green-500' : 'bg-red-500'
              }`}
            />
            <span className="text-sm text-slate-400">
              {isRunning ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-4">
          {status?.node_id && (
            <div
              onClick={handleCopyNodeId}
              className="text-sm text-slate-400 cursor-pointer hover:text-slate-200 transition-colors group relative"
              title={`${status.node_id}\nClick to copy`}
            >
              <span className="text-slate-500">Node: </span>
              <span className="font-mono">
                {status.node_id.slice(0, 12)}...{status.node_id.slice(-4)}
              </span>
              {copied && (
                <span className="absolute -top-8 left-1/2 -translate-x-1/2 bg-green-600 text-white text-xs px-2 py-1 rounded whitespace-nowrap">
                  Copied to clipboard!
                </span>
              )}
            </div>
          )}

          <div className="flex items-center gap-2 text-sm text-slate-400">
            <span>{status?.active_sessions ?? 0} sessions</span>
            <span className="text-slate-600">|</span>
            <span>{status?.active_transfers ?? 0} transfers</span>
          </div>

          <button
            onClick={toggleTheme}
            className="p-2 text-slate-400 hover:text-white transition-colors text-lg"
            title={`Theme: ${theme} (click to cycle)`}
            aria-label={`Current theme: ${theme}. Click to cycle themes`}
          >
            {getThemeIcon()}
          </button>

          <button
            onClick={onOpenSettings}
            className="p-2 text-slate-400 hover:text-white transition-colors"
            title="Settings"
            aria-label="Open settings"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </button>

          <button
            onClick={isRunning ? stopNode : startNode}
            disabled={loading}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition-colors ${
              isRunning
                ? 'bg-red-600 hover:bg-red-700 text-white'
                : 'bg-wraith-primary hover:bg-wraith-secondary text-white'
            } ${loading ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            {loading ? 'Loading...' : isRunning ? 'Stop Node' : 'Start Node'}
          </button>
        </div>
      </div>
    </header>
  );
}
