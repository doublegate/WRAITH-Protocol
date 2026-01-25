interface Node {
  id: string;
  name: string;
  type: 'server' | 'beacon';
  status: string;
  x: number;
  y: number;
}

interface NetworkGraphProps {
  implants: { id: string; hostname: string; status: string }[];
}

export const NetworkGraph = ({ implants }: NetworkGraphProps) => {
  const serverX = 400;
  const serverY = 100;

  const nodes: Node[] = [
    { id: 'server', name: 'TEAM SERVER', type: 'server', status: 'active', x: serverX, y: serverY },
    ...implants.map((imp, i) => ({
      id: imp.id,
      name: imp.hostname,
      type: 'beacon' as const,
      status: imp.status,
      x: 100 + (i * 150) % 600,
      y: 300 + Math.floor(i / 4) * 100,
    }))
  ];

  return (
    <div className="w-full h-full bg-slate-950 rounded border border-slate-800 relative overflow-hidden">
      <div className="absolute top-2 left-3 text-[10px] text-slate-500 uppercase tracking-widest font-mono">Topology Visualization</div>
      <svg width="100%" height="100%" viewBox="0 0 800 600">
        {/* Links */}
        {nodes.filter(n => n.id !== 'server').map(n => (
          <line
            key={`link-${n.id}`}
            x1={serverX}
            y1={serverY}
            x2={n.x}
            y2={n.y}
            stroke="#1e293b"
            strokeWidth="1"
            strokeDasharray="4 2"
          />
        ))}

        {/* Nodes */}
        {nodes.map(n => (
          <g key={n.id}>
            <circle
              cx={n.x}
              cy={n.y}
              r={n.type === 'server' ? 12 : 8}
              fill={n.type === 'server' ? '#ef4444' : (n.status === 'active' ? '#22c55e' : '#64748b')}
              className={n.status === 'active' ? 'animate-pulse' : ''}
            />
            <text
              x={n.x}
              y={n.y + 20}
              textAnchor="middle"
              className="text-[10px] font-mono fill-slate-400"
            >
              {n.name.toUpperCase()}
            </text>
          </g>
        ))}
      </svg>
    </div>
  );
};
