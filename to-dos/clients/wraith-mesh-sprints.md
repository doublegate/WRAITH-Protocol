# WRAITH-Mesh Client - Sprint Planning

**Client Name:** WRAITH-Mesh
**Tier:** 3 (Lower Priority)
**Description:** Network topology visualization and diagnostics
**Target Platforms:** Windows, macOS, Linux, Web
**UI Framework:** Electron + React + D3.js/Three.js
**Timeline:** 4 weeks (1 sprint)
**Total Story Points:** 52

---

## Overview

WRAITH-Mesh provides real-time visualization of the WRAITH network topology, peer connections, and traffic flows. Essential for network operators, researchers, and advanced users who need to understand and debug the decentralized network.

**Core Value Proposition:**
- Real-time network graph visualization
- Peer connection status and statistics
- DHT routing table inspection
- Traffic flow analysis
- Network diagnostics and health monitoring

---

## Success Criteria

**Visualization:**
- [x] Renders 1,000+ node networks smoothly (30+ FPS)
- [x] Real-time updates with <100ms latency
- [x] 3D and 2D visualization modes
- [x] Interactive node inspection

**Diagnostics:**
- [x] Connection latency measurement
- [x] Bandwidth utilization graphs
- [x] DHT lookup path visualization
- [x] Relay server status monitoring

---

## Sprint 1: Complete Implementation (Weeks 49-52)

### S1.1: Network Graph Renderer (13 points)

**Task:** Build force-directed graph visualization with D3.js/Three.js.

**Implementation:**
```tsx
// src/components/NetworkGraph.tsx
import React, { useEffect, useRef } from 'react';
import * as d3 from 'd3';

interface Node {
  id: string;
  label: string;
  type: 'self' | 'direct' | 'relay' | 'indirect';
  x?: number;
  y?: number;
}

interface Link {
  source: string;
  target: string;
  strength: number; // 0-1
  latency: number; // ms
}

interface NetworkGraphProps {
  nodes: Node[];
  links: Link[];
  onNodeClick?: (node: Node) => void;
}

export function NetworkGraph({ nodes, links, onNodeClick }: NetworkGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current) return;

    const width = 1200;
    const height = 800;

    const svg = d3.select(svgRef.current)
      .attr('width', width)
      .attr('height', height);

    svg.selectAll('*').remove(); // Clear previous render

    // Create force simulation
    const simulation = d3.forceSimulation<Node>(nodes)
      .force('link', d3.forceLink<Node, Link>(links)
        .id(d => d.id)
        .distance(d => 100 + d.latency)
      )
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(30));

    // Draw links
    const link = svg.append('g')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', d => getStrokeColor(d.strength))
      .attr('stroke-width', d => 1 + d.strength * 3)
      .attr('stroke-opacity', 0.6);

    // Draw nodes
    const node = svg.append('g')
      .selectAll('circle')
      .data(nodes)
      .join('circle')
      .attr('r', d => d.type === 'self' ? 12 : 8)
      .attr('fill', d => getNodeColor(d.type))
      .call(drag(simulation))
      .on('click', (event, d) => onNodeClick?.(d));

    // Draw labels
    const label = svg.append('g')
      .selectAll('text')
      .data(nodes)
      .join('text')
      .text(d => d.label)
      .attr('font-size', 10)
      .attr('dx', 12)
      .attr('dy', 4);

    // Update positions on each tick
    simulation.on('tick', () => {
      link
        .attr('x1', d => (d.source as any).x)
        .attr('y1', d => (d.source as any).y)
        .attr('x2', d => (d.target as any).x)
        .attr('y2', d => (d.target as any).y);

      node
        .attr('cx', d => d.x!)
        .attr('cy', d => d.y!);

      label
        .attr('x', d => d.x!)
        .attr('y', d => d.y!);
    });

    return () => {
      simulation.stop();
    };
  }, [nodes, links]);

  return <svg ref={svgRef} />;
}

function getNodeColor(type: string): string {
  switch (type) {
    case 'self': return '#2196F3';
    case 'direct': return '#4CAF50';
    case 'relay': return '#FF9800';
    case 'indirect': return '#9E9E9E';
    default: return '#000000';
  }
}

function getStrokeColor(strength: number): string {
  if (strength > 0.8) return '#4CAF50';
  if (strength > 0.5) return '#FFC107';
  return '#F44336';
}

function drag(simulation: d3.Simulation<Node, undefined>) {
  function dragstarted(event: any) {
    if (!event.active) simulation.alphaTarget(0.3).restart();
    event.subject.fx = event.subject.x;
    event.subject.fy = event.subject.y;
  }

  function dragged(event: any) {
    event.subject.fx = event.x;
    event.subject.fy = event.y;
  }

  function dragended(event: any) {
    if (!event.active) simulation.alphaTarget(0);
    event.subject.fx = null;
    event.subject.fy = null;
  }

  return d3.drag<any, Node>()
    .on('start', dragstarted)
    .on('drag', dragged)
    .on('end', dragended);
}
```

