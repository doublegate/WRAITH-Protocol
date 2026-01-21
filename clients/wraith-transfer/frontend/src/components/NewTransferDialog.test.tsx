// WRAITH Transfer - NewTransferDialog Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '../test/utils';
import userEvent from '@testing-library/user-event';
import { NewTransferDialog } from './NewTransferDialog';
import { useTransferStore } from '../stores/transferStore';

// Mock the transfer store
vi.mock('../stores/transferStore');

const mockUseTransferStore = vi.mocked(useTransferStore);

describe('NewTransferDialog', () => {
  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseTransferStore.mockReturnValue({
      transfers: [],
      loading: false,
      error: null,
      fetchTransfers: vi.fn(),
      sendFile: vi.fn().mockResolvedValue('test-transfer-id'),
      cancelTransfer: vi.fn(),
      clearError: vi.fn(),
    });
  });

  describe('visibility', () => {
    it('renders nothing when not open', () => {
      render(<NewTransferDialog isOpen={false} onClose={mockOnClose} />);

      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    it('renders dialog when open', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText('New Transfer')).toBeInTheDocument();
    });
  });

  describe('form elements', () => {
    it('renders peer ID input', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByPlaceholderText('Enter 64-character hex peer ID')).toBeInTheDocument();
    });

    it('renders file selection', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByPlaceholderText('Select a file...')).toBeInTheDocument();
      expect(screen.getByText('Browse')).toBeInTheDocument();
    });

    it('renders action buttons', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Cancel')).toBeInTheDocument();
      expect(screen.getByText('Send File')).toBeInTheDocument();
    });
  });

  describe('peer ID validation', () => {
    it('shows error for invalid peer ID length', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      await user.type(input, 'abc123');
      fireEvent.blur(input);

      expect(screen.getByText('Peer ID must be exactly 64 hexadecimal characters')).toBeInTheDocument();
    });

    it('shows error for non-hex characters', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      // 64 characters but with invalid chars
      await user.type(input, 'g'.repeat(64));
      fireEvent.blur(input);

      expect(screen.getByText('Peer ID must contain only hexadecimal characters (0-9, a-f, A-F)')).toBeInTheDocument();
    });

    it('accepts valid 64-character hex peer ID', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      await user.type(input, 'a'.repeat(64));
      fireEvent.blur(input);

      expect(screen.queryByText(/Peer ID must/)).not.toBeInTheDocument();
    });

    it('displays character count', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      await user.type(input, 'abc123');

      expect(screen.getByText('6/64 characters')).toBeInTheDocument();
    });
  });

  describe('button states', () => {
    it('disables Send button when peer ID is empty', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const sendButton = screen.getByText('Send File');
      expect(sendButton).toBeDisabled();
    });

    it('disables Send button when file is not selected', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      await user.type(input, 'a'.repeat(64));

      const sendButton = screen.getByText('Send File');
      expect(sendButton).toBeDisabled();
    });

    it('shows loading state when sending', () => {
      mockUseTransferStore.mockReturnValue({
        transfers: [],
        loading: true,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Sending...')).toBeInTheDocument();
    });
  });

  describe('error handling', () => {
    it('displays store error message', () => {
      mockUseTransferStore.mockReturnValue({
        transfers: [],
        loading: false,
        error: 'Failed to connect to peer',
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Failed to connect to peer')).toBeInTheDocument();
    });
  });

  describe('dialog actions', () => {
    it('calls onClose when Cancel is clicked', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Cancel'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('calls onClose when clicking backdrop', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      // Click the backdrop (dialog overlay)
      await user.click(screen.getByRole('dialog'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('clears error when closing', async () => {
      const clearError = vi.fn();
      mockUseTransferStore.mockReturnValue({
        transfers: [],
        loading: false,
        error: 'Some error',
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError,
      });

      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Cancel'));

      expect(clearError).toHaveBeenCalled();
    });
  });

  describe('accessibility', () => {
    it('has proper ARIA attributes', () => {
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const dialog = screen.getByRole('dialog');
      expect(dialog).toHaveAttribute('aria-modal', 'true');
      expect(dialog).toHaveAttribute('aria-labelledby', 'new-transfer-title');
    });

    it('marks input as invalid when there is a validation error', async () => {
      const user = userEvent.setup();
      render(<NewTransferDialog isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByPlaceholderText('Enter 64-character hex peer ID');
      await user.type(input, 'invalid');
      fireEvent.blur(input);

      expect(input).toHaveAttribute('aria-invalid', 'true');
    });
  });
});
