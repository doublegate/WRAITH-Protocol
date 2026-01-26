import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Command {
    id: string;
    command_type: string;
    status: string;
    created_at: string; // sort by date
}

interface CommandResult {
    output: string;
}

export default function DiscoveryDashboard({ implantId }: { implantId: string }) {
    const [sysInfo, setSysInfo] = useState<string>('');
    const [netScan, setNetScan] = useState<string>('');
    const [loading, setLoading] = useState(false);

    const refresh = async () => {
        if (!implantId) return;
        setLoading(true);
        try {
            // 1. List commands
            const cmdsJson = await invoke<string>('list_commands', { implantId });
            const cmds: Command[] = JSON.parse(cmdsJson);
            
            // 2. Find latest sys_info
            const sysCmd = cmds.filter(c => c.command_type === 'sys_info' && c.status === 'completed')[0]; // List is sorted desc?
            if (sysCmd) {
                const resJson = await invoke<string>('get_command_result', { commandId: sysCmd.id });
                const res: CommandResult = JSON.parse(resJson);
                setSysInfo(res.output);
            }

            // 3. Find latest net_scan
            const netCmd = cmds.filter(c => c.command_type === 'net_scan' && c.status === 'completed')[0];
            if (netCmd) {
                const resJson = await invoke<string>('get_command_result', { commandId: netCmd.id });
                const res: CommandResult = JSON.parse(resJson);
                setNetScan(res.output);
            }
        } catch (e) {
            console.error("Failed to fetch discovery data:", e);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => { refresh(); }, [implantId]);

    return (
        <div className="p-4 bg-slate-900 text-white rounded-lg h-full overflow-y-auto shadow">
            <div className="flex justify-between items-center mb-4 border-b border-slate-700 pb-2">
                <h2 className="text-lg font-bold flex items-center gap-2">
                    <span className="text-blue-500">🔍</span> Discovery Dashboard
                </h2>
                <button onClick={refresh} className="text-xs bg-slate-700 hover:bg-slate-600 px-2 py-1 rounded transition-colors">
                    {loading ? 'Refreshing...' : 'Refresh'}
                </button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <div className="bg-slate-800 p-4 rounded border border-slate-700 flex flex-col h-[400px]">
                    <h3 className="font-bold text-blue-400 mb-2 text-sm uppercase tracking-wider">System Information</h3>
                    <div className="flex-1 bg-slate-950 p-3 rounded font-mono text-xs overflow-auto text-slate-300 border border-slate-900">
                        {sysInfo || <span className="text-slate-600 italic">No system info found. Execute 'sys_info' or 'recon' in console.</span>}
                    </div>
                </div>
                
                <div className="bg-slate-800 p-4 rounded border border-slate-700 flex flex-col h-[400px]">
                    <h3 className="font-bold text-green-400 mb-2 text-sm uppercase tracking-wider">Network Reconnaissance</h3>
                    <div className="flex-1 bg-slate-950 p-3 rounded font-mono text-xs overflow-auto text-slate-300 border border-slate-900">
                        {netScan || <span className="text-slate-600 italic">No network scan data found. Execute 'net_scan <target>' in console.</span>}
                    </div>
                </div>
            </div>
        </div>
    );
}
