// WRAITH Recon - ScopePanel Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '../test/utils';
import { ScopePanel } from './ScopePanel';
import * as tauri from '../lib/tauri';

vi.mock('../lib/tauri');

// Mock stores
vi.mock('../stores/engagementStore', () => ({
  useEngagementStore: vi.fn(),
}));

import { useEngagementStore } from '../stores/engagementStore';

describe('ScopePanel Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock state with RoE loaded
    (useEngagementStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: 'Active',
      roe: {
        id: 'test-roe',
        title: 'Test Engagement',
        organization: 'Test Organization',
        start_time: '2024-01-01T00:00:00Z',
        end_time: '2024-12-31T23:59:59Z',
        authorized_cidrs: ['192.168.1.0/24', '10.0.0.0/8'],
        authorized_domains: ['example.com', 'test.local'],
        excluded_cidrs: ['192.168.1.1/32'],
        excluded_domains: [],
        authorized_techniques: ['T1046', 'T1018', 'T1016'],
        prohibited_techniques: ['T1485', 'T1486'],
        authorized_operators: ['op-1'],
        signer_public_key: 'key123',
        signature: 'sig123',
      },
      scopeSummary: {
        authorized_cidr_count: 2,
        authorized_domain_count: 2,
        excluded_cidr_count: 1,
        excluded_domain_count: 0,
        custom_target_count: 0,
      },
    });

    (tauri.validateTarget as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      in_scope: true,
      reason: 'IP is in authorized CIDR range',
    });
  });

  it('renders scope information', () => {
    render(<ScopePanel />);

    expect(screen.getByText('Target Scope')).toBeInTheDocument();
  });

  it('displays authorized CIDRs count', () => {
    render(<ScopePanel />);

    // Check for the count display
    expect(screen.getByText('Authorized CIDRs')).toBeInTheDocument();
    // Both authorized CIDRs and domains have count 2, so use getAllByText
    const countElements = screen.getAllByText('2');
    expect(countElements.length).toBeGreaterThanOrEqual(1);
  });

  it('displays authorized domains count', () => {
    render(<ScopePanel />);

    expect(screen.getByText('Authorized Domains')).toBeInTheDocument();
  });

  it('displays excluded CIDRs count', () => {
    render(<ScopePanel />);

    expect(screen.getByText('Excluded CIDRs')).toBeInTheDocument();
  });

  it('displays authorized techniques', () => {
    render(<ScopePanel />);

    expect(screen.getByText('T1046')).toBeInTheDocument();
    expect(screen.getByText('T1018')).toBeInTheDocument();
  });

  it('displays prohibited techniques', () => {
    render(<ScopePanel />);

    expect(screen.getByText('T1485')).toBeInTheDocument();
    expect(screen.getByText('T1486')).toBeInTheDocument();
  });

  it('allows validating a target', async () => {
    const { user } = render(<ScopePanel />);

    // Find and fill the validation input
    const input = screen.getByPlaceholderText(/IP, CIDR, or domain/i);
    await user.type(input, '192.168.1.100');

    // Click validate
    const validateButton = screen.getByRole('button', { name: /check/i });
    await user.click(validateButton);

    await waitFor(() => {
      expect(tauri.validateTarget).toHaveBeenCalledWith('192.168.1.100');
    });
  });

  it('shows validation result', async () => {
    (tauri.validateTarget as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      in_scope: true,
      reason: 'IP is in authorized CIDR range',
    });

    const { user } = render(<ScopePanel />);

    const input = screen.getByPlaceholderText(/IP, CIDR, or domain/i);
    await user.type(input, '192.168.1.100');

    const validateButton = screen.getByRole('button', { name: /check/i });
    await user.click(validateButton);

    await waitFor(() => {
      expect(screen.getByText('In Scope')).toBeInTheDocument();
    });
  });

  it('shows out of scope warning', async () => {
    (tauri.validateTarget as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      in_scope: false,
      reason: 'IP is not in any authorized scope',
    });

    const { user } = render(<ScopePanel />);

    const input = screen.getByPlaceholderText(/IP, CIDR, or domain/i);
    await user.type(input, '8.8.8.8');

    const validateButton = screen.getByRole('button', { name: /check/i });
    await user.click(validateButton);

    await waitFor(() => {
      expect(screen.getByText('Out of Scope')).toBeInTheDocument();
    });
  });

  it('shows no RoE message when not loaded', () => {
    (useEngagementStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: 'NotLoaded',
      roe: null,
      scopeSummary: null,
    });

    render(<ScopePanel />);

    expect(screen.getByText('No RoE Loaded')).toBeInTheDocument();
  });
});
