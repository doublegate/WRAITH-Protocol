// WRAITH Recon - KillSwitchButton Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '../test/utils';
import { KillSwitchButton } from './KillSwitchButton';

// Mock stores
vi.mock('../stores/engagementStore', () => ({
  useEngagementStore: vi.fn(),
}));

import { useEngagementStore } from '../stores/engagementStore';

describe('KillSwitchButton Component', () => {
  const mockActivateKillSwitch = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock state - active engagement
    (useEngagementStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      status: 'Active',
      killSwitchState: null,
      activateKillSwitch: mockActivateKillSwitch,
      loading: false,
    });

    mockActivateKillSwitch.mockResolvedValue(undefined);
  });

  it('renders the kill switch button', () => {
    render(<KillSwitchButton />);

    expect(screen.getByRole('button', { name: /kill switch/i })).toBeInTheDocument();
  });

  it('has danger styling when active', () => {
    render(<KillSwitchButton />);

    const button = screen.getByRole('button', { name: /kill switch/i });
    expect(button).toHaveClass('bg-red-600');
  });

  it('shows confirmation modal when clicked', async () => {
    const { user } = render(<KillSwitchButton />);

    const button = screen.getByRole('button', { name: /kill switch/i });
    await user.click(button);

    expect(screen.getByText('Confirm Kill Switch')).toBeInTheDocument();
    expect(screen.getByText(/this will immediately terminate all active operations/i)).toBeInTheDocument();
  });

  it('allows canceling the confirmation', async () => {
    const { user } = render(<KillSwitchButton />);

    // Open modal
    const button = screen.getByRole('button', { name: /kill switch/i });
    await user.click(button);

    // Click cancel
    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    await user.click(cancelButton);

    // Modal should be closed
    expect(screen.queryByText('Confirm Kill Switch')).not.toBeInTheDocument();
  });

  it('calls activateKillSwitch when confirmed with reason', async () => {
    const { user } = render(<KillSwitchButton />);

    // Open modal
    const button = screen.getByRole('button', { name: /kill switch/i });
    await user.click(button);

    // Enter a reason
    const reasonInput = screen.getByPlaceholderText(/enter reason for emergency termination/i);
    await user.type(reasonInput, 'Emergency situation');

    // Confirm
    const confirmButton = screen.getByRole('button', { name: /terminate/i });
    await user.click(confirmButton);

    await waitFor(() => {
      expect(mockActivateKillSwitch).toHaveBeenCalledWith('Emergency situation');
    });
  });

  it('requires reason input for confirmation', async () => {
    const { user } = render(<KillSwitchButton />);

    // Open modal
    const button = screen.getByRole('button', { name: /kill switch/i });
    await user.click(button);

    // Confirm button should be disabled without reason
    const confirmButton = screen.getByRole('button', { name: /terminate/i });
    expect(confirmButton).toBeDisabled();

    // Enter a reason
    const reasonInput = screen.getByPlaceholderText(/enter reason for emergency termination/i);
    await user.type(reasonInput, 'Emergency situation');

    // Now confirm should be enabled
    expect(confirmButton).not.toBeDisabled();
  });
});
