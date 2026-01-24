// WRAITH Recon - Reconnaissance Panel Component

import { useState } from 'react';
import { useReconStore } from '../stores/reconStore';
import { useEngagementStore } from '../stores/engagementStore';
import type { ProbeType, ActiveScanConfig } from '../types';

type TabType = 'passive' | 'active';

export function ReconPanel() {
  const { status } = useEngagementStore();
  const {
    passiveStats, discoveredAssets, passiveRunning,
    activeScanProgress, activeScanResults, activeRunning,
    startPassiveRecon, stopPassiveRecon, fetchDiscoveredAssets,
    startActiveScan, stopActiveScan, fetchActiveScanResults,
    loading, error,
  } = useReconStore();

  const [activeTab, setActiveTab] = useState<TabType>('passive');
  const [interfaceName, setInterfaceName] = useState('');
  const [captureTimeout, setCaptureTimeout] = useState(60);

  // Active scan config
  const [targets, setTargets] = useState('');
  const [ports, setPorts] = useState('80,443,22,21,25,8080');
  const [probeType, setProbeType] = useState<ProbeType>('TcpConnect');
  const [rateLimit, setRateLimit] = useState(100);
  const [serviceDetection, setServiceDetection] = useState(true);

  const canOperate = status === 'Active';

  const handleStartPassive = async () => {
    await startPassiveRecon(
      interfaceName || undefined,
      captureTimeout > 0 ? captureTimeout : undefined
    );
  };

  const handleStopPassive = async () => {
    await stopPassiveRecon();
    await fetchDiscoveredAssets();
  };

  const handleStartActive = async () => {
    const targetList = targets.split(/[\n,;]/).map(t => t.trim()).filter(Boolean);
    const portList = ports.split(',').map(p => parseInt(p.trim())).filter(p => !isNaN(p) && p > 0 && p < 65536);

    if (targetList.length === 0 || portList.length === 0) {
      return;
    }

    const config: ActiveScanConfig = {
      targets: targetList,
      ports: portList,
      probe_type: probeType,
      rate_limit: rateLimit,
      timeout_ms: 5000,
      retries: 2,
      service_detection: serviceDetection,
      os_detection: false,
    };

    await startActiveScan(config);
  };

  const handleStopActive = async () => {
    await stopActiveScan();
    await fetchActiveScanResults();
  };

  return (
    <div className="card flex flex-col h-full">
      {/* Tab Header */}
      <div className="flex border-b border-border-primary">
        <button
          onClick={() => setActiveTab('passive')}
          className={`flex-1 py-3 px-4 text-sm font-medium transition-colors ${
            activeTab === 'passive'
              ? 'text-primary-400 border-b-2 border-primary-500 bg-primary-500/5'
              : 'text-text-secondary hover:text-text-primary'
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
            Passive Recon
            {passiveRunning && (
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            )}
          </div>
        </button>
        <button
          onClick={() => setActiveTab('active')}
          className={`flex-1 py-3 px-4 text-sm font-medium transition-colors ${
            activeTab === 'active'
              ? 'text-primary-400 border-b-2 border-primary-500 bg-primary-500/5'
              : 'text-text-secondary hover:text-text-primary'
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            Active Scanning
            {activeRunning && (
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            )}
          </div>
        </button>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto p-4">
        {error && (
          <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
            {error}
          </div>
        )}

        {activeTab === 'passive' ? (
          <div className="space-y-4">
            {/* Passive Controls */}
            <div className="space-y-3">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">
                  Network Interface (optional)
                </label>
                <input
                  type="text"
                  value={interfaceName}
                  onChange={(e) => setInterfaceName(e.target.value)}
                  placeholder="eth0, wlan0, etc..."
                  className="input"
                  disabled={passiveRunning}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">
                  Capture Timeout (seconds)
                </label>
                <input
                  type="number"
                  value={captureTimeout}
                  onChange={(e) => setCaptureTimeout(parseInt(e.target.value) || 0)}
                  min={0}
                  className="input"
                  disabled={passiveRunning}
                />
              </div>

              {passiveRunning ? (
                <button
                  onClick={handleStopPassive}
                  disabled={loading || !canOperate}
                  className="btn btn-danger w-full"
                >
                  Stop Capture
                </button>
              ) : (
                <button
                  onClick={handleStartPassive}
                  disabled={loading || !canOperate}
                  className="btn btn-primary w-full"
                >
                  Start Passive Capture
                </button>
              )}
            </div>

            {/* Passive Stats */}
            {passiveStats && (
              <div className="p-3 rounded-lg bg-bg-tertiary space-y-2">
                <h4 className="text-sm font-medium text-text-primary">Capture Statistics</h4>
                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div>
                    <span className="text-text-muted">Packets:</span>
                    <span className="ml-2 text-text-primary">{passiveStats.packets_captured.toLocaleString()}</span>
                  </div>
                  <div>
                    <span className="text-text-muted">Bytes:</span>
                    <span className="ml-2 text-text-primary">{(passiveStats.bytes_captured / 1024).toFixed(1)} KB</span>
                  </div>
                  <div>
                    <span className="text-text-muted">Unique IPs:</span>
                    <span className="ml-2 text-text-primary">{passiveStats.unique_ips}</span>
                  </div>
                  <div>
                    <span className="text-text-muted">Services:</span>
                    <span className="ml-2 text-text-primary">{passiveStats.services_identified}</span>
                  </div>
                </div>
              </div>
            )}

            {/* Discovered Assets */}
            {discoveredAssets.length > 0 && (
              <div>
                <h4 className="text-sm font-medium text-text-secondary mb-2">
                  Discovered Assets ({discoveredAssets.length})
                </h4>
                <div className="space-y-2 max-h-64 overflow-auto">
                  {discoveredAssets.map((asset) => (
                    <div key={asset.ip} className="p-2 rounded bg-bg-tertiary">
                      <div className="flex justify-between">
                        <span className="font-mono text-sm text-primary-400">{asset.ip}</span>
                        <span className="text-xs text-text-muted">{asset.packet_count} pkts</span>
                      </div>
                      {asset.hostnames.length > 0 && (
                        <p className="text-xs text-text-secondary mt-1">
                          {asset.hostnames.join(', ')}
                        </p>
                      )}
                      {asset.ports.length > 0 && (
                        <div className="flex flex-wrap gap-1 mt-1">
                          {asset.ports.slice(0, 10).map((port) => (
                            <span key={port} className="text-xs px-1 rounded bg-bg-elevated text-text-muted">
                              {port}
                            </span>
                          ))}
                          {asset.ports.length > 10 && (
                            <span className="text-xs text-text-muted">
                              +{asset.ports.length - 10} more
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="space-y-4">
            {/* Active Scan Controls */}
            <div className="space-y-3">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">
                  Targets (one per line or comma-separated)
                </label>
                <textarea
                  value={targets}
                  onChange={(e) => setTargets(e.target.value)}
                  placeholder="192.168.1.0/24&#10;10.0.0.1&#10;example.com"
                  className="input h-20 resize-none font-mono text-sm"
                  disabled={activeRunning}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">
                  Ports (comma-separated)
                </label>
                <input
                  type="text"
                  value={ports}
                  onChange={(e) => setPorts(e.target.value)}
                  placeholder="80,443,22,8080"
                  className="input font-mono text-sm"
                  disabled={activeRunning}
                />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-sm font-medium text-text-secondary mb-1">
                    Probe Type
                  </label>
                  <select
                    value={probeType}
                    onChange={(e) => setProbeType(e.target.value as ProbeType)}
                    className="input"
                    disabled={activeRunning}
                  >
                    <option value="TcpConnect">TCP Connect</option>
                    <option value="TcpSyn">TCP SYN</option>
                    <option value="TcpAck">TCP ACK</option>
                    <option value="UdpProbe">UDP</option>
                    <option value="IcmpPing">ICMP Ping</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-secondary mb-1">
                    Rate Limit (probes/sec)
                  </label>
                  <input
                    type="number"
                    value={rateLimit}
                    onChange={(e) => setRateLimit(parseInt(e.target.value) || 100)}
                    min={1}
                    max={10000}
                    className="input"
                    disabled={activeRunning}
                  />
                </div>
              </div>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={serviceDetection}
                  onChange={(e) => setServiceDetection(e.target.checked)}
                  disabled={activeRunning}
                  className="w-4 h-4 rounded border-border-primary"
                />
                <span className="text-sm text-text-secondary">Enable Service Detection</span>
              </label>

              {activeRunning ? (
                <button
                  onClick={handleStopActive}
                  disabled={loading || !canOperate}
                  className="btn btn-danger w-full"
                >
                  Stop Scan
                </button>
              ) : (
                <button
                  onClick={handleStartActive}
                  disabled={loading || !canOperate || !targets.trim()}
                  className="btn btn-primary w-full"
                >
                  Start Active Scan
                </button>
              )}
            </div>

            {/* Scan Progress */}
            {activeScanProgress && (
              <div className="p-3 rounded-lg bg-bg-tertiary space-y-3">
                <div className="flex justify-between items-center">
                  <h4 className="text-sm font-medium text-text-primary">Scan Progress</h4>
                  <span className={`text-xs px-2 py-0.5 rounded-full ${
                    activeScanProgress.status === 'Running' ? 'bg-green-500/20 text-green-400' :
                    activeScanProgress.status === 'Completed' ? 'bg-blue-500/20 text-blue-400' :
                    'bg-gray-500/20 text-gray-400'
                  }`}>
                    {activeScanProgress.status}
                  </span>
                </div>

                {/* Progress Bar */}
                <div className="progress-bar">
                  <div
                    className="progress-bar-fill"
                    style={{
                      width: `${(activeScanProgress.completed_probes / activeScanProgress.total_probes) * 100}%`
                    }}
                  />
                </div>

                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div>
                    <span className="text-text-muted">Probes:</span>
                    <span className="ml-2 text-text-primary">
                      {activeScanProgress.completed_probes} / {activeScanProgress.total_probes}
                    </span>
                  </div>
                  <div>
                    <span className="text-text-muted">Open Ports:</span>
                    <span className="ml-2 text-green-400">{activeScanProgress.open_ports_found}</span>
                  </div>
                </div>

                {activeScanProgress.current_target && (
                  <div className="text-xs text-text-muted">
                    Current: <span className="font-mono text-text-secondary">{activeScanProgress.current_target}</span>
                  </div>
                )}
              </div>
            )}

            {/* Scan Results */}
            {activeScanResults.length > 0 && (
              <div>
                <h4 className="text-sm font-medium text-text-secondary mb-2">
                  Open Ports ({activeScanResults.filter(r => r.open).length})
                </h4>
                <div className="space-y-1 max-h-48 overflow-auto">
                  {activeScanResults.filter(r => r.open).map((result, idx) => (
                    <div key={idx} className="flex items-center gap-2 p-2 rounded bg-bg-tertiary text-sm">
                      <span className="font-mono text-primary-400">{result.target}</span>
                      <span className="text-text-muted">:</span>
                      <span className="font-mono text-green-400">{result.port}</span>
                      {result.service && (
                        <span className="text-xs px-1.5 py-0.5 rounded bg-primary-500/20 text-primary-400">
                          {result.service}
                        </span>
                      )}
                      <span className="ml-auto text-xs text-text-muted">
                        {result.response_time_ms.toFixed(1)}ms
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
