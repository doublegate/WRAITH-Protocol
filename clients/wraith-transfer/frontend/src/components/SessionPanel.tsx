// WRAITH Transfer - Session Panel Component

import { useState, useEffect } from 'react';
import { useSessionStore } from '../stores/sessionStore';
import type { SessionInfo } from '../types';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function SessionItem({ session }: { session: SessionInfo }) {
  const { closeSession } = useSessionStore();
  const [duration, setDuration] = useState(0);

  // Calculate connection duration with interval
  useEffect(() => {
    const updateDuration = () => {
      const now = Math.floor(Date.now() / 1000);
      setDuration(now - session.established_at);
    };
    updateDuration();
    const interval = setInterval(updateDuration, 1000);
    return () => clearInterval(interval);
  }, [session.established_at]);

  const connectionStatus = session.connection_status || 'connected';
  const statusColors: Record<string, string> = {
    connecting: 'text-yellow-500',
    connected: 'text-green-500',
    disconnecting: 'text-orange-500',
    failed: 'text-red-500',
  };

  const statusDots: Record<string, string> = {
    connecting: 'bg-yellow-500 animate-pulse',
    connected: 'bg-green-500',
    disconnecting: 'bg-orange-500 animate-pulse',
    failed: 'bg-red-500',
  };
  const formatDuration = (seconds: number): string => {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    return `${Math.floor(seconds / 3600)}h`;
  };

  return (
    <div className="bg-bg-secondary rounded-lg p-3 border border-slate-700">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div
            className={`w-2 h-2 rounded-full ${statusDots[connectionStatus]}`}
            title={connectionStatus}
          />
          <div className="font-mono text-sm text-white">
            {session.nickname || `${session.peer_id.slice(0, 16)}...`}
          </div>
        </div>
        <button
          onClick={() => closeSession(session.peer_id)}
          className="text-xs text-slate-400 hover:text-red-500 transition-colors"
          title="Close session"
        >
          ✕
        </button>
      </div>

      <div className="space-y-1 text-xs text-slate-400">
        <div className="flex justify-between">
          <span className="text-slate-500">Status:</span>
          <span className={statusColors[connectionStatus]}>
            {connectionStatus}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Duration:</span>
          <span>{formatDuration(duration)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Sent:</span>
          <span>{formatBytes(session.bytes_sent)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Recv:</span>
          <span>{formatBytes(session.bytes_received)}</span>
        </div>
      </div>
    </div>
  );
}

export function SessionPanel() {
  const { sessions } = useSessionStore();

  return (
    <div className="w-72 bg-bg-primary border-l border-slate-700 flex flex-col">
      <div className="p-4 border-b border-slate-700">
        <h2 className="font-semibold text-white">Active Sessions</h2>
        <p className="text-sm text-slate-400">{sessions.length} connected</p>
      </div>

      <div className="flex-1 overflow-auto p-4 space-y-2">
        {sessions.length === 0 ? (
          <div className="text-center text-slate-500 text-sm py-4">
            No active sessions
          </div>
        ) : (
          sessions.map((session) => (
            <SessionItem key={session.peer_id} session={session} />
          ))
        )}
      </div>
    </div>
  );
}
