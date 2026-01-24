// WRAITH Recon - Main Application

import { useEffect, useState } from 'react';
import {
  Header,
  ScopePanel,
  ReconPanel,
  ChannelPanel,
  AuditViewer,
  EngagementControls,
  RoeLoader,
  SettingsPanel,
} from './components';
import {
  useEngagementStore,
  useReconStore,
  useChannelStore,
  useAuditStore,
  useNodeStore,
  useSettingsStore,
} from './stores';

type ActivePanel = 'scope' | 'recon' | 'channels' | 'audit';

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [showRoeLoader, setShowRoeLoader] = useState(false);
  const [activePanel, setActivePanel] = useState<ActivePanel>('scope');

  const { status, fetchStatus, fetchScopeSummary, fetchTimingInfo, fetchKillSwitchState } = useEngagementStore();
  const { fetchPassiveStats, fetchDiscoveredAssets, fetchActiveScanProgress, fetchActiveScanResults } = useReconStore();
  const { fetchChannels } = useChannelStore();
  const { fetchEntries, fetchDatabaseStats } = useAuditStore();
  const { fetchStatus: fetchNodeStatus, fetchStatistics } = useNodeStore();
  const { theme, refreshIntervalMs } = useSettingsStore();

  // Initial fetch
  useEffect(() => {
    fetchStatus();
    fetchNodeStatus();
  }, [fetchStatus, fetchNodeStatus]);

  // Polling for updates when engagement is active
  useEffect(() => {
    if (status === 'NotLoaded' || status === 'Terminated') return;

    const interval = setInterval(() => {
      fetchStatus();
      fetchTimingInfo();
      fetchKillSwitchState();
      fetchNodeStatus();
      fetchStatistics();

      if (status === 'Active') {
        fetchPassiveStats();
        fetchDiscoveredAssets();
        fetchActiveScanProgress();
        fetchActiveScanResults();
        fetchChannels();
        fetchEntries(0, 20);
        fetchDatabaseStats();
      }
    }, refreshIntervalMs);

    return () => clearInterval(interval);
  }, [
    status, refreshIntervalMs,
    fetchStatus, fetchTimingInfo, fetchKillSwitchState,
    fetchNodeStatus, fetchStatistics,
    fetchPassiveStats, fetchDiscoveredAssets,
    fetchActiveScanProgress, fetchActiveScanResults,
    fetchChannels, fetchEntries, fetchDatabaseStats,
  ]);

  // Fetch scope summary when RoE is loaded
  useEffect(() => {
    if (status !== 'NotLoaded') {
      fetchScopeSummary();
    }
  }, [status, fetchScopeSummary]);

  // Apply theme to document root
  useEffect(() => {
    const root = document.documentElement;
    const applyTheme = (isDark: boolean) => {
      if (isDark) {
        root.classList.add('dark');
      } else {
        root.classList.remove('dark');
      }
    };

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      applyTheme(mediaQuery.matches);

      const listener = (e: MediaQueryListEvent) => applyTheme(e.matches);
      mediaQuery.addEventListener('change', listener);
      return () => mediaQuery.removeEventListener('change', listener);
    } else {
      applyTheme(theme === 'dark');
    }
  }, [theme]);

  const navItems: { id: ActivePanel; label: string; icon: string }[] = [
    { id: 'scope', label: 'Scope', icon: 'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2' },
    { id: 'recon', label: 'Recon', icon: 'M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z' },
    { id: 'channels', label: 'Channels', icon: 'M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z' },
    { id: 'audit', label: 'Audit', icon: 'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01' },
  ];

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col dark:bg-slate-950">
      <Header
        onOpenSettings={() => setShowSettings(true)}
        onOpenRoe={() => setShowRoeLoader(true)}
      />

      <main className="flex-1 flex overflow-hidden">
        {/* Left Sidebar - Navigation & Controls */}
        <div className="w-72 flex flex-col border-r border-border-primary bg-bg-secondary">
          {/* Navigation */}
          <nav className="p-2 border-b border-border-primary">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => setActivePanel(item.id)}
                className={`
                  w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
                  ${activePanel === item.id
                    ? 'bg-primary-500/10 text-primary-400'
                    : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary'}
                `}
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={item.icon} />
                </svg>
                {item.label}
              </button>
            ))}
          </nav>

          {/* Engagement Controls */}
          <div className="flex-1 overflow-auto p-2">
            <EngagementControls />
          </div>
        </div>

        {/* Main Content Area */}
        <div className="flex-1 flex overflow-hidden">
          {/* Primary Panel */}
          <div className="flex-1 p-4 overflow-auto">
            {activePanel === 'scope' && <ScopePanel />}
            {activePanel === 'recon' && <ReconPanel />}
            {activePanel === 'channels' && <ChannelPanel />}
            {activePanel === 'audit' && <AuditViewer />}
          </div>

          {/* Right Sidebar - Quick Stats */}
          <div className="w-64 p-4 border-l border-border-primary bg-bg-secondary overflow-auto">
            <QuickStats />
          </div>
        </div>
      </main>

      {/* Modals */}
      <RoeLoader
        isOpen={showRoeLoader}
        onClose={() => setShowRoeLoader(false)}
      />

      <SettingsPanel
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
      />
    </div>
  );
}

