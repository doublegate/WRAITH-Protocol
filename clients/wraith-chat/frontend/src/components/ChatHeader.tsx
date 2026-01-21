// WRAITH Chat - Chat Header Component

import { useCallStore } from "../stores/callStore";
import type { Conversation } from "../types";

interface ChatHeaderProps {
  conversation: Conversation;
  onToggleInfoPanel: () => void;
  onVoiceCall: () => void;
  onVideoCall: () => void;
  infoPanelOpen: boolean;
}

export default function ChatHeader({
  conversation,
  onToggleInfoPanel,
  onVoiceCall,
  onVideoCall,
  infoPanelOpen,
}: ChatHeaderProps) {
  const { activeCall, loading } = useCallStore();

  const isGroup = conversation.conv_type === "group";
  const name =
    conversation.display_name ||
    (conversation.peer_id
      ? `${conversation.peer_id.substring(0, 16)}...`
      : null) ||
    (conversation.group_id ? `Group` : "Unknown");

  const initial = name[0]?.toUpperCase() || "?";
  const hasActiveCall = !!activeCall;

  return (
    <div className="flex items-center justify-between px-4 py-3 bg-bg-secondary border-b border-slate-700">
      {/* Left: Contact/Group Info */}
      <div className="flex items-center gap-3">
        {/* Avatar */}
        <div className="relative">
          <div
            className={`w-10 h-10 rounded-full flex items-center justify-center text-lg font-semibold ${
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
          {/* Online status for direct chats */}
          {!isGroup && (
            <div className="absolute bottom-0 right-0 w-2.5 h-2.5 bg-green-500 rounded-full border-2 border-bg-secondary" />
          )}
        </div>

        {/* Name and Status */}
        <div>
          <h2 className="font-semibold text-white">{name}</h2>
          <p className="text-xs text-slate-400">
            {isGroup ? (
              "Group chat"
            ) : (
              <span className="flex items-center gap-1">
                <span className="w-1.5 h-1.5 bg-green-500 rounded-full" />
                Online
              </span>
            )}
          </p>
        </div>
      </div>

      {/* Right: Action Buttons */}
      <div className="flex items-center gap-1">
        {/* Voice Call Button */}
        {!isGroup && (
          <button
            onClick={onVoiceCall}
            disabled={hasActiveCall || loading}
            className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title="Start voice call"
            aria-label="Start voice call"
          >
            <PhoneIcon className="w-5 h-5" />
          </button>
        )}

        {/* Video Call Button */}
        {!isGroup && (
          <button
            onClick={onVideoCall}
            disabled={hasActiveCall || loading}
            className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title="Start video call"
            aria-label="Start video call"
          >
            <VideoIcon className="w-5 h-5" />
          </button>
        )}

        {/* Search in Chat (placeholder) */}
        <button
          className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
          title="Search in conversation"
          aria-label="Search in conversation"
        >
          <SearchIcon className="w-5 h-5" />
        </button>

        {/* Info Panel Toggle */}
        <button
          onClick={onToggleInfoPanel}
          className={`p-2 rounded-lg transition-colors ${
            infoPanelOpen
              ? "text-wraith-primary bg-wraith-primary/20"
              : "text-slate-400 hover:text-white hover:bg-bg-tertiary"
          }`}
          title={infoPanelOpen ? "Hide details" : "Show details"}
          aria-label={infoPanelOpen ? "Hide details" : "Show details"}
        >
          <InfoIcon className="w-5 h-5" />
        </button>

        {/* More Options */}
        <button
          className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
          title="More options"
          aria-label="More options"
        >
          <MoreIcon className="w-5 h-5" />
        </button>
      </div>
    </div>
  );
}

// Icons
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

function PhoneIcon({ className }: { className?: string }) {
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
        d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z"
      />
    </svg>
  );
}

function VideoIcon({ className }: { className?: string }) {
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
        d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
      />
    </svg>
  );
}

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

function InfoIcon({ className }: { className?: string }) {
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
        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

function MoreIcon({ className }: { className?: string }) {
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
        d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"
      />
    </svg>
  );
}
