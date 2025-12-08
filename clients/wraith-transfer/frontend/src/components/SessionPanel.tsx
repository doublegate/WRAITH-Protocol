// WRAITH Transfer - Session Panel Component

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

  return (
    <div className="bg-bg-secondary rounded-lg p-3 border border-slate-700">
      <div className="flex items-center justify-between mb-2">
        <div className="font-mono text-sm text-white">
          {session.peer_id.slice(0, 16)}...
        </div>
        <button
          onClick={() => closeSession(session.peer_id)}
          className="text-xs text-slate-400 hover:text-red-500 transition-colors"
          title="Close session"
        >
          Disconnect
        </button>
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs text-slate-400">
        <div>
          <span className="text-slate-500">Sent: </span>
          {formatBytes(session.bytes_sent)}
        </div>
        <div>
          <span className="text-slate-500">Recv: </span>
          {formatBytes(session.bytes_received)}
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
