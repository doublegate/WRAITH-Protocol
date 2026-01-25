import { useMemo, useState } from 'react';

interface Node {
  id: string;
  name: string;
  type: 'server' | 'beacon';
  status: string;
  x: number;
  y: number;
}

interface NetworkGraphProps {
  implants: { id: string; hostname: string; status: string; internal_ip?: string }[];
}

export const NetworkGraph = ({ implants }: NetworkGraphProps) => {
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  // Calculate positions using a radial layout
  const nodes: Node[] = useMemo(() => {
    const centerX = 400;
    const centerY = 200;
    const radius = 150;

    const serverNode: Node = {
      id: 'server',
      name: 'TEAM SERVER',
      type: 'server',
      status: 'active',
      x: centerX,
      y: centerY,
    };

    // Distribute beacons in a circle around the server
    const beaconNodes: Node[] = implants.map((imp, i) => {
      const angle = (2 * Math.PI * i) / Math.max(implants.length, 1) - Math.PI / 2;
      return {
        id: imp.id,
        name: imp.hostname || 'Unknown',
        type: 'beacon' as const,
        status: imp.status,
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      };
    });

    return [serverNode, ...beaconNodes];
  }, [implants]);

  const getNodeColor = (node: Node) => {
    if (node.type === 'server') return '#ef4444'; // red-500
    if (node.status === 'active') return '#22c55e'; // green-500
    if (node.status === 'dormant') return '#eab308'; // yellow-500
    return '#64748b'; // slate-500
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'active': return 'ONLINE';
      case 'dormant': return 'DORMANT';
      case 'killed': return 'KILLED';
      default: return status.toUpperCase();
    }
  };

  const selectedImplant = implants.find(i => i.id === selectedNode);

  return (
    <div className="w-full h-full bg-slate-950 rounded border border-slate-800 relative overflow-hidden">
      <div className="absolute top-2 left-3 text-[10px] text-slate-500 uppercase tracking-widest font-mono">
        Topology Visualization
      </div>

      {/* Stats overlay */}
      <div className="absolute top-2 right-3 text-[10px] text-slate-500 font-mono">
        <span className="text-green-500">{implants.filter(i => i.status === 'active').length}</span> active /
        <span className="text-slate-400"> {implants.length}</span> total
      </div>

      {/* Legend */}
      <div className="absolute bottom-2 left-3 flex gap-4 text-[9px] text-slate-500 font-mono">
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-red-500"></div>
          <span>Server</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-green-500"></div>
          <span>Active</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-yellow-500"></div>
          <span>Dormant</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-slate-500"></div>
          <span>Offline</span>
        </div>
      </div>

      {/* Selected node info panel */}
      {selectedNode && selectedNode !== 'server' && selectedImplant && (
        <div className="absolute bottom-2 right-3 bg-slate-900 border border-slate-700 rounded p-2 text-[10px] font-mono min-w-[150px]">
          <div className="text-slate-400 uppercase tracking-wider mb-1">Selected Beacon</div>
          <div className="text-white">{selectedImplant.hostname}</div>
          <div className="text-slate-500">{selectedImplant.internal_ip || 'N/A'}</div>
          <div className={`${selectedImplant.status === 'active' ? 'text-green-500' : 'text-slate-500'}`}>
            {getStatusLabel(selectedImplant.status)}
          </div>
        </div>
      )}

      <svg width="100%" height="100%" viewBox="0 0 800 400" preserveAspectRatio="xMidYMid meet">
        {/* Grid background */}
        <defs>
          <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
            <path d="M 40 0 L 0 0 0 40" fill="none" stroke="#1e293b" strokeWidth="0.5" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#grid)" />

        {/* Links with glow effect */}
        {nodes.filter(n => n.id !== 'server').map(n => {
          const serverNode = nodes[0];
          const isHighlighted = hoveredNode === n.id || selectedNode === n.id;
          return (
            <g key={`link-${n.id}`}>
              {/* Glow effect */}
              {isHighlighted && (
                <line
                  x1={serverNode.x}
                  y1={serverNode.y}
                  x2={n.x}
                  y2={n.y}
                  stroke={getNodeColor(n)}
                  strokeWidth="4"
                  strokeOpacity="0.3"
                />
              )}
              <line
                x1={serverNode.x}
                y1={serverNode.y}
                x2={n.x}
                y2={n.y}
                stroke={isHighlighted ? getNodeColor(n) : '#1e293b'}
                strokeWidth={isHighlighted ? 2 : 1}
                strokeDasharray={n.status === 'active' ? 'none' : '4 2'}
              />
              {/* Data flow animation for active connections */}
              {n.status === 'active' && (
                <circle r="2" fill={getNodeColor(n)}>
                  <animateMotion
                    dur="2s"
                    repeatCount="indefinite"
                    path={`M${serverNode.x},${serverNode.y} L${n.x},${n.y}`}
                  />
                </circle>
              )}
            </g>
          );
        })}

        {/* Nodes */}
        {nodes.map(n => {
          const isHovered = hoveredNode === n.id;
          const isSelected = selectedNode === n.id;
          const radius = n.type === 'server' ? 16 : 10;
          const displayRadius = isHovered || isSelected ? radius + 4 : radius;

          return (
            <g
              key={n.id}
              style={{ cursor: 'pointer' }}
              onMouseEnter={() => setHoveredNode(n.id)}
              onMouseLeave={() => setHoveredNode(null)}
              onClick={() => setSelectedNode(n.id === selectedNode ? null : n.id)}
            >
              {/* Selection ring */}
              {isSelected && (
                <circle
                  cx={n.x}
                  cy={n.y}
                  r={displayRadius + 6}
                  fill="none"
                  stroke={getNodeColor(n)}
                  strokeWidth="2"
                  strokeDasharray="4 2"
                  className="animate-spin"
                  style={{ animationDuration: '3s' }}
                />
              )}

              {/* Outer glow */}
              {(isHovered || (n.status === 'active' && n.type !== 'server')) && (
                <circle
                  cx={n.x}
                  cy={n.y}
                  r={displayRadius + 2}
                  fill={getNodeColor(n)}
                  fillOpacity={0.2}
                />
              )}

              {/* Main node circle */}
              <circle
                cx={n.x}
                cy={n.y}
                r={displayRadius}
                fill={getNodeColor(n)}
                className={n.status === 'active' ? 'animate-pulse' : ''}
              />

              {/* Inner highlight */}
              <circle
                cx={n.x - 2}
                cy={n.y - 2}
                r={displayRadius * 0.3}
                fill="white"
                fillOpacity={0.3}
              />

              {/* Node label */}
              <text
                x={n.x}
                y={n.y + displayRadius + 12}
                textAnchor="middle"
                className="text-[9px] font-mono fill-slate-400"
                style={{ pointerEvents: 'none' }}
              >
                {n.name.toUpperCase().substring(0, 12)}
                {n.name.length > 12 ? '...' : ''}
              </text>

              {/* Status indicator for beacons */}
              {n.type === 'beacon' && (
                <text
                  x={n.x}
                  y={n.y + displayRadius + 22}
                  textAnchor="middle"
                  className={`text-[8px] font-mono ${n.status === 'active' ? 'fill-green-600' : 'fill-slate-600'}`}
                  style={{ pointerEvents: 'none' }}
                >
                  {getStatusLabel(n.status)}
                </text>
              )}
            </g>
          );
        })}
      </svg>
    </div>
  );
};
