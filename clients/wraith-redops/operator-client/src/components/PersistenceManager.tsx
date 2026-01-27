import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from './ui/Button';

interface PersistenceItem {
    id: string;
    implant_id: string;
    method: string;
    details: string;
}

export default function PersistenceManager({ implantId }: { implantId: string }) {
    const [items, setItems] = useState<PersistenceItem[]>([]);
    
    const refresh = async () => {
        try {
            const json = await invoke<string>('list_persistence', { implantId });
            setItems(JSON.parse(json));
        } catch (e) {
            console.error("Failed to list persistence:", e);
        }
    };

    useEffect(() => { 
        if (implantId) refresh(); 
    }, [implantId]);

    const handleRemove = async (id: string) => {
        if (!confirm("Are you sure you want to remove this persistence mechanism?")) return;
        try {
            await invoke('remove_persistence', { id });
            refresh();
        } catch (e) {
            console.error("Failed to remove persistence:", e);
            alert("Error removing persistence: " + e);
        }
    };

    return (
        <div className="p-4 bg-slate-900 text-white rounded-lg shadow h-full overflow-y-auto">
            <div className="flex justify-between items-center mb-4 border-b border-slate-700 pb-2">
                <h2 className="text-lg font-bold flex items-center gap-2">
                    <span className="text-red-500">⚡</span> Persistence Manager
                </h2>
                <Button onClick={refresh} variant="secondary" size="sm">
                    Refresh
                </Button>
            </div>
            
            <div className="space-y-3">
                {items.length === 0 ? (
                    <div className="text-slate-500 text-center py-8 italic">
                        No persistence mechanisms tracked for this implant.
                    </div>
                ) : (
                    items.map(item => (
                        <div key={item.id} className="flex justify-between items-start bg-slate-800 p-3 rounded border border-slate-700 hover:border-slate-600 transition-colors">
                            <div className="overflow-hidden">
                                <div className="flex items-center gap-2 mb-1">
                                    <span className="font-bold text-red-400 uppercase text-[10px] tracking-wider bg-red-900/20 px-1 rounded">
                                        {item.method}
                                    </span>
                                    <span className="text-slate-500 text-[10px]">{item.id.substring(0,8)}</span>
                                </div>
                                <div className="text-sm font-mono text-slate-300 break-all">{item.details}</div>
                            </div>
                            <Button 
                                onClick={() => handleRemove(item.id)}
                                variant="danger"
                                size="sm"
                                className="ml-4 whitespace-nowrap"
                            >
                                Cleanup
                            </Button>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}
