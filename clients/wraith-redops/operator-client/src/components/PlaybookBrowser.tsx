import { useState, useEffect } from 'react';
import { Button } from './ui/Button';
import { useToastStore } from '../stores/toastStore';
import * as ipc from '../lib/ipc';
import type { Playbook, AttackChain } from '../types';
import { BookOpen, Play, ChevronRight, List } from 'lucide-react';

export default function PlaybookBrowser() {
  const addToast = useToastStore((s) => s.addToast);

  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [chains, setChains] = useState<AttackChain[]>([]);
  const [activeView, setActiveView] = useState<'playbooks' | 'chains'>('playbooks');
  const [selectedPlaybook, setSelectedPlaybook] = useState<Playbook | null>(null);
  const [selectedChain, setSelectedChain] = useState<AttackChain | null>(null);
  const [loading, setLoading] = useState(false);

  const loadPlaybooks = async () => {
    setLoading(true);
    try {
      const data = await ipc.listPlaybooks();
      setPlaybooks(data);
    } catch (e) {
      addToast('error', 'Failed to load playbooks: ' + e);
    } finally {
      setLoading(false);
    }
  };

  const loadChains = async () => {
    setLoading(true);
    try {
      const data = await ipc.listAttackChains();
      setChains(data);
    } catch (e) {
      addToast('error', 'Failed to load attack chains: ' + e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlaybooks();
    loadChains();
  }, []);

  const handleInstantiate = async (playbookId: string, playbookName: string) => {
    try {
      const chain = await ipc.instantiatePlaybook(playbookId, '');
      addToast('success', `Playbook "${playbookName}" instantiated as chain "${chain.name}"`);
      loadChains();
    } catch (e) {
      addToast('error', 'Failed to instantiate playbook: ' + e);
    }
  };

  const handleViewChain = async (chainId: string) => {
    try {
      const chain = await ipc.getAttackChain(chainId);
      setSelectedChain(chain);
    } catch (e) {
      addToast('error', 'Failed to load chain details: ' + e);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-bold text-white uppercase tracking-wider">
          Playbooks & Chains
        </h2>
        <div className="flex gap-1 bg-slate-900 rounded p-1 border border-slate-800">
          <button
            onClick={() => setActiveView('playbooks')}
            className={`px-3 py-1 rounded text-xs transition-colors flex items-center gap-1 ${
              activeView === 'playbooks'
                ? 'bg-slate-700 text-white font-bold'
                : 'text-slate-500 hover:text-white'
            }`}
          >
            <BookOpen className="w-3 h-3" /> Playbooks
          </button>
          <button
            onClick={() => setActiveView('chains')}
            className={`px-3 py-1 rounded text-xs transition-colors flex items-center gap-1 ${
              activeView === 'chains'
                ? 'bg-slate-700 text-white font-bold'
                : 'text-slate-500 hover:text-white'
            }`}
          >
            <List className="w-3 h-3" /> Saved Chains
          </button>
        </div>
      </div>

      {loading && (
        <div className="text-center text-xs text-slate-500 py-8">Loading...</div>
      )}

      {/* Playbooks view */}
      {activeView === 'playbooks' && !loading && (
        <div className="space-y-2">
          {playbooks.length === 0 ? (
            <div className="text-center text-xs text-slate-600 py-12 italic">
              No playbooks available from the team server.
            </div>
          ) : (
            playbooks.map((pb) => (
              <div
                key={pb.id}
                className={`rounded border p-3 transition-colors cursor-pointer ${
                  selectedPlaybook?.id === pb.id
                    ? 'border-red-900/50 bg-red-950/10'
                    : 'border-slate-800 bg-slate-900 hover:border-slate-700'
                }`}
                onClick={() => setSelectedPlaybook(pb)}
              >
                <div className="flex justify-between items-center">
                  <div>
                    <div className="text-xs font-bold text-slate-200 flex items-center gap-1">
                      <BookOpen className="w-3 h-3 text-red-500" />
                      {pb.name}
                    </div>
                    <div className="text-[10px] text-slate-500 mt-0.5">{pb.description}</div>
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleInstantiate(pb.id, pb.name);
                    }}
                  >
                    <span className="flex items-center gap-1">
                      <Play className="w-3 h-3" /> Instantiate
                    </span>
                  </Button>
                </div>
                {selectedPlaybook?.id === pb.id && pb.content && (
                  <pre className="mt-3 bg-slate-950 rounded p-2 text-[10px] text-slate-400 font-mono overflow-x-auto border border-slate-800 max-h-48 overflow-y-auto">
                    {pb.content}
                  </pre>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* Chains view */}
      {activeView === 'chains' && !loading && (
        <div className="space-y-2">
          {chains.length === 0 ? (
            <div className="text-center text-xs text-slate-600 py-12 italic">
              No saved attack chains. Create one in the Attack Chains editor or instantiate a
              playbook.
            </div>
          ) : (
            chains.map((chain) => (
              <div
                key={chain.id}
                className="rounded border border-slate-800 bg-slate-900 p-3 hover:border-slate-700 transition-colors cursor-pointer"
                onClick={() => handleViewChain(chain.id)}
              >
                <div className="flex justify-between items-center">
                  <div>
                    <div className="text-xs font-bold text-slate-200">{chain.name}</div>
                    <div className="text-[10px] text-slate-500">
                      {chain.steps.length} steps &middot; {chain.description || 'No description'}
                    </div>
                  </div>
                  <ChevronRight className="w-4 h-4 text-slate-600" />
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {/* Chain detail panel */}
      {selectedChain && (
        <div className="mt-4 rounded border border-slate-800 bg-slate-900 p-4">
          <div className="flex justify-between items-center mb-3 border-b border-slate-800 pb-2">
            <h3 className="text-xs font-bold text-white uppercase">
              Chain: {selectedChain.name}
            </h3>
            <button
              onClick={() => setSelectedChain(null)}
              className="text-slate-500 hover:text-white text-xs"
            >
              Close
            </button>
          </div>
          <div className="space-y-1">
            {selectedChain.steps.map((step, i) => (
              <div
                key={step.id}
                className="flex items-center gap-2 text-xs bg-slate-950 rounded p-2 border border-slate-800"
              >
                <span className="text-slate-600 font-mono w-6 text-right">{i + 1}.</span>
                <span className="text-red-400 uppercase text-[10px] bg-red-900/20 px-1 rounded">
                  {step.command_type}
                </span>
                <span className="text-slate-300 flex-1 truncate">{step.description}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
