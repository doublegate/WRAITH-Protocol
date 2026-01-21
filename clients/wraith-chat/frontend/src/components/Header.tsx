// WRAITH Chat - Header Component

import { useState } from "react";
import { useNodeStore } from "../stores/nodeStore";

interface HeaderProps {
  onOpenSettings: () => void;
}

export default function Header({ onOpenSettings }: HeaderProps) {
  const { status } = useNodeStore();
  const [copied, setCopied] = useState(false);

  const isConnected = status?.running ?? false;

  const handleCopyPeerId = async () => {
    if (!status?.local_peer_id) return;

    try {
      await navigator.clipboard.writeText(status.local_peer_id);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy peer ID:", err);
    }
  };

  return (
    <header className="bg-bg-secondary border-b border-slate-700 px-6 py-3 flex-shrink-0">
      <div className="flex items-center justify-between">
        {/* Left: Logo and Title */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-3">
            {/* WRAITH Logo */}
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center">
              <svg
                className="w-5 h-5 text-white"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
                />
              </svg>
            </div>
            <h1 className="text-lg font-semibold text-white">WRAITH Chat</h1>
          </div>

          {/* Connection Status */}
          <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-bg-primary">
            <div
              className={`w-2 h-2 rounded-full ${
                isConnected ? "bg-green-500 animate-pulse" : "bg-red-500"
              }`}
            />
            <span className="text-sm text-slate-400">
              {isConnected ? "Connected" : "Offline"}
            </span>
          </div>
        </div>

        {/* Right: Peer ID, Stats, and Settings */}
        <div className="flex items-center gap-4">
          {/* Peer ID */}
          {status?.local_peer_id && (
            <button
              onClick={handleCopyPeerId}
              className="flex items-center gap-2 text-sm text-slate-400 cursor-pointer hover:text-slate-200 transition-colors group relative px-3 py-1.5 rounded bg-bg-primary hover:bg-bg-tertiary"
              title={`${status.local_peer_id}\nClick to copy`}
            >
              <span className="text-slate-500">Peer:</span>
              <span className="font-mono">
                {status.local_peer_id.slice(0, 8)}...
                {status.local_peer_id.slice(-4)}
              </span>
              <CopyIcon className="w-3.5 h-3.5 opacity-50 group-hover:opacity-100" />
              {copied && (
                <span className="absolute -bottom-8 left-1/2 -translate-x-1/2 bg-green-600 text-white text-xs px-2 py-1 rounded whitespace-nowrap z-10">
                  Copied!
                </span>
              )}
            </button>
          )}

          {/* Stats */}
          {status && (
            <div className="flex items-center gap-2 text-sm text-slate-400 px-3 py-1.5 rounded bg-bg-primary">
              <span>{status.session_count} sessions</span>
              <span className="text-slate-600">|</span>
              <span>{status.active_conversations} chats</span>
            </div>
          )}

          {/* Settings Button */}
          <button
            onClick={onOpenSettings}
            className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
            title="Settings"
            aria-label="Open settings"
          >
            <SettingsIcon className="w-5 h-5" />
          </button>
        </div>
      </div>
    </header>
  );
}

// Icons
function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
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
  );
}

function CopyIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
      />
    </svg>
  );
}
