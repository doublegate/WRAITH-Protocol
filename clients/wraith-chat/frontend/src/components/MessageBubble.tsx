// WRAITH Chat - Message Bubble Component (Improved)

import type { Message } from '../types';
import { format } from 'date-fns';

interface MessageBubbleProps {
  message: Message;
  showAvatar?: boolean;
  isGrouped?: boolean;
}

export default function MessageBubble({
  message,
  showAvatar = true,
  isGrouped = false,
}: MessageBubbleProps) {
  const isOutgoing = message.direction === 'outgoing';
  const initial = message.sender_peer_id?.[0]?.toUpperCase() || '?';

  return (
    <div
      className={`flex gap-2 ${isOutgoing ? 'justify-end' : 'justify-start'} ${
        isGrouped ? 'mt-0.5' : 'mt-3'
      }`}
    >
      {/* Avatar (for incoming messages) */}
      {!isOutgoing && showAvatar && !isGrouped && (
        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-sm font-semibold text-white flex-shrink-0">
          {initial}
        </div>
      )}
      {!isOutgoing && showAvatar && isGrouped && <div className="w-8 flex-shrink-0" />}

      {/* Message Content */}
      <div
        className={`group relative max-w-[70%] ${
          isOutgoing ? 'order-first' : ''
        }`}
      >
        <div
          className={`rounded-2xl px-4 py-2.5 ${
            isOutgoing
              ? 'bg-wraith-primary text-white rounded-br-md'
              : 'bg-bg-tertiary text-white rounded-bl-md'
          }`}
        >
          {/* Message Body */}
          <p className="text-sm break-words whitespace-pre-wrap leading-relaxed">
            {message.body}
          </p>

          {/* Footer: Time and Status */}
          <div
            className={`flex items-center gap-1.5 mt-1.5 text-xs ${
              isOutgoing ? 'justify-end text-white/70' : 'justify-end text-slate-400'
            }`}
          >
            <span>{format(new Date(message.timestamp * 1000), 'HH:mm')}</span>
            {isOutgoing && <MessageStatus message={message} />}
          </div>
        </div>

        {/* Context Menu (shows on hover) */}
        <div
          className={`absolute top-0 ${
            isOutgoing ? 'left-0 -translate-x-full pr-2' : 'right-0 translate-x-full pl-2'
          } opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1`}
        >
          <button
            className="p-1.5 text-slate-500 hover:text-white hover:bg-bg-tertiary rounded transition-colors"
            title="Reply"
          >
            <ReplyIcon className="w-4 h-4" />
          </button>
          <button
            className="p-1.5 text-slate-500 hover:text-white hover:bg-bg-tertiary rounded transition-colors"
            title="React"
          >
            <EmojiIcon className="w-4 h-4" />
          </button>
          <button
            className="p-1.5 text-slate-500 hover:text-white hover:bg-bg-tertiary rounded transition-colors"
            title="More"
          >
            <MoreIcon className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Avatar spacer (for outgoing messages alignment) */}
      {isOutgoing && showAvatar && !isGrouped && <div className="w-8 flex-shrink-0" />}
      {isOutgoing && showAvatar && isGrouped && <div className="w-8 flex-shrink-0" />}
    </div>
  );
}

// Message Status Indicator
function MessageStatus({ message }: { message: Message }) {
  if (message.read_by_me) {
    // Read (double check, blue)
    return (
      <span className="flex text-blue-300" title="Read">
        <CheckIcon className="w-3.5 h-3.5" />
        <CheckIcon className="w-3.5 h-3.5 -ml-2" />
      </span>
    );
  }

  if (message.delivered) {
    // Delivered (double check)
    return (
      <span className="flex" title="Delivered">
        <CheckIcon className="w-3.5 h-3.5" />
        <CheckIcon className="w-3.5 h-3.5 -ml-2" />
      </span>
    );
  }

  if (message.sent) {
    // Sent (single check)
    return (
      <span title="Sent">
        <CheckIcon className="w-3.5 h-3.5" />
      </span>
    );
  }

  // Pending (clock)
  return (
    <span title="Sending...">
      <ClockIcon className="w-3.5 h-3.5" />
    </span>
  );
}

// System Message Component (for dates, join/leave notifications, etc.)
export function SystemMessage({ text }: { text: string }) {
  return (
    <div className="flex justify-center my-4">
      <span className="px-3 py-1 bg-bg-tertiary/50 rounded-full text-xs text-slate-400">
        {text}
      </span>
    </div>
  );
}

// Date Separator Component
export function DateSeparator({ date }: { date: Date }) {
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  let dateText: string;
  if (date.toDateString() === today.toDateString()) {
    dateText = 'Today';
  } else if (date.toDateString() === yesterday.toDateString()) {
    dateText = 'Yesterday';
  } else {
    dateText = format(date, 'MMMM d, yyyy');
  }

  return (
    <div className="flex items-center my-6">
      <div className="flex-1 border-t border-slate-700/50" />
      <span className="px-4 text-xs text-slate-500">{dateText}</span>
      <div className="flex-1 border-t border-slate-700/50" />
    </div>
  );
}

// Icons
function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M5 13l4 4L19 7" />
    </svg>
  );
}

function ClockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function ReplyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
    </svg>
  );
}

function EmojiIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function MoreIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z" />
    </svg>
  );
}
