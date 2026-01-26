import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

export default function PhishingBuilder() {
    const [type, setType] = useState('html');
    const [c2Url, setC2Url] = useState('http://localhost:8080');
    const [status, setStatus] = useState('');

    const handleGenerate = async () => {
        try {
            const savePath = await save({
                filters: [{
                    name: 'Phishing Payload',
                    extensions: [type === 'html' ? 'html' : 'docm']
                }]
            });
            
            if (!savePath) return;

            setStatus('Generating...');
            await invoke('create_phishing', { 
                type_: type,  // Rust arg is type_, JS sends mapped name usually? 
                // Tauri command args match Rust function arg names.
                // In lib.rs: fn create_phishing(type_: String, ...)
                // So key should be 'type_'? 
                // Tauri 2.0 usually renames `type` to `type_` automatically if it's a keyword in Rust but valid in JS?
                // Or we match exact Rust argument name. Rust arg is `type_`.
                c2Url, 
                savePath 
            });
            setStatus('Generated successfully!');
        } catch (e) {
            setStatus('Error: ' + e);
        }
    };

    return (
        <div className="p-4 space-y-4 bg-slate-900 text-white rounded-lg shadow">
            <h2 className="text-xl font-bold border-b border-slate-700 pb-2">Phishing Payload Builder</h2>
            
            <div className="space-y-2">
                <label className="block text-sm font-medium">Payload Type</label>
                <select 
                    value={type} 
                    onChange={(e) => setType(e.target.value)}
                    className="w-full p-2 rounded bg-slate-800 border border-slate-700 focus:border-red-500 focus:outline-none"
                >
                    <option value="html">HTML Smuggling (ISO/ZIP)</option>
                    <option value="macro">VBA Macro (Word/Excel)</option>
                </select>
                <p className="text-xs text-slate-400">
                    {type === 'html' ? 
                        "Generates an HTML file that uses JavaScript to drop and execute the implant via an ISO/ZIP container." : 
                        "Generates VBA code to be embedded in an Office document. Uses basic shellcode injection."}
                </p>
            </div>

            <div className="space-y-2">
                <label className="block text-sm font-medium">C2 Connection URL</label>
                <input 
                    type="text" 
                    value={c2Url} 
                    onChange={(e) => setC2Url(e.target.value)}
                    className="w-full p-2 rounded bg-slate-800 border border-slate-700 focus:border-red-500 focus:outline-none"
                    placeholder="http://192.168.1.100:8080"
                />
            </div>

            <button 
                onClick={handleGenerate}
                disabled={status === 'Generating...'}
                className="w-full bg-red-600 hover:bg-red-700 disabled:bg-slate-600 text-white font-bold py-2 px-4 rounded transition-colors"
            >
                {status === 'Generating...' ? 'Building Artifact...' : 'Generate Payload'}
            </button>

            {status && (
                <div className={`mt-4 p-2 rounded text-sm ${status.includes('Error') ? 'bg-red-900/50 text-red-200' : 'bg-green-900/50 text-green-200'}`}>
                    {status}
                </div>
            )}
        </div>
    );
}
