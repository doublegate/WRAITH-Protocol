import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

interface Implant {
  id: string;
  hostname: string;
  internal_ip: string;
  last_checkin: string;
  status: string;
}

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [serverStatus, setServerStatus] = useState('Disconnected')
  const [implants, setImplants] = useState<Implant[]>([])
  
  const connect = async () => {
    try {
      setServerStatus('Connecting...')
      await invoke('connect_to_server', { address: '127.0.0.1:50051' })
      setServerStatus('Connected')
      refreshImplants()
    } catch (e) {
      setServerStatus('Error: ' + e)
    }
  }

  const refreshImplants = async () => {
    try {
      const json = await invoke('list_implants') as string
      // Note: In real app we parse this properly, here we trust the backend
      // But since backend returns raw proto structs converted to JSON string via default serde,
      // it should match if we annotated proto with serde, or we construct manual JSON in rust.
      // For MVP, assuming the rust side returned a valid JSON array string.
      setImplants(JSON.parse(json))
    } catch (e) {
      console.error(e)
    }
  }

  useEffect(() => {
    connect()
    const interval = setInterval(refreshImplants, 5000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="flex h-screen w-full flex-col font-mono">
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
            {['Dashboard', 'Beacons', 'Listeners', 'Artifacts'].map(tab => (
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
                            <button className="text-blue-400 hover:text-blue-300 mr-2">INTERACT</button>
                          </td>
                        </tr>
                      ))
                    )}
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