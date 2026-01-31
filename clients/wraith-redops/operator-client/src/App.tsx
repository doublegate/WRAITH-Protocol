import { useEffect, useState } from 'react';
import { useAppStore } from './stores/appStore';
import { useToastStore } from './stores/toastStore';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { ToastContainer } from './components/ui/Toast';
import { NetworkGraph } from './components/NetworkGraph';
import { EventLogWidget } from './components/EventLog';
import BeaconInteraction from './components/BeaconInteraction';
import PhishingBuilder from './components/PhishingBuilder';
import LootGallery from './components/LootGallery';
import AttackChainEditor from './components/AttackChainEditor';
import ListenerManager from './components/ListenerManager';
import ImplantDetailPanel from './components/ImplantDetailPanel';
import CampaignDetail from './components/CampaignDetail';
import ImplantGenerator from './components/ImplantGenerator';
import PlaybookBrowser from './components/PlaybookBrowser';
import EventLog from './components/EventLog';
import { ContextMenu, type ContextMenuItem } from './components/ui/ContextMenu';
import * as ipc from './lib/ipc';
import {
  LayoutDashboard,
  FolderOpen,
  Link,
  Radio,
  Headphones,
  Gem,
  Fish,
  Settings,
  Crosshair,
  BookOpen,
  Info,
  Sun,
  Moon,
} from 'lucide-react';

