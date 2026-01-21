// Main App Component

import { useEffect, useState } from 'react';
import { useConversationStore } from './stores/conversationStore';
import { useContactStore } from './stores/contactStore';
import { useNodeStore } from './stores/nodeStore';
import ConversationList from './components/ConversationList';
import ChatView from './components/ChatView';

export default function App() {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const { loadConversations, selectedConversationId } = useConversationStore();
  const { loadContacts } = useContactStore();
  const { startNode, status } = useNodeStore();

  useEffect(() => {
    // Initialize app
    (async () => {
      await startNode();
      await loadConversations();
      await loadContacts();
    })();
  }, [startNode, loadConversations, loadContacts]);

  return (
    <div className="flex h-screen bg-wraith-darker text-white">
      {/* Sidebar */}
      <div
        className={`${
          sidebarOpen ? 'w-80' : 'w-0'
        } transition-all duration-300 bg-wraith-dark border-r border-gray-700 flex flex-col overflow-hidden`}
      >
        <div className="p-4 border-b border-gray-700">
          <h1 className="text-2xl font-bold text-wraith-primary">WRAITH Chat</h1>
          {status && (
            <p className="text-sm text-gray-400 mt-1">
              {status.running ? 'Connected' : 'Offline'}
            </p>
          )}
        </div>

        <ConversationList />
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        {selectedConversationId ? (
          <ChatView conversationId={selectedConversationId} />
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <div className="text-6xl mb-4">💬</div>
              <h2 className="text-2xl font-semibold mb-2">Welcome to WRAITH Chat</h2>
              <p className="text-gray-400">
                Select a conversation to start messaging
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Sidebar Toggle */}
      <button
        onClick={() => setSidebarOpen(!sidebarOpen)}
        className="fixed top-4 left-4 z-50 p-2 rounded bg-wraith-primary hover:bg-wraith-secondary transition"
      >
        {sidebarOpen ? '←' : '→'}
      </button>
    </div>
  );
}
