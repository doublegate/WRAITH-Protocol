import { useState } from 'react';
import { Button } from './ui/Button';
import { ConfirmDialog } from './ui/ConfirmDialog';
import { useToastStore } from '../stores/toastStore';
import { useAppStore } from '../stores/appStore';
import * as ipc from '../lib/ipc';
import { Play, Square, Plus, X } from 'lucide-react';

export default function ListenerManager() {
  const { listeners, refreshListeners } = useAppStore();
  const addToast = useToastStore((s) => s.addToast);

  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState('');
  const [type_, setType] = useState('http');
  const [bindAddress, setBindAddress] = useState('0.0.0.0');
  const [port, setPort] = useState(8080);

  const [confirmAction, setConfirmAction] = useState<{
    open: boolean;
    title: string;
    message: string;
    action: () => void;
  }>({ open: false, title: '', message: '', action: () => {} });

  const handleCreate = async () => {
    if (!name.trim()) {
      addToast('warning', 'Listener name is required');
      return;
    }
    try {
      await ipc.createListener(name, type_, bindAddress, port);
      addToast('success', `Listener "${name}" created`);
      setShowCreate(false);
      setName('');
      refreshListeners();
    } catch (e) {
      addToast('error', 'Failed to create listener: ' + e);
    }
  };

  const handleStart = (id: string, lName: string) => {
    setConfirmAction({
      open: true,
      title: 'Start Listener',
      message: `Start listener "${lName}"?`,
      action: async () => {
        try {
          await ipc.startListener(id);
          addToast('success', `Listener "${lName}" started`);
          refreshListeners();
        } catch (e) {
          addToast('error', 'Failed to start listener: ' + e);
        }
      },
    });
  };

  const handleStop = (id: string, lName: string) => {
    setConfirmAction({
      open: true,
      title: 'Stop Listener',
      message: `Stop listener "${lName}"? Active connections will be dropped.`,
      action: async () => {
        try {
          await ipc.stopListener(id);
          addToast('success', `Listener "${lName}" stopped`);
          refreshListeners();
        } catch (e) {
          addToast('error', 'Failed to stop listener: ' + e);
        }
      },
    });
  };

  return (
    <div>
      <ConfirmDialog
        open={confirmAction.open}
        title={confirmAction.title}
        message={confirmAction.message}
        onConfirm={() => {
          confirmAction.action();
          setConfirmAction((s) => ({ ...s, open: false }));
        }}
        onCancel={() => setConfirmAction((s) => ({ ...s, open: false }))}
        variant="primary"
      />

      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-bold text-white uppercase tracking-wider">C2 LISTENERS</h2>
        <div className="flex gap-2">
          <Button size="sm" onClick={() => setShowCreate(true)}>
            <span className="flex items-center gap-1">
              <Plus className="w-3 h-3" /> NEW LISTENER
            </span>
          </Button>
          <Button variant="secondary" size="sm" onClick={refreshListeners}>
            REFRESH
          </Button>
        </div>
      </div>

      {showCreate && (
        <div className="mb-6 p-6 rounded border border-red-900/50 bg-red-950/10 border-dashed">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-xs font-bold text-red-500 uppercase tracking-widest">
              New Listener
            </h3>
            <button onClick={() => setShowCreate(false)} className="text-slate-500 hover:text-white">
              <X className="w-4 h-4" />
            </button>
          </div>
          <div className="grid gap-4 max-w-lg">
            <div>
              <label className="text-[10px] text-slate-500 uppercase mb-1 block">Name</label>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="HTTP-Primary"
                className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
              />
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div>
                <label className="text-[10px] text-slate-500 uppercase mb-1 block">Protocol</label>
                <select
                  value={type_}
                  onChange={(e) => setType(e.target.value)}
                  className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                >
                  <option value="http">HTTP</option>
                  <option value="https">HTTPS</option>
                  <option value="dns">DNS</option>
                  <option value="smb">SMB</option>
                </select>
              </div>
              <div>
                <label className="text-[10px] text-slate-500 uppercase mb-1 block">Bind Address</label>
                <input
                  value={bindAddress}
                  onChange={(e) => setBindAddress(e.target.value)}
                  className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                />
              </div>
              <div>
                <label className="text-[10px] text-slate-500 uppercase mb-1 block">Port</label>
                <input
                  type="number"
                  value={port}
                  onChange={(e) => setPort(parseInt(e.target.value) || 0)}
                  className="w-full bg-slate-900 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
                />
              </div>
            </div>
            <div className="flex gap-2 mt-2">
              <Button size="sm" onClick={handleCreate}>
                CREATE
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setShowCreate(false)}>
                CANCEL
              </Button>
            </div>
          </div>
        </div>
      )}

      <div className="border border-slate-800 bg-slate-900 rounded overflow-hidden shadow">
        <table className="w-full text-left text-xs">
          <thead className="border-b border-slate-800 bg-slate-950 text-slate-500">
            <tr>
              <th className="px-4 py-3 font-medium">NAME</th>
              <th className="px-4 py-3 font-medium">PROTOCOL</th>
              <th className="px-4 py-3 font-medium">BIND ADDRESS</th>
              <th className="px-4 py-3 font-medium">STATUS</th>
              <th className="px-4 py-3 font-medium text-right">ACTIONS</th>
            </tr>
          </thead>
          <tbody>
            {listeners.length === 0 ? (
              <tr>
                <td className="px-4 py-12 text-center text-slate-600 italic" colSpan={5}>
                  No listeners configured. Create one to start receiving connections.
                </td>
              </tr>
            ) : (
              listeners.map((l) => (
                <tr
                  key={l.id}
                  className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors"
                >
                  <td className="px-4 py-3 font-bold text-slate-300">{l.name}</td>
                  <td className="px-4 py-3 uppercase text-slate-400">{l.type_}</td>
                  <td className="px-4 py-3 font-mono text-slate-400">
                    {l.bind_address}:{l.port}
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className={`px-2 py-0.5 rounded border ${
                        l.status === 'active'
                          ? 'bg-blue-900/20 text-blue-500 border-blue-900/30'
                          : 'bg-slate-800 text-slate-500 border-slate-700'
                      }`}
                    >
                      {l.status.toUpperCase()}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <div className="flex gap-1 justify-end">
                      {l.status !== 'active' ? (
                        <button
                          onClick={() => handleStart(l.id, l.name)}
                          className="text-green-400 hover:text-white bg-green-900/20 hover:bg-green-600 px-2 py-1 rounded transition-all"
                          title="Start"
                        >
                          <Play className="w-3 h-3" />
                        </button>
                      ) : (
                        <button
                          onClick={() => handleStop(l.id, l.name)}
                          className="text-red-400 hover:text-white bg-red-900/20 hover:bg-red-600 px-2 py-1 rounded transition-all"
                          title="Stop"
                        >
                          <Square className="w-3 h-3" />
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
