import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

interface Credential {
    id: string;
    username: string;
    domain: string;
    source: string;
    credential_type: string;
}

interface Artifact {
    id: string;
    filename: string;
    size: number;
}

export default function LootGallery() {
    const [creds, setCreds] = useState<Credential[]>([]);
    const [artifacts, setArtifacts] = useState<Artifact[]>([]);
    const [activeTab, setActiveTab] = useState<'credentials' | 'files'>('credentials');

    const refresh = async () => {
        try {
            const credsJson = await invoke<string>('list_credentials');
            setCreds(JSON.parse(credsJson));
            const artsJson = await invoke<string>('list_artifacts');
            setArtifacts(JSON.parse(artsJson));
        } catch (e) {
            // Error handling
        }
    };

    useEffect(() => { refresh(); }, []);

    const handleDownload = async (id: string, filename: string) => {
        try {
            const savePath = await save({ defaultPath: filename });
            if (!savePath) return;
            await invoke('download_artifact', { artifactId: id, savePath });
            // alert("Download complete"); // Avoid native alerts in production UI if possible, status bar preferred
        } catch (e) {
            // Error handling
        }
    };

    return (
        <div className="p-4 bg-slate-900 text-white rounded-lg h-full flex flex-col shadow">
            <div className="flex justify-between items-center mb-4 border-b border-slate-700 pb-2">
                <h2 className="text-lg font-bold flex items-center gap-2">
                    <span className="text-yellow-500">💰</span> Loot Gallery
                </h2>
                <div className="flex gap-2">
                    <button 
                        onClick={() => setActiveTab('credentials')}
                        className={`px-3 py-1 rounded text-xs font-bold transition-colors ${activeTab === 'credentials' ? 'bg-red-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}`}
                    >
                        Credentials
                    </button>
                    <button 
                        onClick={() => setActiveTab('files')}
                        className={`px-3 py-1 rounded text-xs font-bold transition-colors ${activeTab === 'files' ? 'bg-red-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}`}
                    >
                        Files
                    </button>
                    <button onClick={refresh} className="bg-slate-700 hover:bg-slate-600 px-2 py-1 rounded text-xs">↻</button>
                </div>
            </div>

            <div className="flex-1 overflow-y-auto pr-2">
                {activeTab === 'credentials' ? (
                    <div className="space-y-2">
                        {creds.length === 0 ? (
                            <div className="text-slate-500 text-center py-8 italic">No credentials collected yet.</div>
                        ) : (
                            creds.map(c => (
                                <div key={c.id} className="bg-slate-800 p-3 rounded border border-slate-700 flex justify-between items-center hover:border-slate-600 transition-colors">
                                    <div>
                                        <div className="font-bold text-red-400 font-mono text-sm">{c.domain}\{c.username}</div>
                                        <div className="text-[10px] text-slate-400 uppercase tracking-wider">{c.source} • {c.credential_type}</div>
                                    </div>
                                    <button 
                                        className="text-xs bg-slate-700 hover:bg-slate-600 px-2 py-1 rounded transition-colors"
                                        onClick={() => navigator.clipboard.writeText(`${c.domain}\\\${c.username}`)}
                                    >
                                        Copy
                                    </button>
                                </div>
                            ))
                        )}
                    </div>
                ) : (
                    <div className="space-y-2">
                        {artifacts.length === 0 ? (
                            <div className="text-slate-500 text-center py-8 italic">No artifacts collected yet.</div>
                        ) : (
                            artifacts.map(a => (
                                <div key={a.id} className="bg-slate-800 p-3 rounded border border-slate-700 flex justify-between items-center hover:border-slate-600 transition-colors">
                                    <div className="flex items-center gap-3">
                                        <div className="text-2xl">📄</div>
                                        <div>
                                            <div className="font-bold text-blue-400 text-sm">{a.filename}</div>
                                            <div className="text-[10px] text-slate-400">{(a.size / 1024).toFixed(1)} KB</div>
                                        </div>
                                    </div>
                                    <button 
                                        onClick={() => handleDownload(a.id, a.filename)}
                                        className="bg-blue-900/30 hover:bg-blue-900/50 text-blue-200 px-3 py-1 rounded text-xs border border-blue-900/50 transition-colors"
                                    >
                                        Download
                                    </button>
                                </div>
                            ))
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}
