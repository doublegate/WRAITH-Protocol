// Conversation List Component

import { useConversationStore } from '../stores/conversationStore';
import { formatDistanceToNow } from 'date-fns';

export default function ConversationList() {
  const { conversations, selectedConversationId, selectConversation } =
    useConversationStore();

  return (
    <div className="flex-1 overflow-y-auto">
      {conversations.length === 0 ? (
        <div className="p-4 text-center text-gray-400">
          <p>No conversations yet</p>
          <p className="text-sm mt-2">Start a new chat to begin</p>
        </div>
      ) : (
        <div>
          {conversations.map((conv) => (
            <button
              key={conv.id}
              onClick={() => selectConversation(conv.id)}
              className={`w-full p-4 flex items-start hover:bg-wraith-darker transition ${
                selectedConversationId === conv.id
                  ? 'bg-wraith-primary/20 border-l-4 border-wraith-primary'
                  : 'border-l-4 border-transparent'
              }`}
            >
              {/* Avatar */}
              <div className="w-12 h-12 rounded-full bg-wraith-primary flex items-center justify-center text-xl font-semibold mr-3">
                {(conv.display_name || 'U')[0].toUpperCase()}
              </div>

              {/* Conversation Info */}
              <div className="flex-1 min-w-0">
                <div className="flex justify-between items-baseline mb-1">
                  <h3 className="font-semibold truncate">
                    {conv.display_name || conv.peer_id?.substring(0, 16) || 'Unknown'}
                  </h3>
                  {conv.last_message_at && (
                    <span className="text-xs text-gray-400 ml-2">
                      {formatDistanceToNow(new Date(conv.last_message_at * 1000), {
                        addSuffix: true,
                      })}
                    </span>
                  )}
                </div>

                {conv.unread_count > 0 && (
                  <span className="inline-block px-2 py-0.5 text-xs bg-wraith-accent rounded-full">
                    {conv.unread_count}
                  </span>
                )}
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
