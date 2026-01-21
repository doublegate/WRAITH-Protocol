// WRAITH Transfer - SessionPanel Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, createMockSession } from '../test/utils';
import userEvent from '@testing-library/user-event';
import { SessionPanel } from './SessionPanel';
import { useSessionStore } from '../stores/sessionStore';
import type { SessionInfo } from '../types';

// Mock the session store
vi.mock('../stores/sessionStore');

const mockUseSessionStore = vi.mocked(useSessionStore);

describe('SessionPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('when there are no sessions', () => {
    it('renders empty state message', () => {
      mockUseSessionStore.mockReturnValue({
        sessions: [],
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('No active sessions')).toBeInTheDocument();
    });

    it('displays zero connected count', () => {
      mockUseSessionStore.mockReturnValue({
        sessions: [],
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('0 connected')).toBeInTheDocument();
    });
  });

  describe('when there are sessions', () => {
    it('renders session items', () => {
      const sessions: SessionInfo[] = [
        createMockSession({ peer_id: 'a'.repeat(64), nickname: 'Alice' }),
        createMockSession({ peer_id: 'b'.repeat(64), nickname: 'Bob' }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('Alice')).toBeInTheDocument();
      expect(screen.getByText('Bob')).toBeInTheDocument();
    });

    it('displays correct connected count', () => {
      const sessions: SessionInfo[] = [
        createMockSession({ peer_id: 'a'.repeat(64) }),
        createMockSession({ peer_id: 'b'.repeat(64) }),
        createMockSession({ peer_id: 'c'.repeat(64) }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('3 connected')).toBeInTheDocument();
    });

    it('shows truncated peer ID when no nickname', () => {
      const peerId = 'a'.repeat(64);
      const sessions: SessionInfo[] = [
        createMockSession({ peer_id: peerId, nickname: undefined }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      // Should show first 16 chars + "..."
      expect(screen.getByText('aaaaaaaaaaaaaaaa...')).toBeInTheDocument();
    });

    it('displays connection status', () => {
      const sessions: SessionInfo[] = [
        createMockSession({ peer_id: 'a'.repeat(64), connection_status: 'connected' }),
        createMockSession({ peer_id: 'b'.repeat(64), connection_status: 'connecting' }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('connected')).toBeInTheDocument();
      expect(screen.getByText('connecting')).toBeInTheDocument();
    });

    it('displays bytes sent and received', () => {
      const sessions: SessionInfo[] = [
        createMockSession({
          peer_id: 'a'.repeat(64),
          bytes_sent: 1048576, // 1 MB
          bytes_received: 524288, // 512 KB
        }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('1 MB')).toBeInTheDocument();
      expect(screen.getByText('512 KB')).toBeInTheDocument();
    });
  });

  describe('close session functionality', () => {
    it('calls closeSession when close button is clicked', async () => {
      const closeSession = vi.fn();
      const peerId = 'a'.repeat(64);
      const sessions: SessionInfo[] = [
        createMockSession({ peer_id: peerId }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession,
        clearError: vi.fn(),
      });

      const user = userEvent.setup();
      render(<SessionPanel />);

      const closeButton = screen.getByTitle('Close session');
      await user.click(closeButton);

      expect(closeSession).toHaveBeenCalledWith(peerId);
    });
  });

  describe('header', () => {
    it('renders title', () => {
      mockUseSessionStore.mockReturnValue({
        sessions: [],
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      expect(screen.getByText('Active Sessions')).toBeInTheDocument();
    });
  });

  describe('duration formatting', () => {
    it('displays duration in seconds for recent sessions', () => {
      const now = Math.floor(Date.now() / 1000);
      const sessions: SessionInfo[] = [
        createMockSession({
          peer_id: 'a'.repeat(64),
          established_at: now - 30, // 30 seconds ago
        }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      // Should show seconds format (e.g., "30s" or similar)
      const durationText = screen.getByText(/\d+s/);
      expect(durationText).toBeInTheDocument();
    });

    it('displays duration in minutes for longer sessions', () => {
      const now = Math.floor(Date.now() / 1000);
      const sessions: SessionInfo[] = [
        createMockSession({
          peer_id: 'a'.repeat(64),
          established_at: now - 300, // 5 minutes ago
        }),
      ];

      mockUseSessionStore.mockReturnValue({
        sessions,
        loading: false,
        error: null,
        fetchSessions: vi.fn(),
        closeSession: vi.fn(),
        clearError: vi.fn(),
      });

      render(<SessionPanel />);

      // Should show minutes format (e.g., "5m")
      const durationText = screen.getByText(/\d+m/);
      expect(durationText).toBeInTheDocument();
    });
  });
});
