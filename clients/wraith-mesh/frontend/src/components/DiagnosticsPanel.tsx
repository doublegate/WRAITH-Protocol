// DiagnosticsPanel Component - Network Diagnostics Tools

import { useState, useCallback } from 'react';
import { useNetworkStore } from '../stores/networkStore';
import type { PingResult, BandwidthResult, HealthReport, NatDetectionResult } from '../types';

type DiagnosticTool = 'ping' | 'bandwidth' | 'health' | 'nat';

export default function DiagnosticsPanel() {
  const {
    pingPeer,
    testBandwidth,
    checkHealth,
    detectNat,
    loading,
    snapshot,
  } = useNetworkStore();

  const [activeTool, setActiveTool] = useState<DiagnosticTool>('ping');
  const [selectedPeerId, setSelectedPeerId] = useState('');
  const [pingResult, setPingResult] = useState<PingResult | null>(null);
  const [bandwidthResult, setBandwidthResult] = useState<BandwidthResult | null>(null);
  const [healthResult, setHealthResult] = useState<HealthReport | null>(null);
  const [natResult, setNatResult] = useState<NatDetectionResult | null>(null);
  const [pingCount, setPingCount] = useState(5);
  const [testSize, setTestSize] = useState(1048576); // 1 MB default

  // Get available peers for selection
  const availablePeers = snapshot?.nodes.filter((n) => n.peer_type !== 'self') ?? [];

  const handlePing = useCallback(async () => {
    if (!selectedPeerId) return;
    const result = await pingPeer(selectedPeerId, pingCount);
    setPingResult(result);
  }, [selectedPeerId, pingCount, pingPeer]);

  const handleBandwidth = useCallback(async () => {
    if (!selectedPeerId) return;
    const result = await testBandwidth(selectedPeerId, testSize);
    setBandwidthResult(result);
  }, [selectedPeerId, testSize, testBandwidth]);

  const handleHealth = useCallback(async () => {
    if (!selectedPeerId) return;
    const result = await checkHealth(selectedPeerId);
    setHealthResult(result);
  }, [selectedPeerId, checkHealth]);

  const handleNatDetect = useCallback(async () => {
    const result = await detectNat();
    setNatResult(result);
  }, [detectNat]);

  const tools: { id: DiagnosticTool; label: string; icon: JSX.Element }[] = [
    {
      id: 'ping',
      label: 'Ping',
      icon: (
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      ),
    },
    {
      id: 'bandwidth',
      label: 'Bandwidth',
      icon: (
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
        </svg>
      ),
    },
    {
      id: 'health',
      label: 'Health Check',
      icon: (
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      ),
    },
    {
      id: 'nat',
      label: 'NAT Detection',
      icon: (
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
        </svg>
      ),
    },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="p-3 bg-bg-secondary border-b border-slate-700">
        <h2 className="text-lg font-semibold text-white">Network Diagnostics</h2>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Tool Selection Sidebar */}
        <div className="w-48 bg-bg-secondary border-r border-slate-700 p-2">
          {tools.map((tool) => (
            <button
              key={tool.id}
              onClick={() => setActiveTool(tool.id)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors mb-1 ${
                activeTool === tool.id
                  ? 'bg-wraith-primary/20 text-wraith-primary'
                  : 'text-slate-400 hover:text-white hover:bg-slate-700'
              }`}
            >
              {tool.icon}
              {tool.label}
            </button>
          ))}
        </div>

        {/* Tool Content */}
        <div className="flex-1 p-4 overflow-y-auto">
          {activeTool === 'ping' && (
            <PingTool
              peers={availablePeers}
              selectedPeerId={selectedPeerId}
              setSelectedPeerId={setSelectedPeerId}
              pingCount={pingCount}
              setPingCount={setPingCount}
              onPing={handlePing}
              result={pingResult}
              loading={loading}
            />
          )}
          {activeTool === 'bandwidth' && (
            <BandwidthTool
              peers={availablePeers}
              selectedPeerId={selectedPeerId}
              setSelectedPeerId={setSelectedPeerId}
              testSize={testSize}
              setTestSize={setTestSize}
              onTest={handleBandwidth}
              result={bandwidthResult}
              loading={loading}
            />
          )}
          {activeTool === 'health' && (
            <HealthTool
              peers={availablePeers}
              selectedPeerId={selectedPeerId}
              setSelectedPeerId={setSelectedPeerId}
              onCheck={handleHealth}
              result={healthResult}
              loading={loading}
            />
          )}
          {activeTool === 'nat' && (
            <NatTool
              onDetect={handleNatDetect}
              result={natResult}
              loading={loading}
            />
          )}
        </div>
      </div>
    </div>
  );
}

function PeerSelector({
  peers,
  selectedPeerId,
  setSelectedPeerId,
}: {
  peers: Array<{ id: string; label: string }>;
  selectedPeerId: string;
  setSelectedPeerId: (id: string) => void;
}) {
  return (
    <div className="mb-4">
      <label className="block text-sm font-medium text-slate-300 mb-2">
        Select Peer
      </label>
      <select
        value={selectedPeerId}
        onChange={(e) => setSelectedPeerId(e.target.value)}
        className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
      >
        <option value="">Choose a peer...</option>
        {peers.map((peer) => (
          <option key={peer.id} value={peer.id}>
            {peer.label} ({peer.id.slice(0, 12)}...)
          </option>
        ))}
      </select>
    </div>
  );
}

function PingTool({
  peers,
  selectedPeerId,
  setSelectedPeerId,
  pingCount,
  setPingCount,
  onPing,
  result,
  loading,
}: {
  peers: Array<{ id: string; label: string }>;
  selectedPeerId: string;
  setSelectedPeerId: (id: string) => void;
  pingCount: number;
  setPingCount: (count: number) => void;
  onPing: () => void;
  result: PingResult | null;
  loading: boolean;
}) {
  return (
    <div className="max-w-2xl">
      <h3 className="text-lg font-semibold text-white mb-4">Ping Test</h3>
      <p className="text-sm text-slate-400 mb-6">
        Measure round-trip time latency to a peer node. This sends ICMP-like echo
        requests through the WRAITH protocol.
      </p>

      <PeerSelector
        peers={peers}
        selectedPeerId={selectedPeerId}
        setSelectedPeerId={setSelectedPeerId}
      />

      <div className="mb-4">
        <label className="block text-sm font-medium text-slate-300 mb-2">
          Ping Count
        </label>
        <select
          value={pingCount}
          onChange={(e) => setPingCount(Number(e.target.value))}
          className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
        >
          <option value={3}>3 pings</option>
          <option value={5}>5 pings</option>
          <option value={10}>10 pings</option>
          <option value={20}>20 pings</option>
        </select>
      </div>

      <button
        onClick={onPing}
        disabled={loading || !selectedPeerId}
        className="w-full px-4 py-2.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors"
      >
        {loading ? 'Pinging...' : 'Start Ping Test'}
      </button>

      {result && (
        <div className="mt-6 bg-slate-800/50 rounded-lg p-4">
          <h4 className="font-medium text-white mb-3">Results</h4>
          <div className="grid grid-cols-2 gap-4">
            <ResultStat label="Packets Sent" value={String(result.packets_sent)} />
            <ResultStat label="Packets Received" value={String(result.packets_received)} />
            <ResultStat
              label="Packet Loss"
              value={`${result.packet_loss.toFixed(1)}%`}
              status={result.packet_loss === 0 ? 'good' : result.packet_loss < 5 ? 'warn' : 'bad'}
            />
            <ResultStat label="Min Latency" value={`${result.min_rtt_ms.toFixed(2)}ms`} />
            <ResultStat label="Avg Latency" value={`${result.avg_rtt_ms.toFixed(2)}ms`} />
            <ResultStat label="Max Latency" value={`${result.max_rtt_ms.toFixed(2)}ms`} />
            <ResultStat label="Jitter" value={`${result.jitter_ms.toFixed(2)}ms`} />
          </div>

          {/* Latency Visualization */}
          <div className="mt-4 pt-4 border-t border-slate-700">
            <div className="text-sm text-slate-500 mb-2">Latency Distribution</div>
            <div className="flex items-end gap-1 h-16">
              {result.rtts_ms.map((rtt, i) => {
                const maxRtt = Math.max(...result.rtts_ms);
                const height = (rtt / maxRtt) * 100;
                return (
                  <div
                    key={i}
                    className="flex-1 bg-cyan-500 rounded-t"
                    style={{ height: `${height}%` }}
                    title={`${rtt.toFixed(2)}ms`}
                  />
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function BandwidthTool({
  peers,
  selectedPeerId,
  setSelectedPeerId,
  testSize,
  setTestSize,
  onTest,
  result,
  loading,
}: {
  peers: Array<{ id: string; label: string }>;
  selectedPeerId: string;
  setSelectedPeerId: (id: string) => void;
  testSize: number;
  setTestSize: (size: number) => void;
  onTest: () => void;
  result: BandwidthResult | null;
  loading: boolean;
}) {
  const formatSize = (bytes: number) => {
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(0)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${bytes} B`;
  };

  return (
    <div className="max-w-2xl">
      <h3 className="text-lg font-semibold text-white mb-4">Bandwidth Test</h3>
      <p className="text-sm text-slate-400 mb-6">
        Measure upload and download throughput to a peer node by transferring a
        test payload.
      </p>

      <PeerSelector
        peers={peers}
        selectedPeerId={selectedPeerId}
        setSelectedPeerId={setSelectedPeerId}
      />

      <div className="mb-4">
        <label className="block text-sm font-medium text-slate-300 mb-2">
          Test Size
        </label>
        <select
          value={testSize}
          onChange={(e) => setTestSize(Number(e.target.value))}
          className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
        >
          <option value={102400}>100 KB (Quick)</option>
          <option value={1048576}>1 MB (Standard)</option>
          <option value={10485760}>10 MB (Thorough)</option>
          <option value={52428800}>50 MB (Extended)</option>
        </select>
      </div>

      <button
        onClick={onTest}
        disabled={loading || !selectedPeerId}
        className="w-full px-4 py-2.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors"
      >
        {loading ? 'Testing...' : `Test with ${formatSize(testSize)}`}
      </button>

      {result && (
        <div className="mt-6 bg-slate-800/50 rounded-lg p-4">
          <h4 className="font-medium text-white mb-3">Results</h4>
          <div className="grid grid-cols-2 gap-4">
            <ResultStat
              label="Upload Speed"
              value={`${result.upload_mbps.toFixed(2)} Mbps`}
              highlight
            />
            <ResultStat
              label="Download Speed"
              value={`${result.download_mbps.toFixed(2)} Mbps`}
              highlight
            />
            <ResultStat label="Bytes Sent" value={formatSize(result.bytes_sent)} />
            <ResultStat label="Bytes Received" value={formatSize(result.bytes_received)} />
            <ResultStat label="Duration" value={`${result.duration_ms}ms`} />
            <ResultStat
              label="Effective Throughput"
              value={`${((result.upload_mbps + result.download_mbps) / 2).toFixed(2)} Mbps`}
            />
          </div>

          {/* Speed Comparison */}
          <div className="mt-4 pt-4 border-t border-slate-700">
            <div className="text-sm text-slate-500 mb-2">Speed Comparison</div>
            <div className="space-y-2">
              <SpeedBar label="Upload" value={result.upload_mbps} max={Math.max(result.upload_mbps, result.download_mbps)} color="cyan" />
              <SpeedBar label="Download" value={result.download_mbps} max={Math.max(result.upload_mbps, result.download_mbps)} color="violet" />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function HealthTool({
  peers,
  selectedPeerId,
  setSelectedPeerId,
  onCheck,
  result,
  loading,
}: {
  peers: Array<{ id: string; label: string }>;
  selectedPeerId: string;
  setSelectedPeerId: (id: string) => void;
  onCheck: () => void;
  result: HealthReport | null;
  loading: boolean;
}) {
  return (
    <div className="max-w-2xl">
      <h3 className="text-lg font-semibold text-white mb-4">Health Check</h3>
      <p className="text-sm text-slate-400 mb-6">
        Perform a comprehensive health assessment of a peer connection including
        latency, bandwidth, packet loss, and connection stability.
      </p>

      <PeerSelector
        peers={peers}
        selectedPeerId={selectedPeerId}
        setSelectedPeerId={setSelectedPeerId}
      />

      <button
        onClick={onCheck}
        disabled={loading || !selectedPeerId}
        className="w-full px-4 py-2.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors"
      >
        {loading ? 'Checking...' : 'Run Health Check'}
      </button>

      {result && (
        <div className="mt-6 bg-slate-800/50 rounded-lg p-4">
          {/* Overall Score */}
          <div className="flex items-center justify-between mb-4 pb-4 border-b border-slate-700">
            <h4 className="font-medium text-white">Health Score</h4>
            <div className="flex items-center gap-3">
              <div
                className={`text-3xl font-bold ${
                  result.score >= 0.8
                    ? 'text-green-400'
                    : result.score >= 0.5
                    ? 'text-yellow-400'
                    : 'text-red-400'
                }`}
              >
                {(result.score * 100).toFixed(0)}%
              </div>
              <span
                className={`px-2 py-1 rounded text-xs font-medium ${
                  result.score >= 0.8
                    ? 'bg-green-500/20 text-green-400'
                    : result.score >= 0.5
                    ? 'bg-yellow-500/20 text-yellow-400'
                    : 'bg-red-500/20 text-red-400'
                }`}
              >
                {result.score >= 0.8 ? 'Excellent' : result.score >= 0.5 ? 'Fair' : 'Poor'}
              </span>
            </div>
          </div>

          {/* Metrics */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            <ResultStat
              label="Latency"
              value={`${result.latency_ms}ms`}
              status={result.latency_ms < 50 ? 'good' : result.latency_ms < 150 ? 'warn' : 'bad'}
            />
            <ResultStat
              label="Packet Loss"
              value={`${(result.packet_loss * 100).toFixed(1)}%`}
              status={result.packet_loss < 0.01 ? 'good' : result.packet_loss < 0.05 ? 'warn' : 'bad'}
            />
            <ResultStat
              label="Bandwidth"
              value={`${result.bandwidth_mbps.toFixed(1)} Mbps`}
              status={result.bandwidth_mbps > 50 ? 'good' : result.bandwidth_mbps > 10 ? 'warn' : 'bad'}
            />
            <ResultStat
              label="Jitter"
              value={`${result.jitter_ms.toFixed(1)}ms`}
              status={result.jitter_ms < 10 ? 'good' : result.jitter_ms < 30 ? 'warn' : 'bad'}
            />
          </div>

          {/* Recommendations */}
          {result.recommendations.length > 0 && (
            <div className="pt-4 border-t border-slate-700">
              <div className="text-sm font-medium text-slate-300 mb-2">Recommendations</div>
              <ul className="space-y-1">
                {result.recommendations.map((rec, i) => (
                  <li key={i} className="flex items-start gap-2 text-sm text-slate-400">
                    <svg className="w-4 h-4 text-yellow-500 mt-0.5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                    {rec}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function NatTool({
  onDetect,
  result,
  loading,
}: {
  onDetect: () => void;
  result: NatDetectionResult | null;
  loading: boolean;
}) {
  const natTypeDescriptions: Record<string, string> = {
    none: 'No NAT detected - direct public IP address',
    full_cone: 'Full Cone NAT - Most permissive, good for P2P',
    restricted_cone: 'Restricted Cone NAT - Moderate restrictions',
    port_restricted: 'Port Restricted NAT - Tighter restrictions',
    symmetric: 'Symmetric NAT - Most restrictive, may need relay',
    unknown: 'Could not determine NAT type',
  };

  const natTypeStatus = (type: string): 'good' | 'warn' | 'bad' => {
    switch (type) {
      case 'none':
      case 'full_cone':
        return 'good';
      case 'restricted_cone':
      case 'port_restricted':
        return 'warn';
      default:
        return 'bad';
    }
  };

  return (
    <div className="max-w-2xl">
      <h3 className="text-lg font-semibold text-white mb-4">NAT Detection</h3>
      <p className="text-sm text-slate-400 mb-6">
        Detect your NAT type and public IP address. This helps determine the best
        connection strategy for peer-to-peer communication.
      </p>

      <button
        onClick={onDetect}
        disabled={loading}
        className="w-full px-4 py-2.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors"
      >
        {loading ? 'Detecting...' : 'Detect NAT Type'}
      </button>

      {result && (
        <div className="mt-6 bg-slate-800/50 rounded-lg p-4">
          <h4 className="font-medium text-white mb-3">Results</h4>

          {/* NAT Type Badge */}
          <div className="flex items-center gap-3 mb-4 pb-4 border-b border-slate-700">
            <div
              className={`px-3 py-1.5 rounded-lg text-sm font-medium ${
                natTypeStatus(result.nat_type) === 'good'
                  ? 'bg-green-500/20 text-green-400'
                  : natTypeStatus(result.nat_type) === 'warn'
                  ? 'bg-yellow-500/20 text-yellow-400'
                  : 'bg-red-500/20 text-red-400'
              }`}
            >
              {result.nat_type.replace('_', ' ').toUpperCase()}
            </div>
            <span className="text-sm text-slate-400">
              {natTypeDescriptions[result.nat_type] ?? 'Unknown NAT type'}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <ResultStat label="Public IP" value={result.public_ip ?? 'Unknown'} />
            <ResultStat label="Public Port" value={result.public_port ? String(result.public_port) : 'Unknown'} />
            <ResultStat
              label="Hairpin Support"
              value={result.hairpin_support ? 'Yes' : 'No'}
              status={result.hairpin_support ? 'good' : 'warn'}
            />
            <ResultStat
              label="Port Mapping Lifetime"
              value={result.mapping_lifetime_secs ? `${result.mapping_lifetime_secs}s` : 'N/A'}
            />
          </div>

          {/* Connectivity Implications */}
          <div className="mt-4 pt-4 border-t border-slate-700">
            <div className="text-sm font-medium text-slate-300 mb-2">Connectivity Implications</div>
            <div className="space-y-2 text-sm text-slate-400">
              {result.nat_type === 'none' && (
                <p>Direct connections possible without NAT traversal.</p>
              )}
              {result.nat_type === 'full_cone' && (
                <p>Direct connections possible after initial outbound packet.</p>
              )}
              {result.nat_type === 'restricted_cone' && (
                <p>Connections possible but may require hole punching coordination.</p>
              )}
              {result.nat_type === 'port_restricted' && (
                <p>Connections require careful coordination. STUN may be needed.</p>
              )}
              {result.nat_type === 'symmetric' && (
                <p>Direct connections difficult. TURN relay may be required for some peers.</p>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function ResultStat({
  label,
  value,
  status,
  highlight,
}: {
  label: string;
  value: string;
  status?: 'good' | 'warn' | 'bad';
  highlight?: boolean;
}) {
  const statusColors = {
    good: 'text-green-400',
    warn: 'text-yellow-400',
    bad: 'text-red-400',
  };

  return (
    <div className={`p-3 rounded-lg ${highlight ? 'bg-slate-700/70' : 'bg-slate-700/50'}`}>
      <div className="text-xs text-slate-500 mb-1">{label}</div>
      <div className={`text-sm font-medium ${status ? statusColors[status] : 'text-white'}`}>
        {value}
      </div>
    </div>
  );
}

function SpeedBar({
  label,
  value,
  max,
  color,
}: {
  label: string;
  value: number;
  max: number;
  color: 'cyan' | 'violet';
}) {
  const percent = (value / max) * 100;
  const bgColor = color === 'cyan' ? 'bg-cyan-500' : 'bg-violet-500';

  return (
    <div className="flex items-center gap-3">
      <div className="w-20 text-sm text-slate-400">{label}</div>
      <div className="flex-1 h-4 bg-slate-700 rounded-full overflow-hidden">
        <div
          className={`h-full ${bgColor} transition-all`}
          style={{ width: `${percent}%` }}
        />
      </div>
      <div className="w-24 text-sm text-white text-right">{value.toFixed(2)} Mbps</div>
    </div>
  );
}
