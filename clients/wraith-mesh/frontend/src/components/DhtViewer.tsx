// DhtViewer Component - DHT Routing Table and Key Lookup Interface

import { useState, useEffect, useCallback, useMemo } from 'react';
import { useNetworkStore } from '../stores/networkStore';
import type { RoutingBucket, LookupResult, StoredKey } from '../types';

export default function DhtViewer() {
  const {
    routingTable,
    lookupResult,
    storedKeys,
    fetchRoutingTable,
    lookupKey,
    getStoredKeys,
    loading
  } = useNetworkStore();

  const [lookupInput, setLookupInput] = useState('');
  const [activeSection, setActiveSection] = useState<'routing' | 'lookup' | 'storage'>('routing');

  // Fetch routing table and stored keys on mount
  useEffect(() => {
    fetchRoutingTable();
    getStoredKeys();
  }, [fetchRoutingTable, getStoredKeys]);

  const handleLookup = useCallback(() => {
    if (lookupInput.trim()) {
      lookupKey(lookupInput.trim());
      setActiveSection('lookup');
    }
  }, [lookupInput, lookupKey]);

  const handleKeyPress = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleLookup();
    }
  }, [handleLookup]);

  const totalPeers = routingTable?.reduce((sum, bucket) => sum + bucket.peer_count, 0) ?? 0;
  const activeBuckets = routingTable?.filter((b) => b.peer_count > 0).length ?? 0;

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between p-3 bg-bg-secondary border-b border-slate-700">
        <div className="flex items-center gap-4">
          <h2 className="text-lg font-semibold text-white">DHT Explorer</h2>
          <div className="flex items-center gap-4 text-sm text-slate-400">
            <span>
              <span className="text-white font-medium">{totalPeers}</span> peers in{' '}
              <span className="text-white font-medium">{activeBuckets}</span> buckets
            </span>
          </div>
        </div>

        {/* Section Tabs */}
        <div className="flex items-center gap-1 bg-slate-700/50 rounded-lg p-1">
          <SectionTab
            label="Routing Table"
            active={activeSection === 'routing'}
            onClick={() => setActiveSection('routing')}
          />
          <SectionTab
            label="Key Lookup"
            active={activeSection === 'lookup'}
            onClick={() => setActiveSection('lookup')}
          />
          <SectionTab
            label="Local Storage"
            active={activeSection === 'storage'}
            onClick={() => setActiveSection('storage')}
          />
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden p-4">
        {activeSection === 'routing' && (
          <RoutingTableView
            routingTable={routingTable}
            onRefresh={fetchRoutingTable}
            loading={loading}
          />
        )}
        {activeSection === 'lookup' && (
          <KeyLookupView
            lookupInput={lookupInput}
            setLookupInput={setLookupInput}
            onLookup={handleLookup}
            onKeyPress={handleKeyPress}
            lookupResult={lookupResult}
            loading={loading}
          />
        )}
        {activeSection === 'storage' && (
          <LocalStorageView
            storedKeys={storedKeys}
            onRefresh={getStoredKeys}
            loading={loading}
          />
        )}
      </div>
    </div>
  );
}

