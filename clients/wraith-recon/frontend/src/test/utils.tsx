// WRAITH Recon - Test Utilities
// Custom render and helpers for component testing

import { render, RenderOptions } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReactElement, ReactNode } from 'react';

// Custom render that includes providers if needed
// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface CustomRenderOptions extends Omit<RenderOptions, 'wrapper'> {
  // Add provider options here if needed
}

function AllTheProviders({ children }: { children: ReactNode }) {
  // Add any providers (context, etc.) here
  return <>{children}</>;
}

function customRender(
  ui: ReactElement,
  options?: CustomRenderOptions
) {
  return {
    user: userEvent.setup(),
    ...render(ui, { wrapper: AllTheProviders, ...options }),
  };
}

// Re-export everything from testing-library
export * from '@testing-library/react';
export { customRender as render };
export { userEvent };

// Mock data factories for testing
export const mockRulesOfEngagement = {
  engagementId: 'test-engagement-001',
  clientName: 'Test Client',
  validFrom: new Date().toISOString(),
  validUntil: new Date(Date.now() + 86400000).toISOString(),
  authorizedScope: {
    networks: ['192.168.1.0/24', '10.0.0.0/8'],
    domains: ['*.test.local', 'example.com'],
    excludedTargets: ['192.168.1.1'],
  },
  authorizedTechniques: ['T1046', 'T1018', 'T1016', 'T1049'],
  prohibitedTechniques: ['T1485', 'T1486', 'T1489'],
  maxBandwidthBps: 1000000,
  timeWindowsUtc: [{ start: '09:00', end: '17:00' }],
  signature: 'test-signature-base64',
  signerPublicKey: 'test-public-key-base64',
  signedAt: new Date().toISOString(),
};

export const mockEngagementStatus = {
  state: 'idle' as const,
  startTime: null,
  currentRoe: null,
  lastHeartbeat: null,
  killSwitchArmed: false,
  activeReconTasks: 0,
  activeChannels: 0,
  bytesTransferred: 0,
  eventsLogged: 0,
};

export const mockReconResults = {
  passive: {
    hosts: [
      {
        address: '192.168.1.100',
        hostname: 'server1.test.local',
        mac: '00:11:22:33:44:55',
        discoveredAt: new Date().toISOString(),
        discoveryMethod: 'arp',
      },
    ],
    services: [
      {
        hostAddress: '192.168.1.100',
        port: 80,
        protocol: 'tcp',
        service: 'HTTP',
        banner: 'nginx/1.20.0',
        discoveredAt: new Date().toISOString(),
      },
    ],
    metadata: {
      dnsRecords: [],
      networkTopology: [],
      trafficPatterns: [],
    },
  },
  active: {
    hosts: [],
    services: [],
    vulnerabilities: [],
    systemInfo: [],
  },
};

export const mockAuditEntry = {
  id: 'audit-001',
  timestamp: new Date().toISOString(),
  eventType: 'engagement_started',
  actorId: 'test-operator',
  targetId: null,
  details: { roe_id: 'test-engagement-001' },
  hashChain: {
    previousHash: '0'.repeat(64),
    currentHash: 'a'.repeat(64),
    signature: 'test-signature',
  },
};

export const mockChannel = {
  id: 'channel-001',
  channelType: 'udp' as const,
  state: 'idle' as const,
  config: {
    targetAddress: '192.168.1.200',
    targetPort: 443,
    encryption: true,
    obfuscation: true,
  },
  stats: {
    bytesSent: 0,
    bytesReceived: 0,
    packetsDropped: 0,
    avgLatencyMs: 0,
    lastActivity: null,
  },
};
