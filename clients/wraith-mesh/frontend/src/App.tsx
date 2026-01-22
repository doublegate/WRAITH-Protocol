// Main App Component - WRAITH Mesh

import { useEffect } from 'react';
import { useNetworkStore } from './stores/networkStore';
import { useUiStore, type TabId } from './stores/uiStore';
import NetworkGraph from './components/NetworkGraph';
import StatsDashboard from './components/StatsDashboard';
import DhtViewer from './components/DhtViewer';
import DiagnosticsPanel from './components/DiagnosticsPanel';
import Header from './components/Header';
import PeerList from './components/PeerList';

export default function App() {
  const { activeTab, setActiveTab, fetchLocalPeerId, monitoringActive, monitorInterval } = useUiStore();
  const { fetchSnapshot, fetchMetricsHistory, error, clearError } = useNetworkStore();

  // Initialize app
  useEffect(() => {
    fetchLocalPeerId();
    fetchSnapshot();
    fetchMetricsHistory(60);
  }, [fetchLocalPeerId, fetchSnapshot, fetchMetricsHistory]);

  // Periodic refresh when monitoring is active
  useEffect(() => {
    if (!monitoringActive) return;

    const interval = setInterval(() => {
      fetchSnapshot();
      fetchMetricsHistory(60);
    }, monitorInterval);

    return () => clearInterval(interval);
  }, [monitoringActive, monitorInterval, fetchSnapshot, fetchMetricsHistory]);

  const tabs: { id: TabId; label: string; icon: JSX.Element }[] = [
    {
      id: 'graph',
      label: 'Network Graph',
      icon: (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      ),
    },
    {
      id: 'stats',
      label: 'Statistics',
      icon: (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>
      ),
    },
    {
      id: 'dht',
      label: 'DHT Explorer',
      icon: (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
        </svg>
      ),
    },
    {
      id: 'diagnostics',
      label: 'Diagnostics',
      icon: (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      ),
    },
  ];

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col">
      <Header />

      {/* Error Banner */}
      {error && (
        <div className="mx-4 mt-2 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm flex items-center justify-between">
          <span>{error}</span>
          <button
            onClick={clearError}
            className="text-red-400 hover:text-red-300 transition-colors"
            aria-label="Dismiss error"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      )}

      {/* Main Content Area */}
      <main className="flex-1 flex overflow-hidden">
        {/* Left Sidebar - Tab Navigation */}
        <nav className="w-48 bg-bg-secondary border-r border-slate-700 flex flex-col">
          <div className="p-2 space-y-1">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                  activeTab === tab.id
                    ? 'bg-wraith-primary/20 text-wraith-primary'
                    : 'text-slate-400 hover:text-white hover:bg-slate-700'
                }`}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
          </div>

          {/* Peer List in Sidebar */}
          <div className="flex-1 border-t border-slate-700 overflow-hidden">
            <PeerList />
          </div>
        </nav>

        {/* Main Content */}
        <div className="flex-1 overflow-hidden">
          {activeTab === 'graph' && <NetworkGraph />}
          {activeTab === 'stats' && <StatsDashboard />}
          {activeTab === 'dht' && <DhtViewer />}
          {activeTab === 'diagnostics' && <DiagnosticsPanel />}
        </div>
      </main>

      {/* Status Bar */}
      <footer className="bg-bg-secondary border-t border-slate-700 px-4 py-2 flex items-center justify-between text-xs text-slate-500">
        <div className="flex items-center gap-4">
          <span>WRAITH Mesh v1.7.2</span>
          <span className="flex items-center gap-1">
            <div className={`w-1.5 h-1.5 rounded-full ${monitoringActive ? 'bg-green-500' : 'bg-slate-500'}`} />
            {monitoringActive ? 'Monitoring Active' : 'Monitoring Paused'}
          </span>
        </div>
        <div className="flex items-center gap-4">
          <span>Update Interval: {monitorInterval}ms</span>
        </div>
      </footer>
    </div>
  );
}
