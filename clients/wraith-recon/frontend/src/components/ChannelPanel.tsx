// WRAITH Recon - Channel Panel Component

import { useState } from 'react';
import { useChannelStore } from '../stores/channelStore';
import { useEngagementStore } from '../stores/engagementStore';
import type { ChannelType } from '../types';

export function ChannelPanel() {
  const { status } = useEngagementStore();
  const {
    channels, selectedChannelId, selectedChannelStats,
    openChannel, closeChannel, sendData, selectChannel,
    loading, error,
  } = useChannelStore();

  const [newChannelType, setNewChannelType] = useState<ChannelType>('Https');
  const [newTarget, setNewTarget] = useState('');
  const [newPort, setNewPort] = useState('');
  const [showNewChannel, setShowNewChannel] = useState(false);
  const [dataToSend, setDataToSend] = useState('');

  const canOperate = status === 'Active';

  const handleOpenChannel = async () => {
    if (!newTarget.trim()) return;

    try {
      await openChannel(
        newChannelType,
        newTarget.trim(),
        newPort ? parseInt(newPort) : undefined
      );
      setNewTarget('');
      setNewPort('');
      setShowNewChannel(false);
    } catch (e) {
      console.error('Failed to open channel:', e);
    }
  };

  const handleSendData = async (channelId: string) => {
    if (!dataToSend.trim()) return;

    try {
      // Convert string to byte array
      const encoder = new TextEncoder();
      const bytes = Array.from(encoder.encode(dataToSend));
      await sendData(channelId, bytes);
      setDataToSend('');
    } catch (e) {
      console.error('Failed to send data:', e);
    }
  };

  const getChannelTypeIcon = (type: ChannelType) => {
    switch (type) {
      case 'Https':
        return (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
          </svg>
        );
      case 'DnsTunnel':
        return (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
          </svg>
        );
      case 'Icmp':
        return (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.143 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
          </svg>
        );
      case 'Udp':
        return (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
        );
      case 'TcpMimicry':
        return (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        );
      default:
        return null;
    }
  };

  const getStateColor = (state: string) => {
    switch (state) {
      case 'Open':
      case 'Active':
        return 'text-green-400';
      case 'Closed':
        return 'text-gray-400';
      case 'Error':
        return 'text-red-400';
      default:
        return 'text-text-secondary';
    }
  };

  return (
    <div className="card flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b border-border-primary flex justify-between items-center">
        <h2 className="text-lg font-semibold text-text-primary flex items-center gap-2">
          <svg className="w-5 h-5 text-primary-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
          </svg>
          Exfiltration Channels
        </h2>
        <button
          onClick={() => setShowNewChannel(!showNewChannel)}
          disabled={!canOperate}
          className="btn btn-primary text-sm"
        >
          + New Channel
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="p-3 m-4 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* New Channel Form */}
      {showNewChannel && (
        <div className="p-4 border-b border-border-primary bg-bg-tertiary/50">
          <div className="grid grid-cols-3 gap-3 mb-3">
            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">Type</label>
              <select
                value={newChannelType}
                onChange={(e) => setNewChannelType(e.target.value as ChannelType)}
                className="input text-sm"
              >
                <option value="Https">HTTPS</option>
                <option value="DnsTunnel">DNS Tunnel</option>
                <option value="Icmp">ICMP</option>
                <option value="Udp">UDP</option>
                <option value="TcpMimicry">TCP Mimicry</option>
              </select>
            </div>
            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">Target</label>
              <input
                type="text"
                value={newTarget}
                onChange={(e) => setNewTarget(e.target.value)}
                placeholder="IP or domain"
                className="input text-sm"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">Port (optional)</label>
              <input
                type="number"
                value={newPort}
                onChange={(e) => setNewPort(e.target.value)}
                placeholder="Auto"
                className="input text-sm"
              />
            </div>
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleOpenChannel}
              disabled={!newTarget.trim() || loading}
              className="btn btn-primary text-sm"
            >
              {loading ? 'Opening...' : 'Open Channel'}
            </button>
            <button
              onClick={() => setShowNewChannel(false)}
              className="btn btn-secondary text-sm"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Channel List */}
      <div className="flex-1 overflow-auto p-4">
        {channels.length === 0 ? (
          <div className="empty-state">
            <svg className="empty-state-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
            <h3 className="empty-state-title">No Active Channels</h3>
            <p className="empty-state-description">
              Open a covert channel to establish data exfiltration paths.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {channels.map((channel) => (
              <div
                key={channel.id}
                onClick={() => selectChannel(channel.id)}
                className={`
                  p-3 rounded-lg cursor-pointer transition-all
                  ${selectedChannelId === channel.id
                    ? 'bg-primary-500/10 border border-primary-500/30'
                    : 'bg-bg-tertiary hover:bg-bg-hover border border-transparent'}
                `}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="text-primary-400">
                      {getChannelTypeIcon(channel.channel_type)}
                    </span>
                    <span className="font-medium text-text-primary">{channel.channel_type}</span>
                    <span className={`text-xs ${getStateColor(channel.state)}`}>
                      {channel.state}
                    </span>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      closeChannel(channel.id);
                    }}
                    disabled={loading}
                    className="text-xs text-red-400 hover:text-red-300"
                  >
                    Close
                  </button>
                </div>

                <div className="text-sm font-mono text-text-secondary">
                  {channel.target}
                  {channel.port && <span className="text-text-muted">:{channel.port}</span>}
                </div>

                <div className="mt-2 flex gap-4 text-xs text-text-muted">
                  <span>Sent: {(channel.bytes_sent / 1024).toFixed(1)} KB</span>
                  <span>Recv: {(channel.bytes_received / 1024).toFixed(1)} KB</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Selected Channel Details */}
      {selectedChannelId && selectedChannelStats && (
        <div className="p-4 border-t border-border-primary bg-bg-tertiary/50">
          <h4 className="text-sm font-medium text-text-secondary mb-2">Channel Stats</h4>
          <div className="grid grid-cols-3 gap-2 text-sm mb-3">
            <div>
              <span className="text-text-muted">Packets Sent:</span>
              <span className="ml-2 text-text-primary">{selectedChannelStats.packets_sent}</span>
            </div>
            <div>
              <span className="text-text-muted">Packets Recv:</span>
              <span className="ml-2 text-text-primary">{selectedChannelStats.packets_received}</span>
            </div>
            <div>
              <span className="text-text-muted">Errors:</span>
              <span className="ml-2 text-red-400">{selectedChannelStats.errors}</span>
            </div>
          </div>

          {/* Send Data */}
          <div className="flex gap-2">
            <input
              type="text"
              value={dataToSend}
              onChange={(e) => setDataToSend(e.target.value)}
              placeholder="Data to send..."
              className="input flex-1 text-sm"
              onKeyDown={(e) => e.key === 'Enter' && handleSendData(selectedChannelId)}
            />
            <button
              onClick={() => handleSendData(selectedChannelId)}
              disabled={!dataToSend.trim() || loading}
              className="btn btn-primary text-sm"
            >
              Send
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