// Quick Stats Component (inline for simplicity)
function QuickStats() {
  const { status, roe, scopeSummary } = useEngagementStore();
  const { passiveStats, activeScanProgress } = useReconStore();
  const { channels } = useChannelStore();
  const { statistics } = useNodeStore();

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-medium text-text-secondary">Quick Stats</h3>

      {/* Engagement Status */}
      <div className="p-3 rounded-lg bg-bg-tertiary">
        <div className="text-xs text-text-muted mb-1">Status</div>
        <div className={`text-lg font-bold ${
          status === 'Active' ? 'text-green-400' :
          status === 'Terminated' ? 'text-red-400' :
          status === 'Paused' ? 'text-yellow-400' :
          'text-text-primary'
        }`}>
          {status}
        </div>
        {roe && (
          <div className="text-xs text-text-muted mt-1 truncate" title={roe.title}>
            {roe.title}
          </div>
        )}
      </div>

      {/* Scope Stats */}
      {scopeSummary && (
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="text-xs text-text-muted mb-2">Scope</div>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <div className="text-green-400 font-bold">{scopeSummary.authorized_cidr_count}</div>
              <div className="text-xs text-text-muted">CIDRs</div>
            </div>
            <div>
              <div className="text-green-400 font-bold">{scopeSummary.authorized_domain_count}</div>
              <div className="text-xs text-text-muted">Domains</div>
            </div>
          </div>
        </div>
      )}

      {/* Passive Recon Stats */}
      {passiveStats && (
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="text-xs text-text-muted mb-2">Passive Recon</div>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <div className="text-primary-400 font-bold">{passiveStats.unique_ips}</div>
              <div className="text-xs text-text-muted">IPs Found</div>
            </div>
            <div>
              <div className="text-primary-400 font-bold">{passiveStats.services_identified}</div>
              <div className="text-xs text-text-muted">Services</div>
            </div>
          </div>
        </div>
      )}

      {/* Active Scan Progress */}
      {activeScanProgress && (
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="text-xs text-text-muted mb-2">Active Scan</div>
          <div className="mb-2">
            <div className="progress-bar">
              <div
                className="progress-bar-fill"
                style={{
                  width: `${(activeScanProgress.completed_probes / activeScanProgress.total_probes) * 100}%`
                }}
              />
            </div>
          </div>
          <div className="text-sm">
            <span className="text-green-400 font-bold">{activeScanProgress.open_ports_found}</span>
            <span className="text-text-muted"> open ports</span>
          </div>
        </div>
      )}

      {/* Channel Stats */}
      {channels.length > 0 && (
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="text-xs text-text-muted mb-2">Channels</div>
          <div className="text-lg font-bold text-primary-400">{channels.length}</div>
          <div className="text-xs text-text-muted">
            Active: {channels.filter(c => c.state === 'Active' || c.state === 'Open').length}
          </div>
        </div>
      )}

      {/* Overall Stats */}
      {statistics && (
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="text-xs text-text-muted mb-2">Overall</div>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span className="text-text-muted">Targets:</span>
              <span className="text-text-primary">{statistics.targets_scanned}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Ports:</span>
              <span className="text-text-primary">{statistics.ports_discovered}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Audit Entries:</span>
              <span className="text-text-primary">{statistics.audit_entries}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