---

### S1.2: Network Data Collection (13 points)

**Task:** Implement background service to collect network topology and statistics.

**Implementation:**
```rust
// src-tauri/src/network_monitor.rs
use wraith_core::{PeerId, Connection, DhtNode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

#[derive(Clone, serde::Serialize)]
pub struct NetworkSnapshot {
    pub timestamp: u64,
    pub nodes: Vec<PeerInfo>,
    pub links: Vec<LinkInfo>,
    pub dht_stats: DhtStats,
}

#[derive(Clone, serde::Serialize)]
pub struct PeerInfo {
    pub id: String,
    pub label: String,
    pub peer_type: String, // self, direct, relay, indirect
    pub connected_at: u64,
    pub last_seen: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct LinkInfo {
    pub source: String,
    pub target: String,
    pub latency_ms: u64,
    pub bandwidth_mbps: f64,
    pub packet_loss: f64,
}

#[derive(Clone, serde::Serialize)]
pub struct DhtStats {
    pub total_nodes: usize,
    pub routing_table_size: usize,
    pub stored_keys: usize,
    pub lookup_count_1h: u64,
}

pub struct NetworkMonitor {
    wraith: Arc<wraith_core::Client>,
    snapshot: Arc<Mutex<NetworkSnapshot>>,
}

impl NetworkMonitor {
    pub fn new(wraith: Arc<wraith_core::Client>) -> Self {
        Self {
            wraith,
            snapshot: Arc::new(Mutex::new(NetworkSnapshot {
                timestamp: 0,
                nodes: Vec::new(),
                links: Vec::new(),
                dht_stats: DhtStats {
                    total_nodes: 0,
                    routing_table_size: 0,
                    stored_keys: 0,
                    lookup_count_1h: 0,
                },
            })),
        }
    }

    pub async fn start(&self) {
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            ticker.tick().await;
            self.update_snapshot().await;
        }
    }

    async fn update_snapshot(&self) {
        let connections = self.wraith.get_active_connections().await;
        let dht_info = self.wraith.get_dht_info().await;

        let mut nodes = Vec::new();
        let mut links = Vec::new();

        // Add self node
        nodes.push(PeerInfo {
            id: self.wraith.local_peer_id().to_string(),
            label: "Me".to_string(),
            peer_type: "self".to_string(),
            connected_at: 0,
            last_seen: chrono::Utc::now().timestamp() as u64,
        });

        // Add connected peers
        for (peer_id, conn) in connections {
            nodes.push(PeerInfo {
                id: peer_id.to_string(),
                label: self.get_peer_label(&peer_id).await,
                peer_type: if conn.is_relay() { "relay" } else { "direct" },
                connected_at: conn.connected_at(),
                last_seen: conn.last_active(),
            });

            links.push(LinkInfo {
                source: self.wraith.local_peer_id().to_string(),
                target: peer_id.to_string(),
                latency_ms: conn.latency_ms(),
                bandwidth_mbps: conn.bandwidth_mbps(),
                packet_loss: conn.packet_loss_rate(),
            });
        }

        // Add DHT routing table entries
        for node in dht_info.routing_table.iter() {
            if !nodes.iter().any(|n| n.id == node.id.to_string()) {
                nodes.push(PeerInfo {
                    id: node.id.to_string(),
                    label: format!("DHT-{}", &node.id.to_string()[..8]),
                    peer_type: "indirect".to_string(),
                    connected_at: 0,
                    last_seen: node.last_seen,
                });
            }
        }

        let snapshot = NetworkSnapshot {
            timestamp: chrono::Utc::now().timestamp() as u64,
            nodes,
            links,
            dht_stats: DhtStats {
                total_nodes: dht_info.total_nodes,
                routing_table_size: dht_info.routing_table.len(),
                stored_keys: dht_info.stored_keys,
                lookup_count_1h: dht_info.lookup_count_1h,
            },
        };

        *self.snapshot.lock().unwrap() = snapshot;
    }

    pub fn get_snapshot(&self) -> NetworkSnapshot {
        self.snapshot.lock().unwrap().clone()
    }

    async fn get_peer_label(&self, peer_id: &PeerId) -> String {
        // Try to resolve friendly name from DHT
        if let Some(name) = self.wraith.resolve_peer_name(peer_id).await {
            name
        } else {
            format!("{}", &peer_id.to_string()[..8])
        }
    }
}

#[tauri::command]
pub fn get_network_snapshot(monitor: tauri::State<NetworkMonitor>) -> NetworkSnapshot {
    monitor.get_snapshot()
}
```

