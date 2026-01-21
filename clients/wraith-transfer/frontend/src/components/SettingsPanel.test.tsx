// WRAITH Transfer - SettingsPanel Component Tests

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '../test/utils';
import userEvent from '@testing-library/user-event';
import { SettingsPanel } from './SettingsPanel';
import { useSettingsStore } from '../stores/settingsStore';

// Mock the settings store
vi.mock('../stores/settingsStore');

const mockUseSettingsStore = vi.mocked(useSettingsStore);

describe('SettingsPanel', () => {
  const mockOnClose = vi.fn();
  const defaultStoreValues = {
    theme: 'dark' as const,
    downloadDir: '/home/user/Downloads',
    maxConcurrentTransfers: 3,
    port: 8337,
    autoAcceptTransfers: false,
    setTheme: vi.fn(),
    setDownloadDir: vi.fn(),
    setMaxConcurrentTransfers: vi.fn(),
    setPort: vi.fn(),
    setAutoAcceptTransfers: vi.fn(),
    resetToDefaults: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseSettingsStore.mockReturnValue(defaultStoreValues);
  });

  describe('visibility', () => {
    it('renders nothing when not open', () => {
      render(<SettingsPanel isOpen={false} onClose={mockOnClose} />);

      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    it('renders dialog when open', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText('Settings')).toBeInTheDocument();
    });
  });

  describe('appearance section', () => {
    it('renders theme buttons', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    it('calls setTheme when theme button is clicked', async () => {
      const setTheme = vi.fn();
      mockUseSettingsStore.mockReturnValue({
        ...defaultStoreValues,
        setTheme,
      });

      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Light'));

      expect(setTheme).toHaveBeenCalledWith('light');
    });

    it('highlights current theme', () => {
      mockUseSettingsStore.mockReturnValue({
        ...defaultStoreValues,
        theme: 'dark',
      });

      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const darkButton = screen.getByText('Dark');
      // Check that dark button has the active styling (contains bg-wraith-primary class)
      expect(darkButton.className).toContain('bg-wraith-primary');
    });
  });

  describe('general section', () => {
    it('renders download directory input', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Download Directory')).toBeInTheDocument();
      expect(screen.getByDisplayValue('/home/user/Downloads')).toBeInTheDocument();
    });

    it('renders max concurrent transfers input', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Max Concurrent Transfers')).toBeInTheDocument();
      expect(screen.getByDisplayValue('3')).toBeInTheDocument();
    });

    it('renders auto-accept toggle', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Auto-accept transfers')).toBeInTheDocument();
      const toggle = screen.getByRole('switch');
      expect(toggle).toHaveAttribute('aria-checked', 'false');
    });

    it('calls setAutoAcceptTransfers when toggle is clicked', async () => {
      const setAutoAcceptTransfers = vi.fn();
      mockUseSettingsStore.mockReturnValue({
        ...defaultStoreValues,
        setAutoAcceptTransfers,
      });

      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const toggle = screen.getByRole('switch');
      await user.click(toggle);

      expect(setAutoAcceptTransfers).toHaveBeenCalledWith(true);
    });
  });

  describe('network section', () => {
    it('renders port input', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByText('Port')).toBeInTheDocument();
      expect(screen.getByDisplayValue('8337')).toBeInTheDocument();
    });
  });

  describe('dialog actions', () => {
    it('calls onClose when close button is clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      // Click the X button
      await user.click(screen.getByLabelText('Close settings'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('calls onClose when Cancel is clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Cancel'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('calls onClose when clicking backdrop', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      // Click the backdrop (dialog overlay)
      await user.click(screen.getByRole('dialog'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('saves settings and closes when Save is clicked', async () => {
      const setDownloadDir = vi.fn();
      const setMaxConcurrentTransfers = vi.fn();
      const setPort = vi.fn();
      mockUseSettingsStore.mockReturnValue({
        ...defaultStoreValues,
        setDownloadDir,
        setMaxConcurrentTransfers,
        setPort,
      });

      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Save'));

      expect(setDownloadDir).toHaveBeenCalled();
      expect(setMaxConcurrentTransfers).toHaveBeenCalled();
      expect(setPort).toHaveBeenCalled();
      expect(mockOnClose).toHaveBeenCalled();
    });

    it('calls resetToDefaults when Reset button is clicked', async () => {
      const resetToDefaults = vi.fn();
      mockUseSettingsStore.mockReturnValue({
        ...defaultStoreValues,
        resetToDefaults,
      });

      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      await user.click(screen.getByText('Reset to Defaults'));

      expect(resetToDefaults).toHaveBeenCalled();
    });
  });

  describe('accessibility', () => {
    it('has proper ARIA attributes', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const dialog = screen.getByRole('dialog');
      expect(dialog).toHaveAttribute('aria-modal', 'true');
      expect(dialog).toHaveAttribute('aria-labelledby', 'settings-title');
    });

    it('has accessible toggle button', () => {
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const toggle = screen.getByRole('switch');
      expect(toggle).toBeInTheDocument();
      expect(toggle).toHaveAttribute('aria-checked');
    });
  });

  describe('form behavior', () => {
    it('allows editing download directory', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByDisplayValue('/home/user/Downloads');
      await user.clear(input);
      await user.type(input, '/new/path');

      expect(input).toHaveValue('/new/path');
    });

    it('allows editing max concurrent transfers', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByDisplayValue('3') as HTMLInputElement;
      // Select all text and replace
      await user.tripleClick(input);
      await user.keyboard('5');

      expect(input).toHaveValue(5);
    });

    it('allows editing port', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel isOpen={true} onClose={mockOnClose} />);

      const input = screen.getByDisplayValue('8337') as HTMLInputElement;
      // Select all text and replace
      await user.tripleClick(input);
      await user.keyboard('9000');

      expect(input).toHaveValue(9000);
    });
  });
});
