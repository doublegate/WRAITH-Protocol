import { useEffect, useState } from 'react';
import { Button } from './ui/Button';
import { ConfirmDialog } from './ui/ConfirmDialog';
import { useToastStore } from '../stores/toastStore';
import * as ipc from '../lib/ipc';
import type { PersistenceItem } from '../types';

export default function PersistenceManager({ implantId }: { implantId: string }) {
    const [items, setItems] = useState<PersistenceItem[]>([]);
    const [confirmTarget, setConfirmTarget] = useState<string | null>(null);
    const addToast = useToastStore((s) => s.addToast);

    const refresh = async () => {
        try {
            const data = await ipc.listPersistence(implantId);
            setItems(data);
        } catch (e) {
            addToast('error', 'Failed to list persistence: ' + e);
        }
    };

    useEffect(() => {
        if (implantId) refresh();
    }, [implantId]);

    const handleRemove = async (id: string) => {
        try {
            await ipc.removePersistence(id);
            addToast('success', 'Persistence mechanism removed');
            refresh();
        } catch (e) {
            addToast('error', 'Failed to remove persistence: ' + e);
        }
        setConfirmTarget(null);
    };

    return (
        <div className="p-4 bg-slate-900 text-white rounded-lg shadow h-full overflow-y-auto">
            <ConfirmDialog
                open={!!confirmTarget}
                title="Remove Persistence"
                message="Are you sure you want to remove this persistence mechanism? This may alert the target."
                confirmLabel="Remove"
                onConfirm={() => confirmTarget && handleRemove(confirmTarget)}
                onCancel={() => setConfirmTarget(null)}
            />

            <div className="flex justify-between items-center mb-4 border-b border-slate-700 pb-2">
                <h2 className="text-lg font-bold flex items-center gap-2">
                    <span className="text-red-500">Persistence Manager</span>
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
                                onClick={() => setConfirmTarget(item.id)}
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
