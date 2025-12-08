// WRAITH Transfer - Main Application

import { useEffect, useState } from 'react';
import { Header, TransferList, SessionPanel, NewTransferDialog, StatusBar } from './components';
import { useNodeStore } from './stores/nodeStore';
import { useTransferStore } from './stores/transferStore';
import { useSessionStore } from './stores/sessionStore';

function App() {
  const [showNewTransfer, setShowNewTransfer] = useState(false);

  const { fetchStatus, status } = useNodeStore();
  const { fetchTransfers } = useTransferStore();
  const { fetchSessions } = useSessionStore();

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

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col">
      <Header />

      <main className="flex-1 flex overflow-hidden">
        <TransferList />
        <SessionPanel />
      </main>

      <StatusBar onNewTransfer={() => setShowNewTransfer(true)} />

      <NewTransferDialog
        isOpen={showNewTransfer}
        onClose={() => setShowNewTransfer(false)}
      />
    </div>
  );
}

export default App;
