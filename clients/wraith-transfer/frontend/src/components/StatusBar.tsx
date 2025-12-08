// WRAITH Transfer - Status Bar Component

import { useNodeStore } from '../stores/nodeStore';

interface Props {
  onNewTransfer: () => void;
}

export function StatusBar({ onNewTransfer }: Props) {
  const { status, error } = useNodeStore();

  return (
    <footer className="bg-bg-secondary border-t border-slate-700 px-6 py-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          {error && (
            <span className="text-sm text-red-400">{error}</span>
          )}
          {!error && status?.running && (
            <span className="text-sm text-slate-400">Ready to transfer</span>
          )}
          {!error && !status?.running && (
            <span className="text-sm text-slate-500">Start node to begin transferring</span>
          )}
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={onNewTransfer}
            disabled={!status?.running}
            className={`px-4 py-2 bg-wraith-accent hover:bg-cyan-600 rounded-lg text-white font-medium text-sm transition-colors ${
              !status?.running ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            New Transfer
          </button>
        </div>
      </div>
    </footer>
  );
}
