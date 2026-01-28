import { useState, useEffect } from 'react';
import { Button } from './ui/Button';
import { useToastStore } from '../stores/toastStore';
import { useAppStore } from '../stores/appStore';
import * as ipc from '../lib/ipc';
import type { Campaign } from '../types';
import { X, Save } from 'lucide-react';

interface CampaignDetailProps {
  campaignId: string;
  onClose: () => void;
}

export default function CampaignDetail({ campaignId, onClose }: CampaignDetailProps) {
  const addToast = useToastStore((s) => s.addToast);
  const refreshCampaigns = useAppStore((s) => s.refreshCampaigns);

  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState('');
  const [editDesc, setEditDesc] = useState('');
  const [editStatus, setEditStatus] = useState('');

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        const data = await ipc.getCampaign(campaignId);
        setCampaign(data);
        setEditName(data.name);
        setEditDesc(data.description);
        setEditStatus(data.status);
      } catch (e) {
        addToast('error', 'Failed to load campaign: ' + e);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [campaignId, addToast]);

  const handleSave = async () => {
    try {
      const updated = await ipc.updateCampaign(campaignId, editName, editDesc, editStatus);
      setCampaign(updated);
      setEditing(false);
      refreshCampaigns();
      addToast('success', 'Campaign updated');
    } catch (e) {
      addToast('error', 'Failed to update campaign: ' + e);
    }
  };

  if (loading) {
    return (
      <div className="rounded border border-slate-800 bg-slate-900 p-6 text-center text-xs text-slate-500">
        Loading campaign...
      </div>
    );
  }

  if (!campaign) return null;

  return (
    <div className="rounded border border-slate-800 bg-slate-900 p-4 shadow-lg mb-4">
      <div className="flex justify-between items-center mb-4 border-b border-slate-800 pb-2">
        <h3 className="text-xs font-bold text-white uppercase tracking-wider">
          Campaign: <span className="text-red-500">{campaign.name}</span>
        </h3>
        <div className="flex gap-2">
          {!editing ? (
            <Button variant="secondary" size="sm" onClick={() => setEditing(true)}>
              EDIT
            </Button>
          ) : (
            <Button size="sm" onClick={handleSave}>
              <span className="flex items-center gap-1">
                <Save className="w-3 h-3" /> SAVE
              </span>
            </Button>
          )}
          <button onClick={onClose} className="text-slate-500 hover:text-white transition-colors">
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {editing ? (
        <div className="grid gap-4 max-w-lg">
          <div>
            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Name</label>
            <input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
            />
          </div>
          <div>
            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Description</label>
            <textarea
              value={editDesc}
              onChange={(e) => setEditDesc(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded min-h-[60px]"
            />
          </div>
          <div>
            <label className="text-[10px] text-slate-500 uppercase mb-1 block">Status</label>
            <select
              value={editStatus}
              onChange={(e) => setEditStatus(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded"
            >
              <option value="active">Active</option>
              <option value="paused">Paused</option>
              <option value="completed">Completed</option>
              <option value="archived">Archived</option>
            </select>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
          <div className="bg-slate-950 rounded p-2 border border-slate-800">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-0.5">Status</div>
            <span className="px-2 py-0.5 bg-green-900/20 text-green-500 rounded border border-green-900/30 text-xs">
              {campaign.status.toUpperCase()}
            </span>
          </div>
          <div className="bg-slate-950 rounded p-2 border border-slate-800">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-0.5">Implants</div>
            <div className="text-xs text-slate-200">{campaign.implant_count}</div>
          </div>
          <div className="bg-slate-950 rounded p-2 border border-slate-800">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-0.5">Active</div>
            <div className="text-xs text-green-400">{campaign.active_implant_count}</div>
          </div>
          <div className="bg-slate-950 rounded p-2 border border-slate-800 col-span-2 lg:col-span-1">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-0.5">Description</div>
            <div className="text-xs text-slate-300 truncate">{campaign.description || 'None'}</div>
          </div>
        </div>
      )}
    </div>
  );
}
