import { useState } from 'react';
import { Console } from './Console';
import DiscoveryDashboard from './DiscoveryDashboard';
import PersistenceManager from './PersistenceManager';

interface Props {
    implantId: string;
    onBack: () => void;
}

export default function BeaconInteraction({ implantId, onBack }: Props) {
    const [subTab, setSubTab] = useState('console');

    return (
        <div className="flex flex-col h-full gap-4">
            <div className="flex items-center justify-between border-b border-slate-800 pb-2">
                <div className="flex gap-4 items-center">
                    <h2 className="text-sm font-bold text-white uppercase tracking-wider flex items-center gap-2">
                        <span className="text-red-500">Target</span> {implantId.substring(0,8)}
                    </h2>
                    <div className="flex bg-slate-900 rounded p-1 gap-1 border border-slate-800">
                        {['Console', 'Discovery', 'Persistence'].map(tab => (
                            <button
                                key={tab}
                                onClick={() => setSubTab(tab.toLowerCase())}
                                className={`px-3 py-1 rounded text-xs transition-colors ${
                                    subTab === tab.toLowerCase() 
                                    ? 'bg-slate-700 text-white font-bold shadow-sm' 
                                    : 'text-slate-500 hover:text-white hover:bg-slate-800'
                                }`}
                            >
                                {tab}
                            </button>
                        ))}
                    </div>
                </div>
                <button 
                    onClick={onBack} 
                    className="rounded bg-slate-800 px-3 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-700 border border-slate-700 transition-colors"
                >
                    CLOSE SESSION
                </button>
            </div>
            <div className="flex-1 min-h-0 relative">
                {subTab === 'console' && <Console implantId={implantId} />}
                {subTab === 'discovery' && <DiscoveryDashboard implantId={implantId} />}
                {subTab === 'persistence' && <PersistenceManager implantId={implantId} />}
            </div>
        </div>
    );
}
