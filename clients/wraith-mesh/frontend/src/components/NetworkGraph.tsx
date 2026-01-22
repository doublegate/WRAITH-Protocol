// NetworkGraph Component - D3.js Force-Directed Network Visualization

import { useEffect, useRef, useCallback, useMemo } from 'react';
import * as d3 from 'd3';
import { useNetworkStore } from '../stores/networkStore';
import { useUiStore } from '../stores/uiStore';
import type { GraphNode, GraphLink, PeerType } from '../types';

// Node colors based on peer type
const nodeColors: Record<PeerType, string> = {
  self: '#3b82f6',    // Blue 500
  direct: '#22c55e',  // Green 500
  relay: '#f59e0b',   // Amber 500
  indirect: '#6b7280', // Gray 500
};

// Node sizes based on peer type
const nodeSizes: Record<PeerType, number> = {
  self: 16,
  direct: 12,
  relay: 14,
  indirect: 8,
};

export default function NetworkGraph() {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simulationRef = useRef<d3.Simulation<GraphNode, GraphLink> | null>(null);

  const { snapshot, selectedPeerId, setSelectedPeer } = useNetworkStore();
  const { showLabels, showIndirectPeers, graphLayout } = useUiStore();

  // Transform snapshot data for D3
  const graphData = useMemo(() => {
    if (!snapshot) return { nodes: [] as GraphNode[], links: [] as GraphLink[] };

    let nodes = snapshot.nodes.map((node) => ({
      ...node,
      x: undefined,
      y: undefined,
      fx: null,
      fy: null,
    })) as GraphNode[];

    let links = snapshot.links.map((link) => ({
      ...link,
    })) as GraphLink[];

    // Filter indirect peers if setting is off
    if (!showIndirectPeers) {
      const directIds = new Set(
        nodes
          .filter((n) => n.peer_type !== 'indirect')
          .map((n) => n.id)
      );
      nodes = nodes.filter((n) => n.peer_type !== 'indirect');
      links = links.filter(
        (l) =>
          directIds.has(typeof l.source === 'string' ? l.source : (l.source as GraphNode).id) &&
          directIds.has(typeof l.target === 'string' ? l.target : (l.target as GraphNode).id)
      );
    }

    return { nodes, links };
  }, [snapshot, showIndirectPeers]);

  // Initialize and update D3 simulation
  const updateGraph = useCallback(() => {
    if (!svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    const width = container.clientWidth;
    const height = container.clientHeight;

    svg.attr('width', width).attr('height', height);

    // Clear existing content
    svg.selectAll('*').remove();

    // Add zoom container
    const g = svg
      .append('g')
      .attr('class', 'zoom-container');

    // Add zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    // Create arrow marker for directed edges
    svg
      .append('defs')
      .append('marker')
      .attr('id', 'arrowhead')
      .attr('viewBox', '-5 -5 10 10')
      .attr('refX', 20)
      .attr('refY', 0)
      .attr('markerWidth', 6)
      .attr('markerHeight', 6)
      .attr('orient', 'auto')
      .append('path')
      .attr('d', 'M-5,-5L5,0L-5,5')
      .attr('fill', '#64748b');

    // Initialize simulation based on layout
    const simulation = d3.forceSimulation<GraphNode>(graphData.nodes);

    if (graphLayout === 'force') {
      simulation
        .force('link', d3.forceLink<GraphNode, GraphLink>(graphData.links)
          .id((d) => d.id)
          .distance((d) => {
            const strength = typeof d.strength === 'number' ? d.strength : 0.5;
            return 100 + (1 - strength) * 100;
          })
        )
        .force('charge', d3.forceManyBody().strength(-300))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('collision', d3.forceCollide().radius(40));
    } else if (graphLayout === 'radial') {
      // Find self node for center
      const selfNode = graphData.nodes.find((n) => n.peer_type === 'self');
      if (selfNode) {
        selfNode.fx = width / 2;
        selfNode.fy = height / 2;
      }

      simulation
        .force('link', d3.forceLink<GraphNode, GraphLink>(graphData.links)
          .id((d) => d.id)
          .distance(150)
        )
        .force('charge', d3.forceManyBody().strength(-200))
        .force('radial', d3.forceRadial<GraphNode>(
          (d) => d.peer_type === 'self' ? 0 : d.peer_type === 'direct' ? 150 : d.peer_type === 'relay' ? 200 : 300,
          width / 2,
          height / 2
        ).strength(0.8));
    } else {
      // Tree layout - simple force with stronger links
      simulation
        .force('link', d3.forceLink<GraphNode, GraphLink>(graphData.links)
          .id((d) => d.id)
          .distance(80)
          .strength(1)
        )
        .force('charge', d3.forceManyBody().strength(-400))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('y', d3.forceY<GraphNode>((d) => {
          if (d.peer_type === 'self') return height / 4;
          if (d.peer_type === 'direct' || d.peer_type === 'relay') return height / 2;
          return (height * 3) / 4;
        }).strength(0.3));
    }

    simulationRef.current = simulation;

    // Create links
    const link = g
      .append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(graphData.links)
      .enter()
      .append('line')
      .attr('class', 'link')
      .attr('stroke', (d) => {
        const loss = d.packet_loss;
        if (loss > 0.05) return '#ef4444'; // Red for high loss
        if (loss > 0.02) return '#f59e0b'; // Amber for medium loss
        return '#4b5563'; // Gray for normal
      })
      .attr('stroke-width', (d) => Math.max(1, d.strength * 3))
      .attr('stroke-opacity', 0.6);

    // Create nodes
    const node = g
      .append('g')
      .attr('class', 'nodes')
      .selectAll('g')
      .data(graphData.nodes)
      .enter()
      .append('g')
      .attr('class', 'node')
      .style('cursor', 'pointer')
      .on('click', (_event, d) => {
        setSelectedPeer(selectedPeerId === d.id ? null : d.id);
      })
      .call(
        d3.drag<SVGGElement, GraphNode>()
          .on('start', (event, d) => {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
          })
          .on('drag', (event, d) => {
            d.fx = event.x;
            d.fy = event.y;
          })
          .on('end', (event, d) => {
            if (!event.active) simulation.alphaTarget(0);
            if (d.peer_type !== 'self' || graphLayout !== 'radial') {
              d.fx = null;
              d.fy = null;
            }
          })
      );

    // Node circles
    node
      .append('circle')
      .attr('r', (d) => nodeSizes[d.peer_type])
      .attr('fill', (d) => nodeColors[d.peer_type])
      .attr('stroke', (d) => (d.id === selectedPeerId ? '#fff' : 'transparent'))
      .attr('stroke-width', 3)
      .style('filter', (d) => d.peer_type === 'self' ? 'drop-shadow(0 0 8px rgba(59, 130, 246, 0.5))' : 'none');

    // Node labels
    if (showLabels) {
      node
        .append('text')
        .attr('class', 'label')
        .attr('dy', (d) => nodeSizes[d.peer_type] + 14)
        .attr('text-anchor', 'middle')
        .attr('fill', '#e2e8f0')
        .attr('font-size', '10px')
        .text((d) => d.label);
    }

    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d) => (d.source as GraphNode).x ?? 0)
        .attr('y1', (d) => (d.source as GraphNode).y ?? 0)
        .attr('x2', (d) => (d.target as GraphNode).x ?? 0)
        .attr('y2', (d) => (d.target as GraphNode).y ?? 0);

      node.attr('transform', (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    // Initial zoom to fit
    setTimeout(() => {
      const bounds = g.node()?.getBBox();
      if (bounds) {
        const scale = Math.min(
          width / (bounds.width + 100),
          height / (bounds.height + 100),
          1.5
        );
        const translateX = (width - bounds.width * scale) / 2 - bounds.x * scale;
        const translateY = (height - bounds.height * scale) / 2 - bounds.y * scale;

        svg
          .transition()
          .duration(750)
          .call(
            zoom.transform,
            d3.zoomIdentity.translate(translateX, translateY).scale(scale)
          );
      }
    }, 500);
  }, [graphData, graphLayout, selectedPeerId, setSelectedPeer, showLabels]);

  // Handle resize
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver(() => {
      updateGraph();
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, [updateGraph]);

  // Update graph when data changes
  useEffect(() => {
    updateGraph();
  }, [updateGraph]);

  // Cleanup simulation on unmount
  useEffect(() => {
    return () => {
      simulationRef.current?.stop();
    };
  }, []);

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between p-3 bg-bg-secondary border-b border-slate-700">
        <div className="flex items-center gap-4">
          <h2 className="text-lg font-semibold text-white">Network Topology</h2>
          <div className="flex items-center gap-4 text-sm">
            <Legend color={nodeColors.self} label="Self" />
            <Legend color={nodeColors.direct} label="Direct" />
            <Legend color={nodeColors.relay} label="Relay" />
            <Legend color={nodeColors.indirect} label="Indirect (DHT)" />
          </div>
        </div>
        <div className="text-sm text-slate-400">
          Drag to pan, scroll to zoom, click nodes to select
        </div>
      </div>

      {/* Graph Container */}
      <div ref={containerRef} className="flex-1 bg-bg-primary">
        <svg ref={svgRef} className="network-graph w-full h-full" />
      </div>

      {/* Selected Node Details */}
      {selectedPeerId && (
        <SelectedNodePanel
          nodeId={selectedPeerId}
          onClose={() => setSelectedPeer(null)}
        />
      )}
    </div>
  );
}

function Legend({ color, label }: { color: string; label: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <div className="w-3 h-3 rounded-full" style={{ backgroundColor: color }} />
      <span className="text-slate-400">{label}</span>
    </div>
  );
}

function SelectedNodePanel({
  nodeId,
  onClose,
}: {
  nodeId: string;
  onClose: () => void;
}) {
  const { snapshot, lastHealthReport, checkHealth, loading } = useNetworkStore();

  const node = snapshot?.nodes.find((n) => n.id === nodeId);
  const links = snapshot?.links.filter(
    (l) =>
      (typeof l.source === 'string' ? l.source : (l.source as GraphNode).id) === nodeId ||
      (typeof l.target === 'string' ? l.target : (l.target as GraphNode).id) === nodeId
  );

  if (!node) return null;

  const avgLatency =
    links && links.length > 0
      ? links.reduce((sum, l) => sum + l.latency_ms, 0) / links.length
      : 0;

  const avgBandwidth =
    links && links.length > 0
      ? links.reduce((sum, l) => sum + l.bandwidth_mbps, 0) / links.length
      : 0;

  return (
    <div className="bg-bg-secondary border-t border-slate-700 p-4">
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-2">
            <div
              className="w-4 h-4 rounded-full"
              style={{ backgroundColor: nodeColors[node.peer_type] }}
            />
            <h3 className="font-semibold text-white">{node.label}</h3>
            <span className="text-xs text-slate-500 capitalize">({node.peer_type})</span>
          </div>
          <p className="text-sm text-slate-400 font-mono mt-1">{nodeId}</p>
        </div>
        <button
          onClick={onClose}
          className="text-slate-400 hover:text-white transition-colors"
          aria-label="Close panel"
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="grid grid-cols-4 gap-4 mt-4">
        <StatBox label="Connections" value={String(links?.length ?? 0)} />
        <StatBox label="Avg Latency" value={`${avgLatency.toFixed(0)}ms`} />
        <StatBox label="Avg Bandwidth" value={`${avgBandwidth.toFixed(1)} Mbps`} />
        <StatBox
          label="Last Seen"
          value={new Date(node.last_seen * 1000).toLocaleTimeString()}
        />
      </div>

      {node.peer_type !== 'self' && (
        <div className="mt-4 flex items-center gap-2">
          <button
            onClick={() => checkHealth(nodeId)}
            disabled={loading}
            className="px-3 py-1.5 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 rounded-lg text-sm text-white font-medium transition-colors"
          >
            {loading ? 'Checking...' : 'Check Health'}
          </button>
          {lastHealthReport?.peer_id === nodeId && (
            <span
              className={`text-sm ${
                lastHealthReport.score >= 0.8
                  ? 'text-green-400'
                  : lastHealthReport.score >= 0.5
                  ? 'text-yellow-400'
                  : 'text-red-400'
              }`}
            >
              Health Score: {(lastHealthReport.score * 100).toFixed(0)}%
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-slate-700/50 rounded-lg p-2">
      <div className="text-xs text-slate-500">{label}</div>
      <div className="text-sm font-medium text-white">{value}</div>
    </div>
  );
}
