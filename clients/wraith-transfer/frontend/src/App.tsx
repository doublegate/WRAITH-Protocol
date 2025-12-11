// WRAITH Transfer - Main Application

import { useEffect, useState } from 'react';
import { Header, TransferList, SessionPanel, NewTransferDialog, StatusBar, SettingsPanel } from './components';
import { useNodeStore } from './stores/nodeStore';
import { useTransferStore } from './stores/transferStore';
import { useSessionStore } from './stores/sessionStore';
import { useSettingsStore } from './stores/settingsStore';

function App() {
  const [showNewTransfer, setShowNewTransfer] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const { fetchStatus, status } = useNodeStore();
  const { fetchTransfers } = useTransferStore();
  const { fetchSessions } = useSessionStore();
  const { theme } = useSettingsStore();

  // Initial fetch
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  // Polling for updates when node is running
  useEffect(() => {
    if (!status?.running) return;

    const interval = setInterval(() => {
      fetchStatus();
      fetchTransfers();
      fetchSessions();
    }, 1000);

    return () => clearInterval(interval);
  }, [status?.running, fetchStatus, fetchTransfers, fetchSessions]);

  // Apply theme to document root
  useEffect(() => {
    const root = document.documentElement;
    const applyTheme = (isDark: boolean) => {
      if (isDark) {
        root.classList.add('dark');
      } else {
        root.classList.remove('dark');
      }
    };

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      applyTheme(mediaQuery.matches);

      const listener = (e: MediaQueryListEvent) => applyTheme(e.matches);
      mediaQuery.addEventListener('change', listener);
      return () => mediaQuery.removeEventListener('change', listener);
    } else {
      applyTheme(theme === 'dark');
    }
  }, [theme]);

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col dark:bg-slate-950">
      <Header onOpenSettings={() => setShowSettings(true)} />

      <main className="flex-1 flex overflow-hidden">
        <TransferList />
        <SessionPanel />
      </main>

      <StatusBar onNewTransfer={() => setShowNewTransfer(true)} />

      <NewTransferDialog
        isOpen={showNewTransfer}
        onClose={() => setShowNewTransfer(false)}
      />

      <SettingsPanel
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
      />
    </div>
  );
}

export default App;
