import { useState, useEffect } from 'react';
import { Button } from './ui/Button';
import { ConfirmDialog } from './ui/ConfirmDialog';
import { useToastStore } from '../stores/toastStore';
import * as ipc from '../lib/ipc';
import type { Implant } from '../types';
import { X, Skull } from 'lucide-react';

interface ImplantDetailPanelProps {
  implantId: string;
  onClose: () => void;
}

export default function ImplantDetailPanel({ implantId, onClose }: ImplantDetailPanelProps) {
  const addToast = useToastStore((s) => s.addToast);
  const [implant, setImplant] = useState<Implant | null>(null);
  const [loading, setLoading] = useState(true);
  const [showKillConfirm, setShowKillConfirm] = useState(false);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        const data = await ipc.getImplant(implantId);
        setImplant(data);
      } catch (e) {
        addToast('error', 'Failed to load implant details: ' + e);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [implantId, addToast]);

  const handleKill = async () => {
    try {
      await ipc.killImplant(implantId);
      addToast('success', 'Kill command sent to implant');
      setShowKillConfirm(false);
      onClose();
    } catch (e) {
      addToast('error', 'Failed to kill implant: ' + e);
    }
  };

  if (loading) {
    return (
      <div className="rounded border border-slate-800 bg-slate-900 p-6 text-center text-xs text-slate-500">
        Loading implant details...
      </div>
    );
  }

  if (!implant) return null;

  const fields: [string, string][] = [
    ['Campaign ID', implant.campaign_id || 'N/A'],
    ['Hostname', implant.hostname],
    ['Internal IP', implant.internal_ip],
    ['External IP', implant.external_ip || 'N/A'],
    ['OS', `${implant.os_type} ${implant.os_version}`],
    ['Architecture', implant.architecture],
    ['User', `${implant.domain}\\${implant.username}`],
    ['Privileges', implant.privileges],
    ['Version', implant.implant_version],
    ['Check-in Interval', `${implant.checkin_interval}s`],
    ['Jitter', `${implant.jitter_percent}%`],
    ['Status', implant.status],
  ];

  return (
    <>
      <ConfirmDialog
        open={showKillConfirm}
        title="Kill Implant"
        message={`Permanently terminate implant ${implantId.substring(0, 8)}? This cannot be undone.`}
        confirmLabel="Kill"
        onConfirm={handleKill}
        onCancel={() => setShowKillConfirm(false)}
      />

      <div className="rounded border border-slate-800 bg-slate-900 p-4 shadow-lg">
        <div className="flex justify-between items-center mb-4 border-b border-slate-800 pb-2">
          <h3 className="text-xs font-bold text-white uppercase tracking-wider">
            Implant Detail: <span className="text-red-500 font-mono">{implantId.substring(0, 8)}</span>
          </h3>
          <div className="flex gap-2">
            <Button variant="danger" size="sm" onClick={() => setShowKillConfirm(true)}>
              <span className="flex items-center gap-1">
                <Skull className="w-3 h-3" /> KILL
              </span>
            </Button>
            <button onClick={onClose} className="text-slate-500 hover:text-white transition-colors">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
          {fields.map(([label, value]) => (
            <div key={label} className="bg-slate-950 rounded p-2 border border-slate-800">
              <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-0.5">
                {label}
              </div>
              <div className="text-xs text-slate-200 font-mono truncate">{value}</div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
