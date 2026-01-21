// WRAITH Chat - Sidebar Component

import { useState, useMemo } from "react";
import { useConversationStore } from "../stores/conversationStore";
import { formatDistanceToNow } from "date-fns";

type ConversationFilter = "all" | "direct" | "groups";

interface SidebarProps {
  onNewChat: () => void;
  onNewGroup: () => void;
}

export default function Sidebar({ onNewChat, onNewGroup }: SidebarProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [filter, setFilter] = useState<ConversationFilter>("all");

  const { conversations, selectedConversationId, selectConversation } =
    useConversationStore();

  // Filter conversations based on search and filter type
  const filteredConversations = useMemo(() => {
    return conversations.filter((conv) => {
      // Filter by type
      if (filter === "direct" && conv.conv_type !== "direct") return false;
      if (filter === "groups" && conv.conv_type !== "group") return false;

      // Filter by search query
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const name = (
          conv.display_name ||
          conv.peer_id ||
          conv.group_id ||
          ""
        ).toLowerCase();
        return name.includes(query);
      }

      return true;
    });
  }, [conversations, filter, searchQuery]);

  // Count by type
  const directCount = conversations.filter(
    (c) => c.conv_type === "direct",
  ).length;
  const groupCount = conversations.filter(
    (c) => c.conv_type === "group",
  ).length;

  return (
    <div className="w-80 bg-bg-secondary border-r border-slate-700 flex flex-col h-full">
      {/* Search and Actions */}
      <div className="p-4 space-y-3 border-b border-slate-700">
        {/* Search */}
        <div className="relative">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search conversations..."
            className="w-full bg-bg-primary border border-slate-600 rounded-lg pl-10 pr-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary"
          />
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2">
          <button
            onClick={onNewChat}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white text-sm font-medium transition-colors"
          >
            <PlusIcon className="w-4 h-4" />
            New Chat
          </button>
          <button
            onClick={onNewGroup}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-bg-tertiary hover:bg-slate-600 rounded-lg text-white text-sm font-medium transition-colors"
          >
            <UsersIcon className="w-4 h-4" />
            New Group
          </button>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex border-b border-slate-700">
        <FilterTab
          label="All"
          count={conversations.length}
          active={filter === "all"}
          onClick={() => setFilter("all")}
        />
        <FilterTab
          label="Direct"
          count={directCount}
          active={filter === "direct"}
          onClick={() => setFilter("direct")}
        />
        <FilterTab
          label="Groups"
          count={groupCount}
          active={filter === "groups"}
          onClick={() => setFilter("groups")}
        />
      </div>

      {/* Conversation List */}
      <div className="flex-1 overflow-y-auto">
        {filteredConversations.length === 0 ? (
          <div className="p-6 text-center">
            {searchQuery ? (
              <>
                <SearchIcon className="w-12 h-12 text-slate-600 mx-auto mb-3" />
                <p className="text-slate-400">No conversations found</p>
                <p className="text-sm text-slate-500 mt-1">
                  Try a different search term
                </p>
              </>
            ) : (
              <>
                <MessageIcon className="w-12 h-12 text-slate-600 mx-auto mb-3" />
                <p className="text-slate-400">No conversations yet</p>
                <p className="text-sm text-slate-500 mt-1">
                  Start a new chat to begin messaging
                </p>
              </>
            )}
          </div>
        ) : (
          <div className="divide-y divide-slate-700/50">
            {filteredConversations.map((conv) => (
              <ConversationItem
                key={conv.id}
                conversation={conv}
                isSelected={selectedConversationId === conv.id}
                onClick={() => selectConversation(conv.id)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// Filter Tab Component
interface FilterTabProps {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}

function FilterTab({ label, count, active, onClick }: FilterTabProps) {
  return (
    <button
      onClick={onClick}
      className={`flex-1 py-2.5 text-sm font-medium transition-colors relative ${
        active ? "text-wraith-primary" : "text-slate-400 hover:text-slate-200"
      }`}
    >
      {label}
      {count > 0 && (
        <span
          className={`ml-1.5 text-xs ${
            active ? "text-wraith-primary" : "text-slate-500"
          }`}
        >
          ({count})
        </span>
      )}
      {active && (
        <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-wraith-primary" />
      )}
    </button>
  );
}

// Conversation Item Component
interface ConversationItemProps {
  conversation: {
    id: number;
    conv_type: "direct" | "group";
    peer_id?: string;
    group_id?: string;
    display_name?: string;
    last_message_at?: number;
    unread_count: number;
    muted: boolean;
  };
  isSelected: boolean;
  onClick: () => void;
}

function ConversationItem({
  conversation,
  isSelected,
  onClick,
}: ConversationItemProps) {
  const name =
    conversation.display_name ||
    (conversation.peer_id
      ? `${conversation.peer_id.substring(0, 12)}...`
      : null) ||
    (conversation.group_id
      ? `Group ${conversation.group_id.substring(0, 8)}`
      : "Unknown");

  const isGroup = conversation.conv_type === "group";
  const initial = name[0]?.toUpperCase() || "?";

  return (
    <button
      onClick={onClick}
      className={`w-full p-3 flex items-center gap-3 transition-colors text-left ${
        isSelected
          ? "bg-wraith-primary/20 border-l-2 border-wraith-primary"
          : "hover:bg-bg-primary border-l-2 border-transparent"
      }`}
    >
      {/* Avatar */}
      <div className="relative flex-shrink-0">
        <div
          className={`w-12 h-12 rounded-full flex items-center justify-center text-lg font-semibold ${
            isGroup
              ? "bg-gradient-to-br from-purple-500 to-pink-500"
              : "bg-gradient-to-br from-wraith-primary to-wraith-secondary"
          }`}
        >
          {isGroup ? (
            <UsersIcon className="w-5 h-5 text-white" />
          ) : (
            <span className="text-white">{initial}</span>
          )}
        </div>
        {/* Online indicator (for direct chats) */}
        {!isGroup && (
          <div className="absolute bottom-0 right-0 w-3 h-3 bg-green-500 rounded-full border-2 border-bg-secondary" />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-0.5">
          <h3 className="font-medium text-white truncate">{name}</h3>
          {conversation.last_message_at && (
            <span className="text-xs text-slate-500 ml-2 flex-shrink-0">
              {formatDistanceToNow(
                new Date(conversation.last_message_at * 1000),
                {
                  addSuffix: false,
                },
              )}
            </span>
          )}
        </div>
        <div className="flex items-center justify-between">
          <p className="text-sm text-slate-400 truncate">
            {isGroup
              ? `${conversation.group_id ? "Group chat" : "No messages"}`
              : "Start a conversation"}
          </p>
          <div className="flex items-center gap-2 flex-shrink-0">
            {conversation.muted && (
              <MuteIcon className="w-3.5 h-3.5 text-slate-500" />
            )}
            {conversation.unread_count > 0 && (
              <span className="min-w-[20px] h-5 px-1.5 flex items-center justify-center bg-wraith-accent text-white text-xs font-medium rounded-full">
                {conversation.unread_count > 99
                  ? "99+"
                  : conversation.unread_count}
              </span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}

// Icons
function SearchIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>
  );
}

function PlusIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 4v16m8-8H4"
      />
    </svg>
  );
}

function UsersIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"
      />
    </svg>
  );
}

function MessageIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
      />
    </svg>
  );
}

function MuteIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2"
      />
    </svg>
  );
}
