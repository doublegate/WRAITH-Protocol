import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Console } from './components/Console'
import { NetworkGraph } from './components/NetworkGraph'
import BeaconInteraction from './components/BeaconInteraction'
import PhishingBuilder from './components/PhishingBuilder'
import LootGallery from './components/LootGallery'

interface Implant {
  id: string;
// ... (interfaces same) ...

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [serverStatus, setServerStatus] = useState('Disconnected')
  const [implants, setImplants] = useState<Implant[]>([])
  const [campaigns, setCampaigns] = useState<Campaign[]>([])
  const [listeners, setListeners] = useState<Listener[]>([])
  // artifacts state moved to LootGallery
  const [interactingImplantId, setInteractingImplantId] = useState<string | null>(null)
  const [showCreateCampaign, setShowCreateCampaign] = useState(false)
  const [newCampaignName, setNewCampaignName] = useState('')
  const [newCampaignDesc, setNewCampaignDesc] = useState('')
  const [serverAddress, setServerAddress] = useState('127.0.0.1:50051')

  const handleCreateCampaign = async () => {
    try {
      await invoke('create_campaign', { name: newCampaignName, description: newCampaignDesc });
      setShowCreateCampaign(false);
      setNewCampaignName('');
      setNewCampaignDesc('');
      refreshCampaigns();
    } catch (e) {
      console.error(e);
    }
  }
  
  const connect = async (addr = serverAddress) => {
    try {
      setServerStatus('Connecting...')
      await invoke('connect_to_server', { address: addr })
      setServerStatus('Connected')
      refreshAll()
    } catch (e) {
      setServerStatus('Error: ' + e)
    }
  }

  const refreshAll = () => {
      refreshImplants()
      refreshCampaigns()
      refreshListeners()
  }

  const refreshImplants = async () => {
    try {
      const json = await invoke('list_implants') as string
      setImplants(JSON.parse(json))
    } catch (e) { console.error(e) }
  }

  const refreshCampaigns = async () => {
    try {
      const json = await invoke('list_campaigns') as string
      setCampaigns(JSON.parse(json))
    } catch (e) { console.error(e) }
  }

  const refreshListeners = async () => {
    try {
      const json = await invoke('list_listeners') as string
      setListeners(JSON.parse(json))
    } catch (e) { console.error(e) }
  }

