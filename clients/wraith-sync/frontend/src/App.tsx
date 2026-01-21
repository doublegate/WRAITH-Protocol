// Main App Component - WRAITH Sync

import { useEffect, useState } from 'react';
import { useSyncStore } from './stores/syncStore';
import { useConfigStore } from './stores/configStore';
import FolderList from './components/FolderList';
import SyncStatus from './components/SyncStatus';
import ConflictResolver from './components/ConflictResolver';
import VersionHistory from './components/VersionHistory';
import Settings from './components/Settings';

type TabId = 'folders' | 'conflicts' | 'versions' | 'settings';

export default function App() {
  const [activeTab, setActiveTab] = useState<TabId>('folders');

  const { refreshStatus, loadFolders, loadConflicts, status, conflicts } =
    useSyncStore();
  const { loadSettings, loadDevices } = useConfigStore();

  useEffect(() => {
    // Initialize app
    (async () => {
      await refreshStatus();
      await loadFolders();
      await loadConflicts();
      await loadSettings();
      await loadDevices();
    })();

    // Periodic refresh
    const interval = setInterval(() => {
      refreshStatus();
      loadFolders();
    }, 5000);

    return () => clearInterval(interval);
  }, [refreshStatus, loadFolders, loadConflicts, loadSettings, loadDevices]);

  const tabs: { id: TabId; label: string; badge?: number }[] = [
    { id: 'folders', label: 'Folders' },
    { id: 'conflicts', label: 'Conflicts', badge: conflicts.length },
    { id: 'versions', label: 'Versions' },
    { id: 'settings', label: 'Settings' },
  ];

  return (
    <div className="flex flex-col h-screen bg-bg-primary text-slate-200">
      {/* Header with status */}
      <header className="bg-bg-secondary border-b border-slate-700 p-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-white">WRAITH Sync</h1>
          <SyncStatus />
        </div>
      </header>

      {/* Tab navigation */}
      <nav className="bg-bg-secondary border-b border-slate-700">
        <div className="flex">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-6 py-3 text-sm font-medium transition-colors relative ${
                activeTab === tab.id
                  ? 'text-wraith-primary border-b-2 border-wraith-primary'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              {tab.label}
              {tab.badge !== undefined && tab.badge > 0 && (
                <span className="ml-2 px-2 py-0.5 text-xs rounded-full bg-error text-white">
                  {tab.badge}
                </span>
              )}
            </button>
          ))}
        </div>
      </nav>

      {/* Main content */}
      <main className="flex-1 overflow-hidden">
        {activeTab === 'folders' && <FolderList />}
        {activeTab === 'conflicts' && <ConflictResolver />}
        {activeTab === 'versions' && <VersionHistory />}
        {activeTab === 'settings' && <Settings />}
      </main>

      {/* Footer */}
      <footer className="bg-bg-secondary border-t border-slate-700 px-4 py-2 text-xs text-slate-500">
        <div className="flex items-center justify-between">
          <span>
            {status
              ? `${status.total_files} files in ${status.total_folders} folders`
              : 'Loading...'}
          </span>
          <span>
            {status?.pending_operations
              ? `${status.pending_operations} pending`
              : 'All synced'}
          </span>
        </div>
      </footer>
    </div>
  );
}
