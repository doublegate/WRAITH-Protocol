# WRAITH Mesh

Network topology visualization and diagnostics tool for the WRAITH Protocol.

## Features

- **Network Graph Visualization**: Real-time force-directed graph showing peer connections
- **Statistics Dashboard**: Live metrics including connected peers, latency, and bandwidth
- **DHT Inspector**: View routing table, stored keys, and lookup traces
- **Network Diagnostics**: Ping peers, bandwidth tests, and NAT detection
- **Data Export**: Export network topology as JSON or CSV

## Development

### Prerequisites

- Rust 1.85+
- Node.js 18+
- npm 9+

### Setup

```bash
# Install dependencies
npm install
cd frontend && npm install && cd ..

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## Architecture

### Backend (Rust/Tauri)

- `network_monitor.rs` - Real-time network data collection
- `dht_inspector.rs` - DHT routing table inspection
- `diagnostics.rs` - Network diagnostic tools
- `export.rs` - Data export functionality

### Frontend (React/TypeScript)

- `NetworkGraph.tsx` - D3.js force-directed graph
- `StatsDashboard.tsx` - Real-time metrics display
- `PeerList.tsx` - Connected peer table
- `DhtViewer.tsx` - DHT visualization
- `DiagnosticsPanel.tsx` - Network tools UI
- `TrafficFlow.tsx` - Data flow visualization

## License

MIT OR Apache-2.0