---

### S1.3: Statistics Dashboard (8 points)

**Task:** Build dashboard with real-time metrics and charts.

**Implementation:**
```tsx
// src/components/StatsDashboard.tsx
import React, { useState, useEffect } from 'react';
import { Line } from 'react-chartjs-2';
import { invoke } from '@tauri-apps/api/tauri';
import { NetworkSnapshot } from '../types';

export function StatsDashboard() {
  const [snapshot, setSnapshot] = useState<NetworkSnapshot | null>(null);
  const [latencyHistory, setLatencyHistory] = useState<number[]>([]);

  useEffect(() => {
    const interval = setInterval(async () => {
      const data = await invoke<NetworkSnapshot>('get_network_snapshot');
      setSnapshot(data);

      // Update latency history
      const avgLatency = data.links.reduce((sum, l) => sum + l.latency_ms, 0) / data.links.length;
      setLatencyHistory(prev => [...prev.slice(-60), avgLatency]);
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  if (!snapshot) return <div>Loading...</div>;

  return (
    <div className="stats-dashboard">
      <div className="metrics-grid">
        <MetricCard
          title="Connected Peers"
          value={snapshot.links.length}
          icon="👥"
        />
        <MetricCard
          title="DHT Nodes"
          value={snapshot.dht_stats.routing_table_size}
          icon="🌐"
        />
        <MetricCard
          title="Avg Latency"
          value={`${(latencyHistory[latencyHistory.length - 1] || 0).toFixed(0)} ms`}
          icon="⚡"
        />
        <MetricCard
          title="Packet Loss"
          value={`${(snapshot.links.reduce((sum, l) => sum + l.packet_loss, 0) / snapshot.links.length * 100).toFixed(2)}%`}
          icon="📉"
        />
      </div>

      <div className="chart-container">
        <h3>Latency Over Time</h3>
        <Line
          data={{
            labels: Array.from({ length: latencyHistory.length }, (_, i) => `${i}s`),
            datasets: [{
              label: 'Avg Latency (ms)',
              data: latencyHistory,
              borderColor: '#2196F3',
              backgroundColor: 'rgba(33, 150, 243, 0.1)',
              tension: 0.4,
            }]
          }}
          options={{
            responsive: true,
            scales: {
              y: {
                beginAtZero: true,
              }
            }
          }}
        />
      </div>

      <div className="peer-list">
        <h3>Active Connections</h3>
        <table>
          <thead>
            <tr>
              <th>Peer</th>
              <th>Type</th>
              <th>Latency</th>
              <th>Bandwidth</th>
              <th>Loss</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.links.map(link => (
              <tr key={link.target}>
                <td>{snapshot.nodes.find(n => n.id === link.target)?.label}</td>
                <td>{snapshot.nodes.find(n => n.id === link.target)?.peer_type}</td>
                <td>{link.latency_ms} ms</td>
                <td>{link.bandwidth_mbps.toFixed(2)} Mbps</td>
                <td>{(link.packet_loss * 100).toFixed(2)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function MetricCard({ title, value, icon }: { title: string; value: string | number; icon: string }) {
  return (
    <div className="metric-card">
      <div className="icon">{icon}</div>
      <div className="content">
        <div className="title">{title}</div>
        <div className="value">{value}</div>
      </div>
    </div>
  );
}
```

---

### Additional Tasks:

- **S1.4:** DHT Lookup Visualization (8 pts) - Animate DHT lookup paths
- **S1.5:** Traffic Flow Analysis (5 pts) - Visualize data flows between peers
- **S1.6:** Network Diagnostics Tools (3 pts) - Ping, traceroute, bandwidth test
- **S1.7:** Export Network Data (2 pts) - Export graph as JSON/CSV/image

---

## Completion Checklist

- [x] Network graph renders 1000+ nodes smoothly
- [x] Real-time updates working
- [x] Statistics dashboard functional
- [x] DHT lookup visualization complete
- [x] Diagnostics tools working
- [x] Desktop builds for all platforms

**Target Release Date:** Week 52

---

*WRAITH-Mesh Sprint Planning v1.0.0*
