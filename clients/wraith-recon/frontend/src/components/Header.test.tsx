// WRAITH Recon - Header Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '../test/utils';
import { Header } from './Header';

// Mock the stores
vi.mock('../stores/engagementStore', () => ({
  useEngagementStore: vi.fn(),
}));

vi.mock('../stores/nodeStore', () => ({
  useNodeStore: vi.fn(),
}));

import { useEngagementStore } from '../stores/engagementStore';
import { useNodeStore } from '../stores/nodeStore';

describe('Header Component', () => {
  const mockOnOpenSettings = vi.fn();
  const mockOnOpenRoe = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock states
    (useEngagementStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: 'NotLoaded',
      engagementId: null,
      roe: null,
    });

    (useNodeStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: { running: false },
    });
  });

  it('renders the header title', () => {
    render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    expect(screen.getByText('WRAITH Recon')).toBeInTheDocument();
  });

  it('shows No RoE Loaded status when not loaded', () => {
    render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    expect(screen.getByText(/No RoE Loaded/i)).toBeInTheDocument();
  });

  it('shows Engagement Active status when active', () => {
    (useEngagementStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: 'Active',
      engagementId: 'test-123',
      roe: {
        id: 'test-roe',
        title: 'Test Engagement',
      },
    });

    render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    expect(screen.getByText('Engagement Active')).toBeInTheDocument();
  });

  it('shows node status as Online when running', () => {
    (useNodeStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: { running: true },
    });

    render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    expect(screen.getByText('Node Online')).toBeInTheDocument();
  });

  it('shows node status as Offline when not running', () => {
    (useNodeStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: { running: false },
    });

    render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    expect(screen.getByText('Node Offline')).toBeInTheDocument();
  });

  it('calls onOpenSettings when settings button is clicked', async () => {
    const { user } = render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    const settingsButton = screen.getByTitle('Settings');
    await user.click(settingsButton);

    expect(mockOnOpenSettings).toHaveBeenCalled();
  });

  it('calls onOpenRoe when RoE button is clicked', async () => {
    const { user } = render(
      <Header
        onOpenSettings={mockOnOpenSettings}
        onOpenRoe={mockOnOpenRoe}
      />
    );

    const roeButton = screen.getByTitle('Load Rules of Engagement');
    await user.click(roeButton);

    expect(mockOnOpenRoe).toHaveBeenCalled();
  });
});