  useEffect(() => {
    connect()
    const interval = setInterval(refreshAll, 5000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="flex h-screen w-full flex-col font-mono text-slate-300">
      {/* Header */}
      <header className="flex h-14 items-center justify-between border-b border-slate-800 bg-slate-900 px-4 shadow-sm z-10">
        <div className="flex items-center gap-2">
          <div className="h-4 w-4 bg-red-600 shadow-[0_0_10px_rgba(220,38,38,0.5)]"></div>
          <span className="text-lg font-bold tracking-tight text-red-500">WRAITH::REDOPS</span>
        </div>
        <div className="flex items-center gap-4 text-xs font-bold">
          <span className={`px-2 py-0.5 rounded ${serverStatus === 'Connected' ? 'bg-green-900/30 text-green-500 border border-green-900/50' : 'bg-red-900/30 text-red-500 border border-red-900/50'}`}>
            {serverStatus.toUpperCase()}
          </span>
          <span className="text-slate-500">OP: ADMIN</span>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-48 border-r border-slate-800 bg-slate-950 p-2 text-xs flex flex-col gap-1">
          <div className="mt-2 mb-1 px-2 text-slate-600 font-bold uppercase text-[10px] tracking-wider">Operations</div>
          {['Dashboard', 'Campaigns', 'Beacons', 'Listeners', 'Loot', 'Phishing'].map(tab => (
            <button 
              key={tab}
              onClick={() => setActiveTab(tab.toLowerCase())}
              className={`w-full rounded px-3 py-2 text-left transition-colors flex items-center gap-2 ${
                activeTab === tab.toLowerCase() 
                  ? 'bg-red-600/10 text-red-500 border border-red-900/20' 
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
              }`}
            >
              {tab === 'Dashboard' && '📊'}
              {tab === 'Campaigns' && '📁'}
              {tab === 'Beacons' && '📡'}
              {tab === 'Listeners' && '👂'}
              {tab === 'Loot' && '💰'}
              {tab === 'Phishing' && '🎣'}
              {tab}
            </button>
          ))}
          
          <div className="mt-4 mb-1 px-2 text-slate-600 font-bold uppercase text-[10px] tracking-wider">System</div>
          <button 
            onClick={() => setActiveTab('settings')}
            className={`w-full rounded px-3 py-2 text-left transition-colors flex items-center gap-2 ${
              activeTab === 'settings' 
                ? 'bg-red-600/10 text-red-500 border border-red-900/20' 
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
            }`}
          >
            ⚙️ Settings
          </button>
        </nav>

        {/* Content Area */}
        <main className="flex-1 overflow-auto bg-slate-950 p-4 relative">
          
          {/* Settings */}
          {activeTab === 'settings' && (
            <div className="max-w-md mx-auto mt-10">
                <div className="rounded border border-slate-800 bg-slate-900 p-6 shadow-lg">
                    <h3 className="text-sm font-bold text-white mb-6 border-b border-slate-800 pb-2">CONNECTION SETTINGS</h3>
                    <div className="grid gap-4">
                        <div>
                            <label className="block text-xs text-slate-500 mb-1">TEAM SERVER ADDRESS</label>
                            <input 
                                value={serverAddress}
                                onChange={(e) => setServerAddress(e.target.value)}
                                className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                            />
                        </div>
                        <button onClick={() => connect(serverAddress)} className="bg-red-600 text-white px-4 py-2 text-xs font-bold rounded hover:bg-red-500 transition-colors">
                            RECONNECT
                        </button>
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
                        <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">Active Campaigns</div>
                        <div className="text-3xl font-bold text-white mt-2">{campaigns.length}</div>
                        <div className="text-[10px] text-slate-600 mt-1">
                          {campaigns.filter(c => c.status === 'active').length} running
                        </div>
                    </div>
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">Live Beacons</div>
                        <div className="text-3xl font-bold text-green-500 mt-2">
                          {implants.filter(i => i.status === 'active').length}
                        </div>
                        <div className="text-[10px] text-slate-600 mt-1">
                          {implants.length} total registered
                        </div>
                    </div>
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">Active Listeners</div>
                        <div className="text-3xl font-bold text-blue-500 mt-2">
                          {listeners.filter(l => l.status === 'active').length}
                        </div>
                        <div className="text-[10px] text-slate-600 mt-1">
                          {listeners.length} configured
                        </div>
                    </div>
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-[10px] text-slate-500 uppercase tracking-wider font-bold">System Status</div>
                        <div className="text-xl font-bold text-slate-300 mt-3 truncate">
                          {serverStatus === 'Connected' ? 'OPERATIONAL' : 'OFFLINE'}
                        </div>
                        <div className="text-[10px] text-slate-600 mt-1">
                          {serverAddress}
                        </div>
                    </div>
                </div>

                {/* Network Graph */}
                <div className="flex-1 min-h-0 rounded border border-slate-800 bg-slate-900 overflow-hidden relative">
                    <div className="absolute top-2 left-2 z-10 text-[10px] text-slate-500 font-bold uppercase tracking-wider">Operational Graph</div>
                    <NetworkGraph implants={implants} />
                </div>
             </div>
          )}

          {/* Campaigns */}
          {activeTab === 'campaigns' && (
            <div className="flex flex-col h-full">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-sm font-bold text-white uppercase tracking-wider">OPERATIONS / CAMPAIGNS</h2>
                <div className="flex gap-2">
                    <button 
                        onClick={() => setShowCreateCampaign(true)}
                        className="rounded bg-red-600 px-3 py-1 text-xs text-white hover:bg-red-500 font-bold shadow-lg shadow-red-900/20 transition-all"
                    >
                        NEW CAMPAIGN
                    </button>
                    <button onClick={refreshCampaigns} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700 transition-colors">REFRESH</button>
                </div>
              </div>

              {showCreateCampaign && (
                  <div className="mb-6 p-6 rounded border border-red-900/50 bg-red-950/10 border-dashed animate-in fade-in slide-in-from-top-2">
                      <h3 className="text-xs font-bold text-red-500 mb-4 uppercase tracking-widest">Initialization Wizard</h3>
                      <div className="grid gap-4 max-w-lg">
                          <div>
                            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Codename</label>
                            <input 
                                placeholder="OP_GHOST" 
                                value={newCampaignName}
                                onChange={(e) => setNewCampaignName(e.target.value)}
                                className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                            />
                          </div>
                          <div>
                            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Mission Objective</label>
                            <textarea 
                                placeholder="Describe the operational goals..." 
                                value={newCampaignDesc}
                                onChange={(e) => setNewCampaignDesc(e.target.value)}
                                className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 min-h-[80px] rounded"
                            />
                          </div>
                          <div className="flex gap-2 mt-2">
                              <button onClick={handleCreateCampaign} className="bg-red-600 px-6 py-2 text-xs font-bold text-white rounded hover:bg-red-500 transition-colors">CREATE OPERATION</button>
                              <button onClick={() => setShowCreateCampaign(false)} className="text-slate-500 text-xs hover:text-white px-4 py-2">CANCEL</button>
                          </div>
                      </div>
                  </div>
              )}

              <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
                 <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
                    <tr><th className="px-4 py-3 font-medium">NAME</th><th className="px-4 py-3 font-medium">STATUS</th><th className="px-4 py-3 font-medium">IMPLANTS</th></tr>
                  </thead>
                  <tbody>
                    {campaigns.map(c => (
                        <tr key={c.id} className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors">
                            <td className="px-4 py-3 font-bold text-slate-300">{c.name}</td>
                            <td className="px-4 py-3"><span className="px-2 py-0.5 bg-green-900/20 text-green-500 rounded border border-green-900/30">{c.status.toUpperCase()}</span></td>
                            <td className="px-4 py-3 text-slate-400">{c.implant_count}</td>
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
                <h2 className="text-sm font-bold text-white uppercase tracking-wider">ACTIVE BEACONS</h2>
                <button onClick={refreshImplants} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700 transition-colors">REFRESH</button>
              </div>
              <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
                    <tr>
                      <th className="px-4 py-3 font-medium">ID</th>
                      <th className="px-4 py-3 font-medium">HOSTNAME</th>
                      <th className="px-4 py-3 font-medium">IP ADDRESS</th>
                      <th className="px-4 py-3 font-medium">LAST SEEN</th>
                      <th className="px-4 py-3 font-medium">STATUS</th>
                      <th className="px-4 py-3 font-medium text-right">ACTIONS</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-300">
                    {implants.length === 0 ? (
                      <tr>
                        <td className="px-4 py-12 text-center text-slate-600 italic" colSpan={6}>No signals detected. Waiting for check-in...</td>
                      </tr>
                    ) : (
                      implants.map(imp => (
                        <tr key={imp.id} className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors group">
                          <td className="px-4 py-3 font-mono text-slate-500">{imp.id.substring(0,8)}...</td>
                          <td className="px-4 py-3 font-bold text-slate-200">{imp.hostname}</td>
                          <td className="px-4 py-3 font-mono text-slate-400">{imp.internal_ip}</td>
                          <td className="px-4 py-3 text-slate-400">{imp.last_checkin || 'Never'}</td>
                          <td className="px-4 py-3"><span className="text-green-500 font-bold flex items-center gap-1"><span className="w-1.5 h-1.5 rounded-full bg-green-500"></span> {imp.status.toUpperCase()}</span></td>
                          <td className="px-4 py-3 text-right">
                            <button 
                              onClick={() => {
                                setInteractingImplantId(imp.id);
                                setActiveTab('console');
                              }}
                              className="text-blue-400 hover:text-white bg-blue-900/20 hover:bg-blue-600 px-3 py-1 rounded transition-all opacity-0 group-hover:opacity-100"
                            >
                              INTERACT
                            </button>
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
          {activeTab === 'listeners' && (
            <div>
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-sm font-bold text-white uppercase tracking-wider">C2 LISTENERS</h2>
                <button onClick={refreshListeners} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700 transition-colors">REFRESH</button>
              </div>
              <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
                 <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
                    <tr><th className="px-4 py-3 font-medium">NAME</th><th className="px-4 py-3 font-medium">PROTOCOL</th><th className="px-4 py-3 font-medium">BIND ADDRESS</th><th className="px-4 py-3 font-medium">STATUS</th></tr>
                  </thead>
                  <tbody>
                    {listeners.map(l => (
                        <tr key={l.id} className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors">
                            <td className="px-4 py-3 font-bold text-slate-300">{l.name}</td>
                            <td className="px-4 py-3 uppercase text-slate-400">{l.type_}</td>
                            <td className="px-4 py-3 font-mono text-slate-400">{l.bind_address}:{l.port}</td>
                            <td className="px-4 py-3">
                                <span className={`px-2 py-0.5 rounded border ${l.status === 'active' ? 'bg-blue-900/20 text-blue-500 border-blue-900/30' : 'bg-slate-800 text-slate-500 border-slate-700'}`}>
                                    {l.status.toUpperCase()}
                                </span>
                            </td>
                        </tr>
                    ))}
                  </tbody>
                 </table>
              </div>
            </div>
          )}

          {/* Loot */}
          {activeTab === 'loot' && <LootGallery />}

          {/* Phishing */}
          {activeTab === 'phishing' && <PhishingBuilder />}

        </main>
      </div>
    </div>
  )
}

export default App
