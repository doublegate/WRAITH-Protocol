// StatsDashboard Component - Real-time Network Statistics

import { useMemo } from 'react';
import { useNetworkStore } from '../stores/networkStore';

export default function StatsDashboard() {
  const { snapshot, metricsHistory } = useNetworkStore();

  // Calculate aggregate statistics
  const stats = useMemo(() => {
    if (!snapshot) {
      return {
        totalNodes: 0,
        directPeers: 0,
        relayServers: 0,
        dhtNodes: 0,
        totalLinks: 0,
        avgLatency: 0,
        totalBandwidth: 0,
        avgPacketLoss: 0,
        healthScore: 0,
      };
    }

    const directPeers = snapshot.nodes.filter((n) => n.peer_type === 'direct').length;
    const relayServers = snapshot.nodes.filter((n) => n.peer_type === 'relay').length;
    const dhtNodes = snapshot.nodes.filter((n) => n.peer_type === 'indirect').length;

    const avgLatency =
      snapshot.links.length > 0
        ? snapshot.links.reduce((sum, l) => sum + l.latency_ms, 0) / snapshot.links.length
        : 0;

    const totalBandwidth = snapshot.links.reduce((sum, l) => sum + l.bandwidth_mbps, 0);

    const avgPacketLoss =
      snapshot.links.length > 0
        ? snapshot.links.reduce((sum, l) => sum + l.packet_loss, 0) / snapshot.links.length
        : 0;

    return {
      totalNodes: snapshot.nodes.length,
      directPeers,
      relayServers,
      dhtNodes,
      totalLinks: snapshot.links.length,
      avgLatency,
      totalBandwidth,
      avgPacketLoss,
      healthScore: snapshot.health_score,
    };
  }, [snapshot]);

  // Calculate trend from metrics history
  const trends = useMemo(() => {
    if (metricsHistory.length < 2) {
      return { latency: 0, bandwidth: 0, peerCount: 0 };
    }

    const recent = metricsHistory.slice(-5);
    const older = metricsHistory.slice(-10, -5);

    const avgRecent = {
      latency: recent.reduce((s, m) => s + m.avg_latency_ms, 0) / recent.length,
      bandwidth: recent.reduce((s, m) => s + m.total_bandwidth_mbps, 0) / recent.length,
      peerCount: recent.reduce((s, m) => s + m.peer_count, 0) / recent.length,
    };

    const avgOlder =
      older.length > 0
        ? {
            latency: older.reduce((s, m) => s + m.avg_latency_ms, 0) / older.length,
            bandwidth: older.reduce((s, m) => s + m.total_bandwidth_mbps, 0) / older.length,
            peerCount: older.reduce((s, m) => s + m.peer_count, 0) / older.length,
          }
        : avgRecent;

    return {
      latency: avgRecent.latency - avgOlder.latency,
      bandwidth: avgRecent.bandwidth - avgOlder.bandwidth,
      peerCount: avgRecent.peerCount - avgOlder.peerCount,
    };
  }, [metricsHistory]);

  return (
    <div className="h-full overflow-auto p-4 space-y-6">
      {/* Overview Cards */}
      <div className="grid grid-cols-4 gap-4">
        <MetricCard
          label="Network Health"
          value={`${(stats.healthScore * 100).toFixed(0)}%`}
          icon={<HealthIcon className="w-8 h-8" />}
          color={
            stats.healthScore >= 0.8
              ? 'green'
              : stats.healthScore >= 0.5
              ? 'yellow'
              : 'red'
          }
          progress={stats.healthScore * 100}
        />
        <MetricCard
          label="Total Nodes"
          value={String(stats.totalNodes)}
          icon={<NodesIcon className="w-8 h-8" />}
          color="blue"
          trend={trends.peerCount}
        />
        <MetricCard
          label="Avg Latency"
          value={`${stats.avgLatency.toFixed(0)}ms`}
          icon={<LatencyIcon className="w-8 h-8" />}
          color={stats.avgLatency < 100 ? 'green' : stats.avgLatency < 200 ? 'yellow' : 'red'}
          trend={-trends.latency}
          trendInverse
        />
        <MetricCard
          label="Total Bandwidth"
          value={`${stats.totalBandwidth.toFixed(1)} Mbps`}
          icon={<BandwidthIcon className="w-8 h-8" />}
          color="cyan"
          trend={trends.bandwidth}
        />
      </div>

      {/* Node Breakdown */}
      <div className="bg-bg-secondary rounded-lg border border-slate-700 p-4">
        <h3 className="text-lg font-semibold text-white mb-4">Node Breakdown</h3>
        <div className="grid grid-cols-4 gap-4">
          <NodeTypeCard
            type="Self"
            count={1}
            color="#3b82f6"
            description="Local node"
          />
          <NodeTypeCard
            type="Direct Peers"
            count={stats.directPeers}
            color="#22c55e"
            description="Directly connected"
          />
          <NodeTypeCard
            type="Relay Servers"
            count={stats.relayServers}
            color="#f59e0b"
            description="Network relays"
          />
          <NodeTypeCard
            type="DHT Nodes"
            count={stats.dhtNodes}
            color="#6b7280"
            description="Discovered via DHT"
          />
        </div>
      </div>

      {/* DHT Statistics */}
      <div className="bg-bg-secondary rounded-lg border border-slate-700 p-4">
        <h3 className="text-lg font-semibold text-white mb-4">DHT Statistics</h3>
        <div className="grid grid-cols-5 gap-4">
          <StatItem
            label="Total DHT Nodes"
            value={String(snapshot?.dht_stats.total_nodes ?? 0)}
          />
          <StatItem
            label="Routing Table Size"
            value={String(snapshot?.dht_stats.routing_table_size ?? 0)}
          />
          <StatItem
            label="Stored Keys"
            value={String(snapshot?.dht_stats.stored_keys ?? 0)}
          />
          <StatItem
            label="Lookups (1h)"
            value={String(snapshot?.dht_stats.lookup_count_1h ?? 0)}
          />
          <StatItem
            label="Avg Lookup Latency"
            value={`${snapshot?.dht_stats.avg_lookup_latency_ms.toFixed(0) ?? 0}ms`}
          />
        </div>
      </div>

      {/* Connection Quality */}
      <div className="bg-bg-secondary rounded-lg border border-slate-700 p-4">
        <h3 className="text-lg font-semibold text-white mb-4">Connection Quality</h3>
        <div className="space-y-4">
          <QualityBar
            label="Latency"
            value={stats.avgLatency}
            max={300}
            unit="ms"
            thresholds={{ good: 50, warning: 150 }}
            inverse
          />
          <QualityBar
            label="Packet Loss"
            value={stats.avgPacketLoss * 100}
            max={10}
            unit="%"
            thresholds={{ good: 1, warning: 5 }}
            inverse
          />
          <QualityBar
            label="Link Strength"
            value={
              snapshot?.links.length
                ? (snapshot.links.reduce((s, l) => s + l.strength, 0) /
                    snapshot.links.length) *
                  100
                : 0
            }
            max={100}
            unit="%"
            thresholds={{ good: 80, warning: 50 }}
          />
        </div>
      </div>

      {/* Metrics History Chart */}
      <div className="bg-bg-secondary rounded-lg border border-slate-700 p-4">
        <h3 className="text-lg font-semibold text-white mb-4">Metrics History</h3>
        <MetricsChart data={metricsHistory} />
      </div>
    </div>
  );
}

