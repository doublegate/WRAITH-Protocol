// WRAITH Transfer - TransferList Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, createMockTransfer } from '../test/utils';
import { TransferList } from './TransferList';
import { useTransferStore } from '../stores/transferStore';
import type { TransferInfo } from '../types';

// Mock the transfer store
vi.mock('../stores/transferStore');

const mockUseTransferStore = vi.mocked(useTransferStore);

describe('TransferList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('when there are no transfers', () => {
    it('renders empty state message', () => {
      mockUseTransferStore.mockReturnValue({
        transfers: [],
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      expect(screen.getByText('No active transfers')).toBeInTheDocument();
      expect(screen.getByText('Start a transfer to see it here')).toBeInTheDocument();
    });
  });

  describe('when there are transfers', () => {
    it('renders transfer items', () => {
      const transfers: TransferInfo[] = [
        createMockTransfer({ id: '1', file_name: 'document.pdf', status: 'in_progress' }),
        createMockTransfer({ id: '2', file_name: 'image.png', status: 'completed' }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      expect(screen.getByText('document.pdf')).toBeInTheDocument();
      expect(screen.getByText('image.png')).toBeInTheDocument();
    });

    it('displays correct status for each transfer', () => {
      const transfers: TransferInfo[] = [
        createMockTransfer({ id: '1', file_name: 'uploading.txt', status: 'in_progress' }),
        createMockTransfer({ id: '2', file_name: 'done.txt', status: 'completed' }),
        createMockTransfer({ id: '3', file_name: 'error.txt', status: 'failed' }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      expect(screen.getByText('in progress')).toBeInTheDocument();
      expect(screen.getByText('completed')).toBeInTheDocument();
      expect(screen.getByText('failed')).toBeInTheDocument();
    });

    it('shows cancel button only for active transfers', () => {
      const transfers: TransferInfo[] = [
        createMockTransfer({ id: '1', file_name: 'active.txt', status: 'in_progress' }),
        createMockTransfer({ id: '2', file_name: 'done.txt', status: 'completed' }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      // Only one cancel button should be present (for active transfer)
      const cancelButtons = screen.getAllByTitle('Cancel transfer');
      expect(cancelButtons).toHaveLength(1);
    });

    it('displays progress percentage', () => {
      const transfers: TransferInfo[] = [
        createMockTransfer({ id: '1', progress: 0.75 }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      expect(screen.getByText('75%')).toBeInTheDocument();
    });

    it('displays file size information', () => {
      const transfers: TransferInfo[] = [
        createMockTransfer({
          id: '1',
          total_bytes: 1048576, // 1 MB
          transferred_bytes: 524288, // 512 KB
        }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      expect(screen.getByText('512 KB / 1 MB')).toBeInTheDocument();
    });

    it('displays truncated peer ID', () => {
      const peerId = 'a'.repeat(64);
      const transfers: TransferInfo[] = [
        createMockTransfer({ id: '1', peer_id: peerId }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      // Should show first 16 chars + "..."
      expect(screen.getByText('aaaaaaaaaaaaaaaa...')).toBeInTheDocument();
    });
  });
});

// Test helper functions
describe('TransferList helper functions', () => {
  describe('formatBytes', () => {
    it.each([
      [0, '0 B'],
      [1024, '1 KB'],
      [1048576, '1 MB'],
      [1073741824, '1 GB'],
      [500, '500 B'],
      [1536, '1.5 KB'],
    ])('formats %i bytes as %s', (bytes, expected) => {
      const transfers: TransferInfo[] = [
        createMockTransfer({
          id: '1',
          total_bytes: bytes,
          transferred_bytes: bytes,
          progress: 1,
        }),
      ];

      mockUseTransferStore.mockReturnValue({
        transfers,
        loading: false,
        error: null,
        fetchTransfers: vi.fn(),
        sendFile: vi.fn(),
        cancelTransfer: vi.fn(),
        clearError: vi.fn(),
      });

      render(<TransferList />);

      // Both transferred and total should show the formatted value
      expect(screen.getByText(`${expected} / ${expected}`)).toBeInTheDocument();
    });
  });
});