function SectionTab({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
        active
          ? 'bg-wraith-primary text-white'
          : 'text-slate-400 hover:text-white'
      }`}
    >
      {label}
    </button>
  );
}

function RoutingTableView({
  routingTable,
  onRefresh,
  loading,
}: {
  routingTable: RoutingBucket[] | null;
  onRefresh: () => void;
  loading: boolean;
}) {
  if (!routingTable) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        Loading routing table...
      </div>
    );
  }

  // Calculate bucket fill percentages for visualization
  const maxPeers = Math.max(...routingTable.map((b) => b.peer_count), 1);

  return (
    <div className="h-full flex flex-col">
      {/* Header with Refresh */}
      <div className="flex items-center justify-between mb-4">
        <div className="text-sm text-slate-400">
          Kademlia routing table with {routingTable.length} distance buckets (k-buckets)
        </div>
        <button
          onClick={onRefresh}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-50 rounded-lg text-sm text-white transition-colors"
        >
          <svg className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Refresh
        </button>
      </div>

      {/* Bucket Visualization */}
      <div className="flex-1 overflow-y-auto">
        <div className="grid grid-cols-1 gap-2">
          {routingTable.map((bucket) => (
            <BucketRow
              key={bucket.index}
              bucket={bucket}
              maxPeers={maxPeers}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function BucketRow({
  bucket,
  maxPeers,
}: {
  bucket: RoutingBucket;
  maxPeers: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const fillPercent = (bucket.peer_count / maxPeers) * 100;
  const capacityPercent = (bucket.peer_count / bucket.capacity) * 100;

  return (
    <div className="bg-slate-800/50 rounded-lg overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-3 flex items-center gap-4 hover:bg-slate-700/50 transition-colors"
      >
        {/* Bucket Index */}
        <div className="w-12 text-left">
          <span className="text-xs text-slate-500">Bucket</span>
          <div className="text-sm font-mono text-white">{bucket.index}</div>
        </div>

        {/* Distance Range */}
        <div className="flex-1">
          <div className="text-xs text-slate-500 mb-1">
            Distance: 2^{bucket.index} - 2^{bucket.index + 1}
          </div>
          <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-cyan-500 to-violet-500 transition-all"
              style={{ width: `${fillPercent}%` }}
            />
          </div>
        </div>

        {/* Peer Count */}
        <div className="w-24 text-right">
          <span className="text-sm text-white font-medium">{bucket.peer_count}</span>
          <span className="text-sm text-slate-500">/{bucket.capacity}</span>
          <div className="text-xs text-slate-500">{capacityPercent.toFixed(0)}% full</div>
        </div>

        {/* Expand Icon */}
        <svg
          className={`w-4 h-4 text-slate-400 transition-transform ${expanded ? 'rotate-180' : ''}`}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {/* Expanded Peer List */}
      {expanded && bucket.peers.length > 0 && (
        <div className="px-4 pb-3 border-t border-slate-700">
          <div className="mt-3 space-y-2">
            {bucket.peers.map((peer) => (
              <div
                key={peer.peer_id}
                className="flex items-center justify-between px-3 py-2 bg-slate-700/50 rounded-lg text-sm"
              >
                <div className="font-mono text-slate-300 truncate max-w-md">
                  {peer.peer_id}
                </div>
                <div className="flex items-center gap-4 text-slate-500">
                  <span>{peer.rtt_ms}ms</span>
                  <span>
                    Last seen: {new Date(peer.last_seen * 1000).toLocaleTimeString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Empty State */}
      {expanded && bucket.peers.length === 0 && (
        <div className="px-4 pb-3 border-t border-slate-700">
          <div className="mt-3 text-sm text-slate-500 text-center py-2">
            No peers in this bucket
          </div>
        </div>
      )}
    </div>
  );
}

function KeyLookupView({
  lookupInput,
  setLookupInput,
  onLookup,
  onKeyPress,
  lookupResult,
  loading,
}: {
  lookupInput: string;
  setLookupInput: (value: string) => void;
  onLookup: () => void;
  onKeyPress: (e: React.KeyboardEvent) => void;
  lookupResult: LookupResult | null;
  loading: boolean;
}) {
  return (
    <div className="h-full flex flex-col">
      {/* Search Input */}
      <div className="flex gap-2 mb-4">
        <div className="flex-1 relative">
          <input
            type="text"
            value={lookupInput}
            onChange={(e) => setLookupInput(e.target.value)}
            onKeyDown={onKeyPress}
            placeholder="Enter key to lookup (peer ID, content hash, etc.)"
            className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-2.5 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-transparent font-mono"
          />
          {lookupInput && (
            <button
              onClick={() => setLookupInput('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-500 hover:text-white"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
        <button
          onClick={onLookup}
          disabled={loading || !lookupInput.trim()}
          className="px-4 py-2.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors"
        >
          {loading ? 'Looking up...' : 'Lookup'}
        </button>
      </div>

      {/* Lookup Result */}
      <div className="flex-1 overflow-y-auto">
        {lookupResult ? (
          <LookupResultDisplay result={lookupResult} />
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-slate-500">
            <svg className="w-12 h-12 mb-4 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            <p>Enter a key to perform a DHT lookup</p>
            <p className="text-sm mt-1">The lookup will trace through the DHT routing</p>
          </div>
        )}
      </div>
    </div>
  );
}

function LookupResultDisplay({ result }: { result: LookupResult }) {
  const statusColors = {
    found: 'text-green-400',
    not_found: 'text-yellow-400',
    timeout: 'text-red-400',
    error: 'text-red-400',
  };

  return (
    <div className="space-y-4">
      {/* Result Header */}
      <div className="bg-slate-800/50 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="font-semibold text-white">Lookup Result</h3>
          <span className={`text-sm font-medium ${statusColors[result.status]}`}>
            {result.status.toUpperCase()}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-slate-500">Key:</span>
            <div className="font-mono text-white truncate">{result.key}</div>
          </div>
          <div>
            <span className="text-slate-500">Total Time:</span>
            <div className="text-white">{result.total_time_ms}ms</div>
          </div>
          <div>
            <span className="text-slate-500">Hops:</span>
            <div className="text-white">{result.hops.length}</div>
          </div>
          <div>
            <span className="text-slate-500">Value Found:</span>
            <div className="text-white">
              {result.value ? 'Yes' : 'No'}
            </div>
          </div>
        </div>

        {result.value && (
          <div className="mt-4 p-3 bg-slate-700/50 rounded-lg">
            <span className="text-xs text-slate-500 block mb-1">Value:</span>
            <pre className="text-sm text-white font-mono overflow-x-auto">
              {result.value}
            </pre>
          </div>
        )}
      </div>

      {/* Hop Trace */}
      <div className="bg-slate-800/50 rounded-lg p-4">
        <h3 className="font-semibold text-white mb-3">Lookup Trace ({result.hops.length} hops)</h3>
        <div className="space-y-2">
          {result.hops.map((hop, index) => (
            <div
              key={index}
              className="flex items-center gap-3 p-2 bg-slate-700/50 rounded-lg"
            >
              {/* Hop Number */}
              <div className="w-8 h-8 flex items-center justify-center bg-slate-600 rounded-full text-sm font-medium text-white">
                {index + 1}
              </div>

              {/* Hop Details */}
              <div className="flex-1 min-w-0">
                <div className="font-mono text-sm text-white truncate">
                  {hop.peer_id}
                </div>
                <div className="text-xs text-slate-500">
                  Distance: {hop.distance} | RTT: {hop.rtt_ms}ms
                </div>
              </div>

              {/* Status Icon */}
              <div
                className={`w-2 h-2 rounded-full ${
                  hop.responded ? 'bg-green-500' : 'bg-red-500'
                }`}
                title={hop.responded ? 'Responded' : 'No response'}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function LocalStorageView({
  storedKeys,
  onRefresh,
  loading,
}: {
  storedKeys: StoredKey[] | null;
  onRefresh: () => void;
  loading: boolean;
}) {
  const [filter, setFilter] = useState('');

  const filteredKeys = storedKeys?.filter(
    (key) =>
      key.key.toLowerCase().includes(filter.toLowerCase()) ||
      key.value.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-4">
          <div className="relative">
            <input
              type="text"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter keys..."
              className="bg-slate-700 border border-slate-600 rounded-lg pl-9 pr-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-transparent w-64"
            />
            <svg
              className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </div>
          <span className="text-sm text-slate-500">
            {filteredKeys?.length ?? 0} keys stored locally
          </span>
        </div>
        <button
          onClick={onRefresh}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-50 rounded-lg text-sm text-white transition-colors"
        >
          <svg className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Refresh
        </button>
      </div>

      {/* Keys Table */}
      <div className="flex-1 overflow-y-auto">
        {filteredKeys && filteredKeys.length > 0 ? (
          <div className="space-y-2">
            {filteredKeys.map((key) => (
              <StoredKeyRow key={key.key} storedKey={key} />
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-slate-500">
            <svg className="w-12 h-12 mb-4 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
            </svg>
            <p>{filter ? 'No keys match your filter' : 'No keys stored locally'}</p>
          </div>
        )}
      </div>
    </div>
  );
}

function StoredKeyRow({ storedKey }: { storedKey: StoredKey }) {
  const [expanded, setExpanded] = useState(false);
  const [now] = useState(() => Date.now()); // Capture time once at mount
  const expiresAt = new Date(storedKey.expires_at * 1000);
  const isExpiring = useMemo(() => expiresAt.getTime() - now < 3600000, [expiresAt, now]); // Less than 1 hour

  return (
    <div className="bg-slate-800/50 rounded-lg overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-3 flex items-center gap-4 hover:bg-slate-700/50 transition-colors"
      >
        {/* Key */}
        <div className="flex-1 text-left min-w-0">
          <div className="font-mono text-sm text-white truncate">
            {storedKey.key}
          </div>
          <div className="text-xs text-slate-500">
            Provider: {storedKey.provider_id.slice(0, 16)}...
          </div>
        </div>

        {/* Size */}
        <div className="text-sm text-slate-400">
          {storedKey.size_bytes} bytes
        </div>

        {/* Expiration */}
        <div className={`text-sm ${isExpiring ? 'text-yellow-400' : 'text-slate-400'}`}>
          Expires: {expiresAt.toLocaleString()}
        </div>

        {/* Expand Icon */}
        <svg
          className={`w-4 h-4 text-slate-400 transition-transform ${expanded ? 'rotate-180' : ''}`}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {expanded && (
        <div className="px-4 pb-3 border-t border-slate-700">
          <div className="mt-3 p-3 bg-slate-700/50 rounded-lg">
            <span className="text-xs text-slate-500 block mb-1">Value:</span>
            <pre className="text-sm text-white font-mono overflow-x-auto whitespace-pre-wrap break-all">
              {storedKey.value}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
