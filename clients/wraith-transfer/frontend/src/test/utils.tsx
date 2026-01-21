// WRAITH Transfer - Test Utilities
// Custom render functions and test helpers

import { render, RenderOptions } from '@testing-library/react';
import { ReactElement, ReactNode } from 'react';

// Custom render function that can wrap components with providers
function customRender(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) {
  // For now we don't need providers, but this allows easy extension
  const Wrapper = ({ children }: { children: ReactNode }) => {
    return <>{children}</>;
  };

  return render(ui, { wrapper: Wrapper, ...options });
}

// Re-export everything from testing-library
export * from '@testing-library/react';
export { customRender as render };

// Test data factories
export function createMockTransfer(overrides: Partial<import('../types').TransferInfo> = {}): import('../types').TransferInfo {
  return {
    id: 'test-transfer-1',
    peer_id: 'a'.repeat(64),
    file_name: 'test-file.txt',
    total_bytes: 1024000,
    transferred_bytes: 512000,
    progress: 0.5,
    status: 'in_progress',
    direction: 'upload',
    ...overrides,
  };
}

export function createMockSession(overrides: Partial<import('../types').SessionInfo> = {}): import('../types').SessionInfo {
  return {
    peer_id: 'b'.repeat(64),
    established_at: Math.floor(Date.now() / 1000) - 300,
    bytes_sent: 1024000,
    bytes_received: 512000,
    connection_status: 'connected',
    ...overrides,
  };
}
