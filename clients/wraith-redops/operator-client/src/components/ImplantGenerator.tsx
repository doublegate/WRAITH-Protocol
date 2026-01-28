import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { useToastStore } from '../stores/toastStore';
import * as ipc from '../lib/ipc';
import { Button } from './ui/Button';
import { Download } from 'lucide-react';

export default function ImplantGenerator() {
  const addToast = useToastStore((s) => s.addToast);

  const [platform, setPlatform] = useState('windows');
  const [arch, setArch] = useState('x86_64');
  const [format, setFormat] = useState('exe');
  const [c2Url, setC2Url] = useState('http://localhost:8080');
  const [sleepInterval, setSleepInterval] = useState(60);
  const [generating, setGenerating] = useState(false);

  const handleGenerate = async () => {
    try {
      const ext =
        format === 'exe'
          ? 'exe'
          : format === 'dll'
            ? 'dll'
            : format === 'shellcode'
              ? 'bin'
              : 'elf';
      const savePath = await save({
        filters: [{ name: 'Implant Binary', extensions: [ext] }],
      });
      if (!savePath) return;

      setGenerating(true);
      await ipc.generateImplant(platform, arch, format, c2Url, sleepInterval, savePath);
      addToast('success', 'Implant binary generated successfully');
    } catch (e) {
      addToast('error', 'Generation failed: ' + e);
    } finally {
      setGenerating(false);
    }
  };

  return (
    <div className="max-w-xl">
      <h2 className="text-sm font-bold text-white uppercase tracking-wider mb-6">
        Implant Generator
      </h2>

      <div className="rounded border border-slate-800 bg-slate-900 p-6 space-y-5">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="text-[10px] text-slate-500 uppercase mb-1 block">
              Target Platform
            </label>
            <select
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
            >
              <option value="windows">Windows</option>
              <option value="linux">Linux</option>
              <option value="macos">macOS</option>
            </select>
          </div>
          <div>
            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Architecture</label>
            <select
              value={arch}
              onChange={(e) => setArch(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
            >
              <option value="x86_64">x86_64</option>
              <option value="x86">x86</option>
              <option value="aarch64">aarch64</option>
            </select>
          </div>
        </div>

        <div>
          <label className="text-[10px] text-slate-500 uppercase mb-1 block">Output Format</label>
          <select
            value={format}
            onChange={(e) => setFormat(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
          >
            <option value="exe">Executable (.exe)</option>
            <option value="dll">DLL (.dll)</option>
            <option value="shellcode">Shellcode (.bin)</option>
            <option value="elf">ELF (Linux)</option>
          </select>
        </div>

        <div>
          <label className="text-[10px] text-slate-500 uppercase mb-1 block">
            C2 Callback URL
          </label>
          <input
            value={c2Url}
            onChange={(e) => setC2Url(e.target.value)}
            placeholder="http://192.168.1.100:8080"
            className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
          />
        </div>

        <div>
          <label className="text-[10px] text-slate-500 uppercase mb-1 block">
            Sleep Interval (seconds)
          </label>
          <input
            type="number"
            value={sleepInterval}
            onChange={(e) => setSleepInterval(parseInt(e.target.value) || 0)}
            min={1}
            className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
          />
          <p className="text-[10px] text-slate-600 mt-1">
            Time between beacon check-ins. Lower = faster response, higher = stealthier.
          </p>
        </div>

        <Button className="w-full" onClick={handleGenerate} disabled={generating}>
          <span className="flex items-center justify-center gap-2">
            <Download className="w-4 h-4" />
            {generating ? 'Generating...' : 'Generate & Save'}
          </span>
        </Button>
      </div>
    </div>
  );
}
