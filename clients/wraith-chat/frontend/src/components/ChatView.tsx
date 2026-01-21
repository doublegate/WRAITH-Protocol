// Chat View Component

import React, { useEffect, useState, useRef } from "react";
import { useConversationStore } from "../stores/conversationStore";
import { useMessageStore } from "../stores/messageStore";
import MessageBubble from "./MessageBubble";

interface ChatViewProps {
  conversationId: number;
}

export default function ChatView({ conversationId }: ChatViewProps) {
  const [inputText, setInputText] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const { conversations } = useConversationStore();
  const { messages, loadMessages, sendMessage, markAsRead } = useMessageStore();

  const conversation = conversations.find((c) => c.id === conversationId);
  const conversationMessages = messages[conversationId] || [];

  useEffect(() => {
    loadMessages(conversationId);
    markAsRead(conversationId);
  }, [conversationId, loadMessages, markAsRead]);

  // Scroll to bottom when messages change
  const messageCount = conversationMessages.length;
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messageCount]);

  const handleSend = async () => {
    if (!inputText.trim() || !conversation?.peer_id) return;

    try {
      await sendMessage(conversationId, conversation.peer_id, inputText);
      setInputText("");
    } catch (error) {
      console.error("Failed to send message:", error);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex-1 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-slate-700 bg-bg-secondary">
        <h2 className="text-xl font-semibold">
          {conversation?.display_name || "Unknown"}
        </h2>
        {conversation?.peer_id && (
          <p className="text-sm text-slate-400 font-mono">
            {conversation.peer_id.substring(0, 32)}...
          </p>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {conversationMessages.length === 0 ? (
          <div className="text-center text-slate-400 mt-8">
            <p>No messages yet</p>
            <p className="text-sm mt-2">
              Send a message to start the conversation
            </p>
          </div>
        ) : (
          conversationMessages.map((msg) => (
            <MessageBubble key={msg.id} message={msg} />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-slate-700 bg-bg-secondary">
        <div className="flex gap-2">
          <textarea
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Type a message..."
            className="flex-1 p-3 rounded-lg bg-bg-primary border border-slate-600 focus:border-wraith-primary focus:outline-none resize-none"
            rows={1}
            maxLength={10000}
          />
          <button
            onClick={handleSend}
            disabled={!inputText.trim()}
            className="px-6 py-3 bg-wraith-primary hover:bg-wraith-secondary disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg font-semibold transition"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