// Metric Card Component
function MetricCard({
  label,
  value,
  icon,
  color,
  progress,
  trend,
  trendInverse,
}: {
  label: string;
  value: string;
  icon: JSX.Element;
  color: 'green' | 'yellow' | 'red' | 'blue' | 'cyan';
  progress?: number;
  trend?: number;
  trendInverse?: boolean;
}) {
  const colorClasses = {
    green: 'text-green-400 bg-green-400/10',
    yellow: 'text-yellow-400 bg-yellow-400/10',
    red: 'text-red-400 bg-red-400/10',
    blue: 'text-blue-400 bg-blue-400/10',
    cyan: 'text-cyan-400 bg-cyan-400/10',
  };

  const trendPositive = trendInverse ? (trend ?? 0) < 0 : (trend ?? 0) > 0;

  return (
    <div className="metric-card bg-bg-secondary rounded-lg border border-slate-700 p-4">
      <div className="flex items-start justify-between mb-3">
        <div className={`p-2 rounded-lg ${colorClasses[color]}`}>{icon}</div>
        {trend !== undefined && trend !== 0 && (
          <div
            className={`flex items-center gap-1 text-xs ${
              trendPositive ? 'text-green-400' : 'text-red-400'
            }`}
          >
            <svg
              className={`w-3 h-3 ${trendPositive ? '' : 'rotate-180'}`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 10l7-7m0 0l7 7m-7-7v18"
              />
            </svg>
            {Math.abs(trend).toFixed(1)}
          </div>
        )}
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
      <div className="text-sm text-slate-400 mt-1">{label}</div>
      {progress !== undefined && (
        <div className="mt-3 h-1.5 bg-slate-700 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${
              color === 'green'
                ? 'bg-green-500'
                : color === 'yellow'
                ? 'bg-yellow-500'
                : color === 'red'
                ? 'bg-red-500'
                : color === 'cyan'
                ? 'bg-cyan-500'
                : 'bg-blue-500'
            }`}
            style={{ width: `${Math.min(100, progress)}%` }}
          />
        </div>
      )}
    </div>
  );
}

// Node Type Card
function NodeTypeCard({
  type,
  count,
  color,
  description,
}: {
  type: string;
  count: number;
  color: string;
  description: string;
}) {
  return (
    <div className="flex items-center gap-3 p-3 bg-slate-700/30 rounded-lg">
      <div className="w-10 h-10 rounded-full flex items-center justify-center" style={{ backgroundColor: `${color}20` }}>
        <div className="w-4 h-4 rounded-full" style={{ backgroundColor: color }} />
      </div>
      <div>
        <div className="text-xl font-bold text-white">{count}</div>
        <div className="text-sm text-slate-400">{type}</div>
        <div className="text-xs text-slate-500">{description}</div>
      </div>
    </div>
  );
}

// Stat Item
function StatItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-center p-3 bg-slate-700/30 rounded-lg">
      <div className="text-xl font-bold text-white">{value}</div>
      <div className="text-xs text-slate-500 mt-1">{label}</div>
    </div>
  );
}

// Quality Bar
function QualityBar({
  label,
  value,
  max,
  unit,
  thresholds,
  inverse,
}: {
  label: string;
  value: number;
  max: number;
  unit: string;
  thresholds: { good: number; warning: number };
  inverse?: boolean;
}) {
  const percent = Math.min(100, (value / max) * 100);
  const isGood = inverse ? value <= thresholds.good : value >= thresholds.good;
  const isWarning = inverse
    ? value > thresholds.good && value <= thresholds.warning
    : value >= thresholds.warning && value < thresholds.good;

  const barColor = isGood ? 'bg-green-500' : isWarning ? 'bg-yellow-500' : 'bg-red-500';

  return (
    <div>
      <div className="flex items-center justify-between text-sm mb-1">
        <span className="text-slate-400">{label}</span>
        <span className={isGood ? 'text-green-400' : isWarning ? 'text-yellow-400' : 'text-red-400'}>
          {value.toFixed(1)}{unit}
        </span>
      </div>
      <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
        <div
          className={`h-full transition-all duration-300 ${barColor}`}
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
}

// Simple Metrics Chart (using SVG)
function MetricsChart({ data }: { data: Array<{ timestamp: number; avg_latency_ms: number; peer_count: number }> }) {
  if (data.length < 2) {
    return (
      <div className="h-40 flex items-center justify-center text-slate-500">
        Collecting metrics data...
      </div>
    );
  }

  const width = 800;
  const height = 160;
  const padding = { top: 20, right: 20, bottom: 30, left: 50 };

  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  const maxLatency = Math.max(...data.map((d) => d.avg_latency_ms), 100);
  const maxPeers = Math.max(...data.map((d) => d.peer_count), 10);

  const xScale = (i: number) => padding.left + (i / (data.length - 1)) * chartWidth;
  const latencyScale = (v: number) => padding.top + chartHeight - (v / maxLatency) * chartHeight;
  const peerScale = (v: number) => padding.top + chartHeight - (v / maxPeers) * chartHeight;

  const latencyPath = data
    .map((d, i) => `${i === 0 ? 'M' : 'L'} ${xScale(i)} ${latencyScale(d.avg_latency_ms)}`)
    .join(' ');

  const peerPath = data
    .map((d, i) => `${i === 0 ? 'M' : 'L'} ${xScale(i)} ${peerScale(d.peer_count)}`)
    .join(' ');

  return (
    <div className="overflow-x-auto">
      <svg viewBox={`0 0 ${width} ${height}`} className="w-full min-w-[600px]">
        {/* Grid lines */}
        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => (
          <line
            key={ratio}
            x1={padding.left}
            y1={padding.top + chartHeight * (1 - ratio)}
            x2={width - padding.right}
            y2={padding.top + chartHeight * (1 - ratio)}
            stroke="#374151"
            strokeDasharray="4,4"
          />
        ))}

        {/* Y-axis labels */}
        <text x={padding.left - 5} y={padding.top} textAnchor="end" className="fill-slate-500 text-[10px]">
          {maxLatency.toFixed(0)}ms
        </text>
        <text
          x={padding.left - 5}
          y={padding.top + chartHeight}
          textAnchor="end"
          className="fill-slate-500 text-[10px]"
        >
          0ms
        </text>

        {/* Latency line */}
        <path d={latencyPath} fill="none" stroke="#f59e0b" strokeWidth={2} />

        {/* Peer count line */}
        <path d={peerPath} fill="none" stroke="#22c55e" strokeWidth={2} />

        {/* Legend */}
        <g transform={`translate(${padding.left + 10}, ${padding.top})`}>
          <rect x={0} y={0} width={80} height={40} fill="#1e293b" rx={4} />
          <line x1={8} y1={12} x2={28} y2={12} stroke="#f59e0b" strokeWidth={2} />
          <text x={32} y={16} className="fill-slate-300 text-[10px]">Latency</text>
          <line x1={8} y1={28} x2={28} y2={28} stroke="#22c55e" strokeWidth={2} />
          <text x={32} y={32} className="fill-slate-300 text-[10px]">Peers</text>
        </g>
      </svg>
    </div>
  );
}

// Icons
function HealthIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
    </svg>
  );
}

function NodesIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function LatencyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function BandwidthIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}
