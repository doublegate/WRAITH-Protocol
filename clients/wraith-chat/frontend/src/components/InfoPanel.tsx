// WRAITH Chat - Info Panel Component (Right Sidebar)

import { useState } from 'react';
import type { Conversation } from '../types';
import { useGroupStore, formatMemberCount } from '../stores/groupStore';

interface InfoPanelProps {
  conversation: Conversation;
  onClose: () => void;
  onOpenGroupSettings?: () => void;
}

export default function InfoPanel({
  conversation,
  onClose,
  onOpenGroupSettings,
}: InfoPanelProps) {
  const [copied, setCopied] = useState(false);
  const { currentGroupMembers } = useGroupStore();

  const isGroup = conversation.conv_type === 'group';
  const name = conversation.display_name ||
    (conversation.peer_id ? `${conversation.peer_id.substring(0, 16)}...` : null) ||
    (conversation.group_id ? `Group` : 'Unknown');

  const initial = name[0]?.toUpperCase() || '?';
  const peerId = conversation.peer_id || conversation.group_id || '';

  const handleCopyId = async () => {
    try {
      await navigator.clipboard.writeText(peerId);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div className="w-80 bg-bg-secondary border-l border-slate-700 flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700">
        <h3 className="font-semibold text-white">
          {isGroup ? 'Group Info' : 'Contact Info'}
        </h3>
        <button
          onClick={onClose}
          className="p-1.5 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
          aria-label="Close panel"
        >
          <CloseIcon className="w-5 h-5" />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {/* Profile Section */}
        <div className="p-6 text-center border-b border-slate-700">
          {/* Avatar */}
          <div
            className={`w-20 h-20 rounded-full flex items-center justify-center text-3xl font-bold mx-auto mb-4 ${
              isGroup
                ? 'bg-gradient-to-br from-purple-500 to-pink-500'
                : 'bg-gradient-to-br from-wraith-primary to-wraith-secondary'
            }`}
          >
            {isGroup ? (
              <UsersIcon className="w-8 h-8 text-white" />
            ) : (
              <span className="text-white">{initial}</span>
            )}
          </div>
          <h2 className="text-lg font-semibold text-white mb-1">{name}</h2>
          {!isGroup && (
            <p className="text-sm text-green-400 flex items-center justify-center gap-1">
              <span className="w-2 h-2 bg-green-500 rounded-full" />
              Online
            </p>
          )}
          {isGroup && (
            <p className="text-sm text-slate-400">
              {formatMemberCount(currentGroupMembers.length)}
            </p>
          )}
        </div>

        {/* ID Section */}
        <div className="p-4 border-b border-slate-700">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-slate-400">
              {isGroup ? 'Group ID' : 'Peer ID'}
            </span>
            <button
              onClick={handleCopyId}
              className="text-xs text-wraith-primary hover:text-wraith-secondary flex items-center gap-1"
            >
              {copied ? 'Copied!' : 'Copy'}
              <CopyIcon className="w-3.5 h-3.5" />
            </button>
          </div>
          <p className="text-sm font-mono text-slate-300 break-all bg-bg-primary p-2 rounded">
            {peerId}
          </p>
        </div>

        {/* Encryption Info */}
        <div className="p-4 border-b border-slate-700">
          <div className="flex items-center gap-3 p-3 bg-bg-primary rounded-lg">
            <div className="w-10 h-10 rounded-full bg-green-500/20 flex items-center justify-center flex-shrink-0">
              <LockIcon className="w-5 h-5 text-green-500" />
            </div>
            <div>
              <p className="text-sm font-medium text-white">Encrypted</p>
              <p className="text-xs text-slate-400">
                {isGroup ? 'Sender Keys Protocol' : 'Double Ratchet'}
              </p>
            </div>
          </div>
        </div>

        {/* Group Members (for groups) */}
        {isGroup && currentGroupMembers.length > 0 && (
          <div className="p-4 border-b border-slate-700">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-slate-300">
                Members ({currentGroupMembers.length})
              </span>
              {onOpenGroupSettings && (
                <button
                  onClick={onOpenGroupSettings}
                  className="text-xs text-wraith-primary hover:text-wraith-secondary"
                >
                  Manage
                </button>
              )}
            </div>
            <div className="space-y-2">
              {currentGroupMembers.slice(0, 5).map((member) => (
                <div
                  key={member.peer_id}
                  className="flex items-center gap-3 p-2 bg-bg-primary rounded-lg"
                >
                  <div className="w-8 h-8 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-sm font-semibold text-white">
                    {(member.display_name || member.peer_id)[0].toUpperCase()}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-white truncate">
                      {member.display_name || `${member.peer_id.substring(0, 12)}...`}
                    </p>
                    <p className="text-xs text-slate-500 capitalize">{member.role}</p>
                  </div>
                </div>
              ))}
              {currentGroupMembers.length > 5 && (
                <button
                  onClick={onOpenGroupSettings}
                  className="w-full p-2 text-sm text-slate-400 hover:text-white text-center"
                >
                  View all {currentGroupMembers.length} members
                </button>
              )}
            </div>
          </div>
        )}

        {/* Shared Media Placeholder */}
        <div className="p-4 border-b border-slate-700">
          <span className="text-sm font-medium text-slate-300 block mb-3">
            Shared Media
          </span>
          <div className="grid grid-cols-3 gap-2">
            {[1, 2, 3].map((i) => (
              <div
                key={i}
                className="aspect-square bg-bg-primary rounded-lg flex items-center justify-center"
              >
                <ImageIcon className="w-6 h-6 text-slate-600" />
              </div>
            ))}
          </div>
          <p className="text-xs text-slate-500 text-center mt-3">
            Media sharing coming soon
          </p>
        </div>

        {/* Actions */}
        <div className="p-4 space-y-2">
          {/* Mute */}
          <button className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-bg-tertiary rounded-lg transition-colors text-left">
            {conversation.muted ? (
              <BellIcon className="w-5 h-5 text-slate-400" />
            ) : (
              <BellOffIcon className="w-5 h-5 text-slate-400" />
            )}
            <span className="text-sm text-slate-300">
              {conversation.muted ? 'Unmute Notifications' : 'Mute Notifications'}
            </span>
          </button>

          {/* Search */}
          <button className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-bg-tertiary rounded-lg transition-colors text-left">
            <SearchIcon className="w-5 h-5 text-slate-400" />
            <span className="text-sm text-slate-300">Search in Conversation</span>
          </button>

          {/* Group Settings (for groups) */}
          {isGroup && onOpenGroupSettings && (
            <button
              onClick={onOpenGroupSettings}
              className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-bg-tertiary rounded-lg transition-colors text-left"
            >
              <SettingsIcon className="w-5 h-5 text-slate-400" />
              <span className="text-sm text-slate-300">Group Settings</span>
            </button>
          )}

          {/* Danger Zone */}
          <div className="pt-4 mt-4 border-t border-slate-700 space-y-2">
            {!isGroup && (
              <button className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-red-500/20 rounded-lg transition-colors text-left group">
                <BlockIcon className="w-5 h-5 text-slate-400 group-hover:text-red-400" />
                <span className="text-sm text-slate-300 group-hover:text-red-400">
                  Block Contact
                </span>
              </button>
            )}

            {isGroup && (
              <button className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-red-500/20 rounded-lg transition-colors text-left group">
                <ExitIcon className="w-5 h-5 text-slate-400 group-hover:text-red-400" />
                <span className="text-sm text-slate-300 group-hover:text-red-400">
                  Leave Group
                </span>
              </button>
            )}

            <button className="w-full flex items-center gap-3 p-3 bg-bg-primary hover:bg-red-500/20 rounded-lg transition-colors text-left group">
              <TrashIcon className="w-5 h-5 text-slate-400 group-hover:text-red-400" />
              <span className="text-sm text-slate-300 group-hover:text-red-400">
                Delete Conversation
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// Icons
function CloseIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function UsersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function CopyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
    </svg>
  );
}

function LockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
    </svg>
  );
}

function ImageIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
    </svg>
  );
}

function BellIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
    </svg>
  );
}

function BellOffIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
    </svg>
  );
}

function SearchIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  );
}

function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
  );
}

function BlockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
    </svg>
  );
}

function ExitIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
    </svg>
  );
}

function TrashIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
    </svg>
  );
}
