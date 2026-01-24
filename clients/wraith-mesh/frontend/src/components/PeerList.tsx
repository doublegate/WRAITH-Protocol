// PeerList Component - Connected Peers Sidebar

import { useMemo, useState, useEffect } from 'react';
import { useNetworkStore } from '../stores/networkStore';
import type { PeerType } from '../types';

// Node colors based on peer type
const nodeColors: Record<PeerType, string> = {
  self: '#3b82f6',    // Blue 500
  direct: '#22c55e',  // Green 500
  relay: '#f59e0b',   // Amber 500
  indirect: '#6b7280', // Gray 500
};

export default function PeerList() {
  const { snapshot, selectedPeerId, setSelectedPeer } = useNetworkStore();

  // Group peers by type
  const groupedPeers = useMemo(() => {
    if (!snapshot) return { direct: [], relay: [], indirect: [] };

    const direct = snapshot.nodes.filter((n) => n.peer_type === 'direct');
    const relay = snapshot.nodes.filter((n) => n.peer_type === 'relay');
    const indirect = snapshot.nodes.filter((n) => n.peer_type === 'indirect');

    return { direct, relay, indirect };
  }, [snapshot]);

  // Get link info for a peer
  const getLinkInfo = (peerId: string) => {
    if (!snapshot) return null;
    return snapshot.links.find(
      (l) => l.source === peerId || l.target === peerId
    );
  };

  const totalPeers =
    groupedPeers.direct.length + groupedPeers.relay.length + groupedPeers.indirect.length;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-3 py-2 border-b border-slate-700">
        <h3 className="text-sm font-semibold text-white">
          Peers ({totalPeers})
        </h3>
      </div>

      {/* Peer List */}
      <div className="flex-1 overflow-y-auto">
        {/* Direct Peers */}
        {groupedPeers.direct.length > 0 && (
          <PeerGroup
            title="Direct"
            color={nodeColors.direct}
            peers={groupedPeers.direct}
            selectedPeerId={selectedPeerId}
            onSelect={setSelectedPeer}
            getLinkInfo={getLinkInfo}
          />
        )}

        {/* Relay Servers */}
        {groupedPeers.relay.length > 0 && (
          <PeerGroup
            title="Relay Servers"
            color={nodeColors.relay}
            peers={groupedPeers.relay}
            selectedPeerId={selectedPeerId}
            onSelect={setSelectedPeer}
            getLinkInfo={getLinkInfo}
          />
        )}

        {/* DHT/Indirect */}
        {groupedPeers.indirect.length > 0 && (
          <PeerGroup
            title="DHT Nodes"
            color={nodeColors.indirect}
            peers={groupedPeers.indirect}
            selectedPeerId={selectedPeerId}
            onSelect={setSelectedPeer}
            getLinkInfo={getLinkInfo}
          />
        )}

        {/* Empty State */}
        {totalPeers === 0 && (
          <div className="p-4 text-center text-slate-500 text-sm">
            No peers connected
          </div>
        )}
      </div>
    </div>
  );
}

function PeerGroup({
  title,
  color,
  peers,
  selectedPeerId,
  onSelect,
  getLinkInfo,
}: {
  title: string;
  color: string;
  peers: Array<{ id: string; label: string; last_seen: number }>;
  selectedPeerId: string | null;
  onSelect: (id: string | null) => void;
  getLinkInfo: (id: string) => { latency_ms: number; bandwidth_mbps: number; strength: number } | null | undefined;
}) {
  // Update time every 10 seconds to recalculate "last seen" values
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now() / 1000), 10000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div>
      {/* Group Header */}
      <div className="px-3 py-1.5 bg-slate-800/50 border-b border-slate-700 sticky top-0">
        <div className="flex items-center gap-2">
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: color }}
          />
          <span className="text-xs font-medium text-slate-400">
            {title} ({peers.length})
          </span>
        </div>
      </div>

      {/* Peer Items */}
      <div className="divide-y divide-slate-700/50">
        {peers.map((peer) => {
          const link = getLinkInfo(peer.id);
          const isSelected = selectedPeerId === peer.id;
          const timeSinceLastSeen = Math.floor(now - peer.last_seen);

          return (
            <button
              key={peer.id}
              onClick={() => onSelect(isSelected ? null : peer.id)}
              className={`w-full px-3 py-2 text-left transition-colors ${
                isSelected
                  ? 'bg-wraith-primary/20 border-l-2 border-wraith-primary'
                  : 'hover:bg-slate-800/50 border-l-2 border-transparent'
              }`}
            >
              <div className="flex items-center gap-2">
                <div
                  className="w-2 h-2 rounded-full flex-shrink-0"
                  style={{
                    backgroundColor: color,
                    opacity: timeSinceLastSeen < 30 ? 1 : 0.5,
                  }}
                />
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-white truncate">{peer.label}</div>
                  <div className="text-xs text-slate-500 font-mono truncate">
                    {peer.id.slice(0, 12)}...
                  </div>
                </div>
              </div>

              {link && (
                <div className="mt-1 flex items-center gap-3 text-xs text-slate-500">
                  <span title="Latency">{link.latency_ms}ms</span>
                  <span title="Bandwidth">{link.bandwidth_mbps.toFixed(1)} Mbps</span>
                  <span
                    title="Signal Strength"
                    className={
                      link.strength >= 0.7
                        ? 'text-green-500'
                        : link.strength >= 0.4
                        ? 'text-yellow-500'
                        : 'text-red-500'
                    }
                  >
                    {(link.strength * 100).toFixed(0)}%
                  </span>
                </div>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