const NAV_ITEMS = [
// ...
function App() {
  const {
    theme,
    toggleTheme,
    activeTab,
// ...
    setActiveTab,
    serverStatus,
    serverAddress,
    setServerAddress,
    connect,
    implants,
    campaigns,
    listeners,
    interactingImplantId,
    setInteractingImplantId,
    showCreateCampaign,
    setShowCreateCampaign,
    refreshAll,
    refreshCampaigns,
    refreshImplants,
    autoRefreshInterval,
    setAutoRefreshInterval,
    authToken,
    doRefreshToken,
  } = useAppStore();

  const addToast = useToastStore((s) => s.addToast);

  useKeyboardShortcuts();

  // Campaign form state (kept local since it's ephemeral)
  const [newCampaignName, setNewCampaignName] = useState('');
  const [newCampaignDesc, setNewCampaignDesc] = useState('');

  // Campaign detail state
  const [selectedCampaignId, setSelectedCampaignId] = useState<string | null>(null);

  // Implant detail state
  const [selectedImplantId, setSelectedImplantId] = useState<string | null>(null);

  // Multi-selection state
  const [selectedImplants, setSelectedImplants] = useState<Set<string>>(new Set());

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItem[];
  } | null>(null);

  const handleCreateCampaign = async () => {
    try {
      await ipc.createCampaign(newCampaignName, newCampaignDesc);
      setShowCreateCampaign(false);
      setNewCampaignName('');
      setNewCampaignDesc('');
      refreshCampaigns();
      addToast('success', `Campaign "${newCampaignName}" created`);
    } catch (e) {
      addToast('error', 'Failed to create campaign: ' + e);
    }
  };

  useEffect(() => {
    connect();
    const interval = setInterval(refreshAll, autoRefreshInterval);
    return () => clearInterval(interval);
  }, [autoRefreshInterval]);

  const handleBeaconContextMenu = (
    e: React.MouseEvent,
    implant: { id: string; hostname: string },
  ) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        {
          label: 'Interact',
          onClick: () => {
            setInteractingImplantId(implant.id);
            setActiveTab('console');
          },
        },
        {
          label: 'View Details',
          onClick: () => setSelectedImplantId(implant.id),
        },
        {
          label: 'Copy ID',
          onClick: () => {
            navigator.clipboard.writeText(implant.id);
            addToast('info', 'Implant ID copied');
          },
        },
        {
          label: 'Kill Implant',
          variant: 'danger',
          onClick: async () => {
            try {
              await ipc.killImplant(implant.id);
              addToast('success', `Kill command sent to ${implant.hostname}`);
              refreshImplants();
            } catch (e) {
              addToast('error', 'Kill failed: ' + e);
            }
          },
        },
      ],
    });
  };

  const toggleImplantSelection = (id: string) => {
    setSelectedImplants((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleBulkKill = async () => {
    if (selectedImplants.size === 0) return;
    const ids = Array.from(selectedImplants);
    try {
      await Promise.all(ids.map((id) => ipc.killImplant(id)));
      addToast('success', `Kill command sent to ${ids.length} beacons`);
      setSelectedImplants(new Set());
      refreshImplants();
    } catch (e) {
      addToast('error', 'Bulk kill failed: ' + e);
    }
  };

  return (
    <div className="flex h-screen w-full flex-col font-mono bg-white text-slate-700 dark:bg-slate-950 dark:text-slate-300">
      <ToastContainer />

      {/* ... */}

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-48 border-r border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 p-2 text-xs flex flex-col gap-1">
          <div className="mt-2 mb-1 px-2 text-slate-400 dark:text-slate-600 font-bold uppercase text-[10px] tracking-wider">
            Operations
          </div>
          {NAV_ITEMS.map(({ label, icon: Icon }) => (
            <button
              key={label}
              onClick={() => setActiveTab(label.toLowerCase())}
              className={`w-full rounded px-3 py-2 text-left transition-colors flex items-center gap-2 ${
                activeTab === label.toLowerCase()
                  ? 'bg-red-600/10 text-red-500 border border-red-900/20'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
              }`}
            >
              <Icon className="w-3.5 h-3.5" />
              {label}
            </button>
          ))}

          <div className="mt-4 mb-1 px-2 text-slate-600 font-bold uppercase text-[10px] tracking-wider">
            System
          </div>
          <button
            onClick={() => setActiveTab('settings')}
            className={`w-full rounded px-3 py-2 text-left transition-colors flex items-center gap-2 ${
              activeTab === 'settings'
                ? 'bg-red-600/10 text-red-500 border border-red-900/20'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            Settings
          </button>
        </nav>

        {/* Content Area */}
        <main className="flex-1 overflow-auto bg-slate-950 p-4 relative">
          {/* Settings */}
          {activeTab === 'settings' && (
            <div className="max-w-md mx-auto mt-10 space-y-6">
              <div className="rounded border border-slate-800 bg-slate-900 p-6 shadow-lg">
                <h3 className="text-sm font-bold text-white mb-6 border-b border-slate-800 pb-2">
                  CONNECTION SETTINGS
                </h3>
                <div className="grid gap-4">
                  <div>
                    <label className="block text-xs text-slate-500 mb-1">
                      TEAM SERVER ADDRESS
                    </label>
                    <input
                      value={serverAddress}
                      onChange={(e) => setServerAddress(e.target.value)}
                      className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                    />
                  </div>
                  <button
                    onClick={() => connect(serverAddress)}
                    className="bg-red-600 text-white px-4 py-2 text-xs font-bold rounded hover:bg-red-500 transition-colors"
                  >
                    RECONNECT
                  </button>
                </div>
              </div>

              <div className="rounded border border-slate-800 bg-slate-900 p-6 shadow-lg">
                <h3 className="text-sm font-bold text-white mb-6 border-b border-slate-800 pb-2">
                  PREFERENCES
                </h3>
                <div className="grid gap-4">
                  <div>
                    <label className="block text-xs text-slate-500 mb-1">
                      AUTO-REFRESH INTERVAL
                    </label>
                    <select
                      value={autoRefreshInterval}
                      onChange={(e) => setAutoRefreshInterval(Number(e.target.value))}
                      className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                    >
                      <option value={2000}>2 seconds</option>
                      <option value={5000}>5 seconds (default)</option>
                      <option value={10000}>10 seconds</option>
                      <option value={30000}>30 seconds</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-xs text-slate-500 mb-1">AUTH TOKEN</label>
                    <div className="flex gap-2">
                      <input
                        readOnly
                        value={authToken || 'No token'}
                        className="flex-1 bg-slate-950 border border-slate-800 p-2 text-sm text-slate-500 rounded"
                      />
                      <button
                        onClick={doRefreshToken}
                        className="bg-slate-800 text-white px-3 py-2 text-xs font-bold rounded hover:bg-slate-700 transition-colors"
                      >
                        REFRESH
                      </button>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded border border-slate-800 bg-slate-900 p-6 shadow-lg">
                <h3 className="text-sm font-bold text-white mb-4 border-b border-slate-800 pb-2 flex items-center gap-2">
                  <Info className="w-4 h-4 text-slate-500" /> ABOUT
                </h3>
                <div className="text-xs text-slate-500 space-y-1">
                  <div>
                    WRAITH::REDOPS Operator Console v2.3.6
                  </div>
                  <div>Server: {serverAddress}</div>
                  <div>Status: {serverStatus}</div>
                  <div>
                    Shortcuts: Ctrl+1-9 tabs, Ctrl+R refresh
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Dashboard */}
          {activeTab === 'dashboard' && (
            <div className="flex flex-col h-full gap-4">
              {/* Primary Metrics */}
              <div className="grid grid-cols-4 gap-4">
                <div className="rounded border border-slate-800 bg-slate-900 p-4">
                  <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">
                    Active Campaigns
                  </div>
                  <div className="text-3xl font-bold text-white mt-2">{campaigns.length}</div>
                  <div className="text-[10px] text-slate-600 mt-1">
                    {campaigns.filter((c) => c.status === 'active').length} running
                  </div>
                </div>
                <div className="rounded border border-slate-800 bg-slate-900 p-4">
                  <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">
                    Live Beacons
                  </div>
                  <div className="text-3xl font-bold text-green-500 mt-2">
                    {implants.filter((i) => i.status === 'active').length}
                  </div>
                  <div className="text-[10px] text-slate-600 mt-1">
                    {implants.length} total registered
                  </div>
                </div>
                <div className="rounded border border-slate-800 bg-slate-900 p-4">
                  <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">
                    Active Listeners
                  </div>
                  <div className="text-3xl font-bold text-blue-500 mt-2">
                    {listeners.filter((l) => l.status === 'active').length}
                  </div>
                  <div className="text-[10px] text-slate-600 mt-1">
                    {listeners.length} configured
                  </div>
                </div>
                <div className="rounded border border-slate-800 bg-slate-900 p-4">
                  <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">
                    System Status
                  </div>
                  <div className="text-xl font-bold text-slate-300 mt-3 truncate">
                    {serverStatus === 'Connected' ? 'OPERATIONAL' : 'OFFLINE'}
                  </div>
                  <div className="text-[10px] text-slate-600 mt-1">{serverAddress}</div>
                </div>
              </div>

              {/* Network Graph (60%) + Events (40%) */}
              <div className="flex-1 min-h-0 grid grid-cols-5 gap-4">
                <div className="col-span-3 rounded border border-slate-800 bg-slate-900 overflow-hidden relative">
                  <div className="absolute top-2 left-2 z-10 text-[10px] text-slate-500 font-bold uppercase tracking-wider">
                    Operational Graph
                  </div>
                  <NetworkGraph implants={implants} />
                </div>
                <div className="col-span-2">
                  <EventLogWidget />
                </div>
              </div>
            </div>
          )}

          {/* Attack Chains */}
          {activeTab === 'attack chains' && <AttackChainEditor />}

          {/* Campaigns */}
          {activeTab === 'campaigns' && (
            <div className="flex flex-col h-full">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-sm font-bold text-white uppercase tracking-wider">
                  OPERATIONS / CAMPAIGNS
                </h2>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowCreateCampaign(true)}
                    className="rounded bg-red-600 px-3 py-1 text-xs text-white hover:bg-red-500 font-bold shadow-lg shadow-red-900/20 transition-all"
                  >
                    NEW CAMPAIGN
                  </button>
                  <button
                    onClick={refreshCampaigns}
                    className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700 transition-colors"
                  >
                    REFRESH
                  </button>
                </div>
              </div>

              {selectedCampaignId && (
                <CampaignDetail
                  campaignId={selectedCampaignId}
                  onClose={() => setSelectedCampaignId(null)}
                />
              )}

              {showCreateCampaign && (
                <div className="mb-6 p-6 rounded border border-red-900/50 bg-red-950/10 border-dashed">
                  <h3 className="text-xs font-bold text-red-500 mb-4 uppercase tracking-widest">
                    Initialization Wizard
                  </h3>
                  <div className="grid gap-4 max-w-lg">
                    <div>
                      <label className="text-[10px] text-slate-500 uppercase mb-1 block">
                        Codename
                      </label>
                      <input
                        placeholder="OP_GHOST"
                        value={newCampaignName}
                        onChange={(e) => setNewCampaignName(e.target.value)}
                        className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                      />
                    </div>
                    <div>
                      <label className="text-[10px] text-slate-500 uppercase mb-1 block">
                        Mission Objective
                      </label>
                      <textarea
                        placeholder="Describe the operational goals..."
                        value={newCampaignDesc}
                        onChange={(e) => setNewCampaignDesc(e.target.value)}
                        className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 min-h-[80px] rounded"
                      />
                    </div>
                    <div className="flex gap-2 mt-2">
                      <button
                        onClick={handleCreateCampaign}
                        className="bg-red-600 px-6 py-2 text-xs font-bold text-white rounded hover:bg-red-500 transition-colors"
                      >
                        CREATE OPERATION
                      </button>
                      <button
                        onClick={() => setShowCreateCampaign(false)}
                        className="text-slate-500 text-xs hover:text-white px-4 py-2"
                      >
                        CANCEL
                      </button>
                    </div>
                  </div>
                </div>
              )}

              <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
                    <tr>
                      <th className="px-4 py-3 font-medium">NAME</th>
                      <th className="px-4 py-3 font-medium">STATUS</th>
                      <th className="px-4 py-3 font-medium">IMPLANTS</th>
                      <th className="px-4 py-3 font-medium text-right">ACTIONS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {campaigns.map((c) => (
                      <tr
                        key={c.id}
                        className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors cursor-pointer"
                        onClick={() => setSelectedCampaignId(c.id)}
                      >
                        <td className="px-4 py-3 font-bold text-slate-300">{c.name}</td>
                        <td className="px-4 py-3">
                          <span className="px-2 py-0.5 bg-green-900/20 text-green-500 rounded border border-green-900/30">
                            {c.status.toUpperCase()}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-slate-400">{c.implant_count}</td>
                        <td className="px-4 py-3 text-right">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedCampaignId(c.id);
                            }}
                            className="text-blue-400 hover:text-white bg-blue-900/20 hover:bg-blue-600 px-3 py-1 rounded transition-all text-xs"
                          >
                            DETAILS
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Beacons */}
          {activeTab === 'beacons' && (
            <div>
              <div className="mb-4 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <h2 className="text-sm font-bold text-white uppercase tracking-wider">
                    ACTIVE BEACONS
                  </h2>
                  {selectedImplants.size > 0 && (
                    <div className="flex items-center gap-2 px-3 py-1 bg-red-900/20 border border-red-900/30 rounded">
                      <span className="text-[10px] font-bold text-red-500">
                        {selectedImplants.size} SELECTED
                      </span>
                      <button
                        onClick={handleBulkKill}
                        className="text-[10px] bg-red-600 hover:bg-red-500 text-white px-2 py-0.5 rounded font-bold"
                      >
                        KILL ALL
                      </button>
                      <button
                        onClick={() => setSelectedImplants(new Set())}
                        className="text-[10px] text-slate-500 hover:text-white"
                      >
                        DESELECT
                      </button>
                    </div>
                  )}
                </div>
                <button
                  onClick={refreshImplants}
                  className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700 transition-colors"
                >
                  REFRESH
                </button>
              </div>

              {selectedImplantId && (
                <div className="mb-4">
                  <ImplantDetailPanel
                    implantId={selectedImplantId}
                    onClose={() => setSelectedImplantId(null)}
                  />
                </div>
              )}

              <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
                    <tr>
                      <th className="px-4 py-3 w-10">
                        <input
                          type="checkbox"
                          onChange={(e) => {
                            if (e.target.checked)
                              setSelectedImplants(new Set(implants.map((i) => i.id)));
                            else setSelectedImplants(new Set());
                          }}
                          checked={
                            selectedImplants.size === implants.length && implants.length > 0
                          }
                        />
                      </th>
                      <th className="px-4 py-3 font-medium">ID</th>
                      <th className="px-4 py-3 font-medium">HOSTNAME</th>
                      <th className="px-4 py-3 font-medium">IP ADDRESS</th>
                      <th className="px-4 py-3 font-medium">USER</th>
                      <th className="px-4 py-3 font-medium">STATUS</th>
                      <th className="px-4 py-3 font-medium text-right">ACTIONS</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-300">
                    {implants.length === 0 ? (
                      <tr>
                        <td className="px-4 py-12 text-center text-slate-600 italic" colSpan={7}>
                          No signals detected. Waiting for check-in...
                        </td>
                      </tr>
                    ) : (
                      implants.map((imp) => (
                        <tr
                          key={imp.id}
                          className={`border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors group ${
                            selectedImplants.has(imp.id) ? 'bg-red-950/5' : ''
                          }`}
                          onContextMenu={(e) =>
                            handleBeaconContextMenu(e, {
                              id: imp.id,
                              hostname: imp.hostname,
                            })
                          }
                        >
                          <td className="px-4 py-3">
                            <input
                              type="checkbox"
                              checked={selectedImplants.has(imp.id)}
                              onChange={() => toggleImplantSelection(imp.id)}
                            />
                          </td>
                          <td className="px-4 py-3 font-mono text-slate-500">
                            {imp.id.substring(0, 8)}...
                          </td>
                          <td className="px-4 py-3 font-bold text-slate-200">{imp.hostname}</td>
                          <td className="px-4 py-3 font-mono text-slate-400">
                            {imp.internal_ip}
                          </td>
                          <td className="px-4 py-3 text-slate-400">
                            {imp.username ? `${imp.domain}\\${imp.username}` : '-'}
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-green-500 font-bold flex items-center gap-1">
                              <span className="w-1.5 h-1.5 rounded-full bg-green-500" />{' '}
                              {imp.status.toUpperCase()}
                            </span>
                          </td>
                          <td className="px-4 py-3 text-right">
                            <div className="flex gap-1 justify-end">
                              <button
                                onClick={() => setSelectedImplantId(imp.id)}
                                className="text-slate-400 hover:text-white bg-slate-800 hover:bg-slate-700 px-2 py-1 rounded transition-all opacity-0 group-hover:opacity-100 text-xs"
                              >
                                DETAILS
                              </button>
                              <button
                                onClick={() => {
                                  setInteractingImplantId(imp.id);
                                  setActiveTab('console');
                                }}
                                className="text-blue-400 hover:text-white bg-blue-900/20 hover:bg-blue-600 px-3 py-1 rounded transition-all opacity-0 group-hover:opacity-100"
                              >
                                INTERACT
                              </button>
                            </div>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Console / Beacon Interaction */}
          {activeTab === 'console' && interactingImplantId && (
            <BeaconInteraction
              implantId={interactingImplantId}
              onBack={() => {
                setInteractingImplantId(null);
                setActiveTab('beacons');
              }}
            />
          )}

          {/* Listeners */}
          {activeTab === 'listeners' && <ListenerManager />}

          {/* Loot */}
          {activeTab === 'loot' && <LootGallery />}

          {/* Phishing */}
          {activeTab === 'phishing' && <PhishingBuilder />}

          {/* Generator */}
          {activeTab === 'generator' && <ImplantGenerator />}

          {/* Playbooks */}
          {activeTab === 'playbooks' && <PlaybookBrowser />}

          {/* Events */}
          {activeTab === 'events' && <EventLog />}
        </main>
      </div>
    </div>
  );
}

export default App;
