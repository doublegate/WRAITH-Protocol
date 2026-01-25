import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Console } from './components/Console'
import { NetworkGraph } from './components/NetworkGraph'

interface Implant {
  id: string;
  hostname: string;
  internal_ip: string;
  last_checkin: string;
  status: string;
}

interface Campaign {
    id: string;
    name: string;
    status: string;
    implant_count: number;
}

interface Listener {
    id: string;
    name: string;
    type_: string;
    bind_address: string;
    port: number;
    status: string;
}

interface Artifact {
    id: string;
    filename: string;
    size: number;
}

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [serverStatus, setServerStatus] = useState('Disconnected')
  const [implants, setImplants] = useState<Implant[]>([])
  const [campaigns, setCampaigns] = useState<Campaign[]>([])
  const [listeners, setListeners] = useState<Listener[]>([])
  const [artifacts, setArtifacts] = useState<Artifact[]>([])
  const [interactingImplantId, setInteractingImplantId] = useState<string | null>(null)
  const [showCreateCampaign, setShowCreateCampaign] = useState(false)
  const [newCampaignName, setNewCampaignName] = useState('')
  const [newCampaignDesc, setNewCampaignDesc] = useState('')

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
  
  const connect = async () => {
    try {
      setServerStatus('Connecting...')
      await invoke('connect_to_server', { address: '127.0.0.1:50051' })
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
      refreshArtifacts()
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

  const refreshArtifacts = async () => {
    try {
      const json = await invoke('list_artifacts') as string
      setArtifacts(JSON.parse(json))
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
      <header className="flex h-14 items-center justify-between border-b border-slate-800 bg-slate-900 px-4">
        <div className="flex items-center gap-2">
          <div className="h-4 w-4 bg-red-600"></div>
          <span className="text-lg font-bold tracking-tight text-red-500">WRAITH::REDOPS</span>
        </div>
        <div className="flex items-center gap-4 text-xs">
          <span className={serverStatus === 'Connected' ? 'text-green-500' : 'text-red-500'}>
            [{serverStatus}]
          </span>
          <span className="text-slate-400">OP: ADMIN</span>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-48 border-r border-slate-800 bg-slate-950 p-2 text-xs">
          <div className="mb-2 px-2 text-slate-500 uppercase">Operations</div>
          <ul className="space-y-1">
            {['Dashboard', 'Campaigns', 'Beacons', 'Listeners', 'Artifacts'].map(tab => (
              <li key={tab}>
                <button 
                  onClick={() => setActiveTab(tab.toLowerCase())}
                  className={`w-full rounded px-2 py-1.5 text-left ${activeTab === tab.toLowerCase() ? 'bg-slate-800 text-white' : 'text-slate-400 hover:text-white'}`}
                >
                  {tab}
                </button>
              </li>
            ))}
          </ul>
        </nav>

        {/* Content Area */}
        <main className="flex-1 overflow-auto bg-slate-950 p-4">
          
          {/* Dashboard - Placeholder */}
          {activeTab === 'dashboard' && (
             <div className="flex flex-col h-full gap-4">
                <div className="grid grid-cols-3 gap-4">
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-xs text-slate-500">ACTIVE CAMPAIGNS</div>
                        <div className="text-2xl font-bold text-white">{campaigns.length}</div>
                    </div>
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-xs text-slate-500">LIVE BEACONS</div>
                        <div className="text-2xl font-bold text-green-500">{implants.filter(i => i.status === 'active').length}</div>
                    </div>
                    <div className="rounded border border-slate-800 bg-slate-900 p-4">
                        <div className="text-xs text-slate-500">ARTIFACTS LOOTED</div>
                        <div className="text-2xl font-bold text-yellow-500">{artifacts.length}</div>
                    </div>
                </div>
                <div className="flex-1 min-h-0">
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
                        className="rounded bg-red-600 px-3 py-1 text-xs text-white hover:bg-red-500 font-bold"
                    >
                        NEW CAMPAIGN
                    </button>
                    <button onClick={refreshCampaigns} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700">REFRESH</button>
                </div>
              </div>

              {showCreateCampaign && (
                  <div className="mb-6 p-4 rounded border border-red-900/50 bg-red-950/10 border-dashed">
                      <h3 className="text-xs font-bold text-red-500 mb-3 uppercase tracking-widest">Initialization Wizard</h3>
                      <div className="grid gap-3">
                          <input 
                            placeholder="Campaign Name" 
                            value={newCampaignName}
                            onChange={(e) => setNewCampaignName(e.target.value)}
                            className="bg-slate-900 border border-slate-800 p-2 text-xs text-white focus:outline-none focus:border-red-500"
                          />
                          <textarea 
                            placeholder="Objective / Description" 
                            value={newCampaignDesc}
                            onChange={(e) => setNewCampaignDesc(e.target.value)}
                            className="bg-slate-900 border border-slate-800 p-2 text-xs text-white focus:outline-none focus:border-red-500 min-h-[80px]"
                          />
                          <div className="flex gap-2">
                              <button onClick={handleCreateCampaign} className="bg-red-600 px-4 py-1 text-xs font-bold text-white">CREATE</button>
                              <button onClick={() => setShowCreateCampaign(false)} className="text-slate-500 text-xs hover:text-white">CANCEL</button>
                          </div>
                      </div>
                  </div>
              )}

              <div className="border border-slate-800 bg-slate-900">
                 <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-900 text-slate-500">
                    <tr><th className="px-4 py-2">NAME</th><th className="px-4 py-2">STATUS</th><th className="px-4 py-2">IMPLANTS</th></tr>
                  </thead>
                  <tbody>
                    {campaigns.map(c => (
                        <tr key={c.id} className="border-b border-slate-800/50 hover:bg-slate-800/50">
                            <td className="px-4 py-2">{c.name}</td>
                            <td className="px-4 py-2">{c.status}</td>
                            <td className="px-4 py-2">{c.implant_count}</td>
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
                <h2 className="text-sm font-bold text-white">ACTIVE BEACONS</h2>
                <button onClick={refreshImplants} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700">Refresh</button>
              </div>
              <div className="border border-slate-800 bg-slate-900">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-900 text-slate-500">
                    <tr>
                      <th className="px-4 py-2 font-medium">ID</th>
                      <th className="px-4 py-2 font-medium">HOSTNAME</th>
                      <th className="px-4 py-2 font-medium">IP</th>
                      <th className="px-4 py-2 font-medium">LAST SEEN</th>
                      <th className="px-4 py-2 font-medium">STATUS</th>
                      <th className="px-4 py-2 font-medium">ACTIONS</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-300">
                    {implants.length === 0 ? (
                      <tr>
                        <td className="px-4 py-8 text-center text-slate-500" colSpan={6}>No signals detected.</td>
                      </tr>
                    ) : (
                      implants.map(imp => (
                        <tr key={imp.id} className="border-b border-slate-800/50 hover:bg-slate-800/50">
                          <td className="px-4 py-2 font-mono text-slate-500">{imp.id.substring(0,8)}...</td>
                          <td className="px-4 py-2">{imp.hostname}</td>
                          <td className="px-4 py-2">{imp.internal_ip}</td>
                          <td className="px-4 py-2">{imp.last_checkin || 'Never'}</td>
                          <td className="px-4 py-2 text-green-500">{imp.status}</td>
                          <td className="px-4 py-2">
                            <button 
                              onClick={() => {
                                setInteractingImplantId(imp.id);
                                setActiveTab('console');
                              }}
                              className="text-blue-400 hover:text-blue-300 mr-2"
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

          {/* Console */}
          {activeTab === 'console' && interactingImplantId && (
            <div className="flex flex-col h-full gap-4">
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-bold text-white uppercase tracking-wider">BEACON INTERACTION / {interactingImplantId.substring(0,8)}</h2>
                <button 
                  onClick={() => {
                    setInteractingImplantId(null);
                    setActiveTab('beacons');
                  }}
                  className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700"
                >
                  BACK TO LIST
                </button>
              </div>
              <div className="flex-1 min-h-0">
                <Console implantId={interactingImplantId} />
              </div>
            </div>
          )}

          {/* Listeners */}
          {activeTab === 'listeners' && (
            <div>
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-sm font-bold text-white">C2 LISTENERS</h2>
                <button onClick={refreshListeners} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700">Refresh</button>
              </div>
              <div className="border border-slate-800 bg-slate-900">
                 <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-900 text-slate-500">
                    <tr><th className="px-4 py-2">NAME</th><th className="px-4 py-2">TYPE</th><th className="px-4 py-2">BIND</th><th className="px-4 py-2">STATUS</th></tr>
                  </thead>
                  <tbody>
                    {listeners.map(l => (
                        <tr key={l.id} className="border-b border-slate-800/50 hover:bg-slate-800/50">
                            <td className="px-4 py-2">{l.name}</td>
                            <td className="px-4 py-2">{l.type_}</td>
                            <td className="px-4 py-2">{l.bind_address}</td>
                            <td className="px-4 py-2">{l.status}</td>
                        </tr>
                    ))}
                  </tbody>
                 </table>
              </div>
            </div>
          )}

          {/* Artifacts */}
          {activeTab === 'artifacts' && (
            <div>
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-sm font-bold text-white">LOOT / ARTIFACTS</h2>
                <button onClick={refreshArtifacts} className="rounded bg-slate-800 px-3 py-1 text-xs text-white hover:bg-slate-700">Refresh</button>
              </div>
              <div className="border border-slate-800 bg-slate-900">
                 <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-900 text-slate-500">
                    <tr><th className="px-4 py-2">FILENAME</th><th className="px-4 py-2">SIZE</th><th className="px-4 py-2">ACTIONS</th></tr>
                  </thead>
                  <tbody>
                    {artifacts.map(a => (
                        <tr key={a.id} className="border-b border-slate-800/50 hover:bg-slate-800/50">
                            <td className="px-4 py-2">{a.filename}</td>
                            <td className="px-4 py-2">{a.size} B</td>
                            <td className="px-4 py-2"><button className="text-blue-400 hover:text-blue-300">DOWNLOAD</button></td>
                        </tr>
                    ))}
                  </tbody>
                 </table>
              </div>
            </div>
          )}

        </main>
      </div>
    </div>
  )
}

export default App
