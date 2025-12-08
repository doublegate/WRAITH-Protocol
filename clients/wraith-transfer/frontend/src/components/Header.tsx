// WRAITH Transfer - Header Component

import { useNodeStore } from '../stores/nodeStore';

export function Header() {
  const { status, loading, startNode, stopNode } = useNodeStore();

  const isRunning = status?.running ?? false;

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
            <div className="text-sm text-slate-400">
              <span className="text-slate-500">Node: </span>
              <span className="font-mono">{status.node_id.slice(0, 8)}...</span>
            </div>
          )}

          <div className="flex items-center gap-2 text-sm text-slate-400">
            <span>{status?.active_sessions ?? 0} sessions</span>
            <span className="text-slate-600">|</span>
            <span>{status?.active_transfers ?? 0} transfers</span>
          </div>

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
