// WRAITH Chat - Main Application (Redesigned)

import { useEffect, useState, useRef } from 'react';
import { useConversationStore } from './stores/conversationStore';
import { useContactStore } from './stores/contactStore';
import { useMessageStore } from './stores/messageStore';
import { useNodeStore } from './stores/nodeStore';
import { useCallStore } from './stores/callStore';
import { useGroupStore } from './stores/groupStore';

// Components
import Header from './components/Header';
import Sidebar from './components/Sidebar';
import ChatHeader from './components/ChatHeader';
import MessageBubble, { DateSeparator } from './components/MessageBubble';
import MessageInput from './components/MessageInput';
import InfoPanel from './components/InfoPanel';
import SettingsModal from './components/SettingsModal';
import NewChatDialog from './components/NewChatDialog';
import NewGroupDialog from './components/NewGroupDialog';
import GroupSettings from './components/GroupSettings';
import VoiceCall from './components/VoiceCall';
import VideoCallOverlay from './components/VideoCallOverlay';

export default function App() {
  // UI State
  const [showSettings, setShowSettings] = useState(false);
  const [showNewChat, setShowNewChat] = useState(false);
  const [showNewGroup, setShowNewGroup] = useState(false);
  const [showGroupSettings, setShowGroupSettings] = useState(false);
  const [showInfoPanel, setShowInfoPanel] = useState(false);
  const [activeVoiceCall, setActiveVoiceCall] = useState<string | null>(null);
  const [activeVideoCall, setActiveVideoCall] = useState<string | null>(null);

  // Stores
  const { conversations, selectedConversationId, loadConversations } = useConversationStore();
  const { loadContacts } = useContactStore();
  const { messages, loadMessages, sendMessage, markAsRead } = useMessageStore();
  const { startNode, refreshStatus, status } = useNodeStore();
  const { incomingCall } = useCallStore();
  const { selectGroup } = useGroupStore();

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Get selected conversation
  const selectedConversation = conversations.find((c) => c.id === selectedConversationId);
  const conversationMessages = selectedConversationId ? messages[selectedConversationId] || [] : [];

  // Initialize app
  useEffect(() => {
    const init = async () => {
      await startNode();
      await loadConversations();
      await loadContacts();
    };
    init();
  }, [startNode, loadConversations, loadContacts]);

  // Refresh status periodically
  useEffect(() => {
    if (!status?.running) return;

    const interval = setInterval(() => {
      refreshStatus();
    }, 5000);

    return () => clearInterval(interval);
  }, [status?.running, refreshStatus]);

  // Load messages when conversation changes
  useEffect(() => {
    if (selectedConversationId) {
      loadMessages(selectedConversationId);
      markAsRead(selectedConversationId);

      // If it's a group, load group members
      const conv = conversations.find((c) => c.id === selectedConversationId);
      if (conv?.conv_type === 'group' && conv.group_id) {
        selectGroup(conv.group_id);
      } else {
        selectGroup(null);
      }
    }
  }, [selectedConversationId, loadMessages, markAsRead, conversations, selectGroup]);

  // Scroll to bottom when messages change
  const messageCount = conversationMessages.length;
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messageCount]);

  // Handle send message
  const handleSendMessage = async (text: string) => {
    if (!selectedConversation) return;

    try {
      if (selectedConversation.peer_id) {
        await sendMessage(selectedConversation.id, selectedConversation.peer_id, text);
      }
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  };

  // Handle voice call
  const handleVoiceCall = () => {
    if (selectedConversation?.peer_id) {
      setActiveVoiceCall(selectedConversation.peer_id);
    }
  };

  // Handle video call
  const handleVideoCall = () => {
    if (selectedConversation?.peer_id) {
      setActiveVideoCall(selectedConversation.peer_id);
    }
  };

  // Group messages by date for display
  const groupedMessages = groupMessagesByDate(conversationMessages);

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col overflow-hidden">
      {/* Header */}
      <Header onOpenSettings={() => setShowSettings(true)} />

      {/* Main Content */}
      <main className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <Sidebar
          onNewChat={() => setShowNewChat(true)}
          onNewGroup={() => setShowNewGroup(true)}
        />

        {/* Chat Area */}
        <div className="flex-1 flex flex-col min-w-0">
          {selectedConversation ? (
            <>
              {/* Chat Header */}
              <ChatHeader
                conversation={selectedConversation}
                onToggleInfoPanel={() => setShowInfoPanel(!showInfoPanel)}
                onVoiceCall={handleVoiceCall}
                onVideoCall={handleVideoCall}
                infoPanelOpen={showInfoPanel}
              />

              {/* Messages */}
              <div className="flex-1 overflow-y-auto px-4 py-2">
                {conversationMessages.length === 0 ? (
                  <div className="h-full flex items-center justify-center">
                    <div className="text-center">
                      <div className="w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center mx-auto mb-4">
                        <MessageIcon className="w-8 h-8 text-slate-500" />
                      </div>
                      <p className="text-slate-400">No messages yet</p>
                      <p className="text-sm text-slate-500 mt-1">
                        Send a message to start the conversation
                      </p>
                    </div>
                  </div>
                ) : (
                  <>
                    {groupedMessages.map((group, groupIndex) => (
                      <div key={groupIndex}>
                        <DateSeparator date={group.date} />
                        {group.messages.map((msg, msgIndex) => {
                          const prevMsg = group.messages[msgIndex - 1];
                          const isGrouped =
                            prevMsg &&
                            prevMsg.direction === msg.direction &&
                            msg.timestamp - prevMsg.timestamp < 60;
                          return (
                            <MessageBubble
                              key={msg.id}
                              message={msg}
                              isGrouped={isGrouped}
                            />
                          );
                        })}
                      </div>
                    ))}
                    <div ref={messagesEndRef} />
                  </>
                )}
              </div>

              {/* Message Input */}
              <MessageInput
                onSend={handleSendMessage}
                disabled={!status?.running}
                placeholder={
                  status?.running
                    ? 'Type a message...'
                    : 'Connect to send messages'
                }
              />
            </>
          ) : (
            // Empty State
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center max-w-md px-4">
                <div className="w-24 h-24 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center mx-auto mb-6">
                  <ChatIcon className="w-12 h-12 text-white" />
                </div>
                <h2 className="text-2xl font-semibold text-white mb-2">
                  Welcome to WRAITH Chat
                </h2>
                <p className="text-slate-400 mb-6">
                  Select a conversation from the sidebar or start a new chat to begin
                  messaging securely.
                </p>
                <div className="flex flex-col sm:flex-row gap-3 justify-center">
                  <button
                    onClick={() => setShowNewChat(true)}
                    className="px-6 py-2.5 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors"
                  >
                    Start New Chat
                  </button>
                  <button
                    onClick={() => setShowNewGroup(true)}
                    className="px-6 py-2.5 bg-bg-tertiary hover:bg-slate-600 rounded-lg text-white font-medium transition-colors"
                  >
                    Create Group
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Info Panel (Right Sidebar) */}
        {showInfoPanel && selectedConversation && (
          <InfoPanel
            conversation={selectedConversation}
            onClose={() => setShowInfoPanel(false)}
            onOpenGroupSettings={
              selectedConversation.conv_type === 'group'
                ? () => setShowGroupSettings(true)
                : undefined
            }
          />
        )}
      </main>

      {/* Modals */}
      <SettingsModal
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
      />

      <NewChatDialog
        isOpen={showNewChat}
        onClose={() => setShowNewChat(false)}
      />

      <NewGroupDialog
        isOpen={showNewGroup}
        onClose={() => setShowNewGroup(false)}
      />

      {showGroupSettings && selectedConversation?.group_id && (
        <GroupSettings
          groupId={selectedConversation.group_id}
          onClose={() => setShowGroupSettings(false)}
        />
      )}

      {/* Call Overlays */}
      {(activeVoiceCall || incomingCall) && (
        <VoiceCall
          peerId={activeVoiceCall || undefined}
          onCallEnd={() => setActiveVoiceCall(null)}
        />
      )}

      {activeVideoCall && (
        <VideoCallOverlay
          peerId={activeVideoCall}
          onCallEnd={() => setActiveVideoCall(null)}
        />
      )}
    </div>
  );
}

// Helper: Group messages by date
interface MessageGroup {
  date: Date;
  messages: Array<{
    id: number;
    conversation_id: number;
    sender_peer_id: string;
    content_type: 'text' | 'media' | 'voice' | 'file';
    body?: string;
    timestamp: number;
    sent: boolean;
    delivered: boolean;
    read_by_me: boolean;
    direction: 'incoming' | 'outgoing';
  }>;
}

function groupMessagesByDate(messages: MessageGroup['messages']): MessageGroup[] {
  const groups: MessageGroup[] = [];
  let currentGroup: MessageGroup | null = null;

  for (const msg of messages) {
    const msgDate = new Date(msg.timestamp * 1000);
    msgDate.setHours(0, 0, 0, 0);

    if (!currentGroup || currentGroup.date.getTime() !== msgDate.getTime()) {
      currentGroup = { date: msgDate, messages: [] };
      groups.push(currentGroup);
    }

    currentGroup.messages.push(msg);
  }

  return groups;
}

// Icons
function MessageIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
      />
    </svg>
  );
}

function ChatIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z"
      />
    </svg>
  );
}
