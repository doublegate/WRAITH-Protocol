// Message Bubble Component

import type { Message } from '../types';
import { format } from 'date-fns';

interface MessageBubbleProps {
  message: Message;
}

export default function MessageBubble({ message }: MessageBubbleProps) {
  const isOutgoing = message.direction === 'outgoing';

  return (
    <div className={`flex ${isOutgoing ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[70%] rounded-lg p-3 ${
          isOutgoing
            ? 'bg-wraith-primary text-white'
            : 'bg-gray-700 text-white'
        }`}
      >
        <p className="break-words whitespace-pre-wrap">{message.body}</p>
        <div className="flex items-center justify-end gap-2 mt-1 text-xs opacity-70">
          <span>{format(new Date(message.timestamp * 1000), 'HH:mm')}</span>
          {isOutgoing && (
            <span>
              {message.read_by_me ? (
                '✓✓'
              ) : message.delivered ? (
                '✓'
              ) : message.sent ? (
                '○'
              ) : (
                '⏳'
              )}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
