// WRAITH Chat - Message Input Component

import { useState, useRef, useEffect } from "react";

interface MessageInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export default function MessageInput({
  onSend,
  disabled = false,
  placeholder = "Type a message...",
}: MessageInputProps) {
  const [message, setMessage] = useState("");
  const [showEmojiPicker, setShowEmojiPicker] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 150)}px`;
    }
  }, [message]);

  const handleSubmit = () => {
    if (!message.trim() || disabled) return;
    onSend(message.trim());
    setMessage("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Common emoji shortcuts
  const quickEmojis = ["thumbsup", "heart", "smile", "check"];

  return (
    <div className="px-4 py-3 bg-bg-secondary border-t border-slate-700">
      {/* Emoji Picker Placeholder */}
      {showEmojiPicker && (
        <div className="mb-3 p-4 bg-bg-primary rounded-lg border border-slate-700">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm font-medium text-slate-300">
              Quick Reactions
            </span>
            <button
              onClick={() => setShowEmojiPicker(false)}
              className="text-slate-500 hover:text-white"
            >
              <CloseIcon className="w-4 h-4" />
            </button>
          </div>
          <div className="flex gap-2">
            {quickEmojis.map((emoji) => (
              <button
                key={emoji}
                onClick={() => {
                  setMessage((prev) => prev + getEmojiChar(emoji));
                  setShowEmojiPicker(false);
                }}
                className="w-10 h-10 flex items-center justify-center text-xl bg-bg-tertiary hover:bg-slate-600 rounded-lg transition-colors"
              >
                {getEmojiChar(emoji)}
              </button>
            ))}
          </div>
          <p className="text-xs text-slate-500 mt-3">
            Full emoji picker coming soon
          </p>
        </div>
      )}

      {/* Input Area */}
      <div className="flex items-end gap-2">
        {/* Attachment Button */}
        <button
          className="p-2.5 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors flex-shrink-0"
          title="Attach file (coming soon)"
          aria-label="Attach file"
        >
          <AttachmentIcon className="w-5 h-5" />
        </button>

        {/* Text Input */}
        <div className="flex-1 relative">
          <textarea
            ref={textareaRef}
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            disabled={disabled}
            rows={1}
            className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 pr-12 text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary resize-none disabled:opacity-50 disabled:cursor-not-allowed"
            style={{ minHeight: "44px", maxHeight: "150px" }}
          />
          {/* Emoji Button */}
          <button
            onClick={() => setShowEmojiPicker(!showEmojiPicker)}
            className={`absolute right-3 bottom-2.5 p-1 rounded transition-colors ${
              showEmojiPicker
                ? "text-wraith-primary"
                : "text-slate-400 hover:text-white"
            }`}
            title="Add emoji"
            aria-label="Add emoji"
          >
            <EmojiIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Send / Voice Button */}
        {message.trim() ? (
          <button
            onClick={handleSubmit}
            disabled={disabled}
            className="p-2.5 bg-wraith-primary hover:bg-wraith-secondary text-white rounded-lg transition-colors flex-shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
            title="Send message"
            aria-label="Send message"
          >
            <SendIcon className="w-5 h-5" />
          </button>
        ) : (
          <button
            className="p-2.5 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors flex-shrink-0"
            title="Send voice message (coming soon)"
            aria-label="Send voice message"
          >
            <MicIcon className="w-5 h-5" />
          </button>
        )}
      </div>

      {/* Typing Hint */}
      <div className="flex items-center justify-between mt-2 px-1">
        <span className="text-xs text-slate-500">
          Press Enter to send, Shift+Enter for new line
        </span>
        <span className="text-xs text-slate-500">
          {message.length > 0 && `${message.length}/10000`}
        </span>
      </div>
    </div>
  );
}

// Helper to get emoji character from name
function getEmojiChar(name: string): string {
  const emojis: Record<string, string> = {
    thumbsup: "\uD83D\uDC4D",
    heart: "\u2764\uFE0F",
    smile: "\uD83D\uDE0A",
    check: "\u2705",
  };
  return emojis[name] || "";
}

// Icons
function AttachmentIcon({ className }: { className?: string }) {
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
        d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"
      />
    </svg>
  );
}

function EmojiIcon({ className }: { className?: string }) {
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
        d="M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

function SendIcon({ className }: { className?: string }) {
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
        d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
      />
    </svg>
  );
}

function MicIcon({ className }: { className?: string }) {
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
        d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
      />
    </svg>
  );
}

function CloseIcon({ className }: { className?: string }) {
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
        d="M6 18L18 6M6 6l12 12"
      />
    </svg>
  );
}
