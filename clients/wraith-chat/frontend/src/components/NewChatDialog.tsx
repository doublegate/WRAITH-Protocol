// New Chat Dialog Component

import { useState } from "react";
import { useConversationStore } from "../stores/conversationStore";
import { useContactStore } from "../stores/contactStore";
import { useNodeStore } from "../stores/nodeStore";

interface NewChatDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function NewChatDialog({ isOpen, onClose }: NewChatDialogProps) {
  const [peerId, setPeerId] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const { createConversation, selectConversation } = useConversationStore();
  const { addContact, contacts } = useContactStore();
  const { status } = useNodeStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    const trimmedPeerId = peerId.trim();
    if (!trimmedPeerId) {
      setError("Peer ID is required");
      return;
    }

    // Validate peer ID format (should be a hex string)
    if (!/^[0-9a-fA-F]{32,}$/.test(trimmedPeerId)) {
      setError("Invalid Peer ID format. Must be a 32+ character hex string.");
      return;
    }

    // Check if trying to chat with self
    if (status?.local_peer_id && trimmedPeerId === status.local_peer_id) {
      setError("Cannot create a conversation with yourself");
      return;
    }

    setLoading(true);

    try {
      // Check if contact already exists
      const existingContact = contacts.find((c) => c.peer_id === trimmedPeerId);

      if (!existingContact) {
        // Create a contact first with a placeholder identity key
        // In a real implementation, we would exchange keys via the protocol
        const placeholderKey = Array(32).fill(0);
        await addContact(trimmedPeerId, displayName || null, placeholderKey);
      }

      // Create the conversation
      const conversationId = await createConversation(
        "direct",
        trimmedPeerId,
        null,
        displayName || null,
      );

      // Select the new conversation
      selectConversation(conversationId);

      // Reset form and close
      setPeerId("");
      setDisplayName("");
      onClose();
    } catch (err) {
      setError((err as Error).message || "Failed to create conversation");
    } finally {
      setLoading(false);
    }
  };

  const handleStartWithContact = async (contact: (typeof contacts)[0]) => {
    setLoading(true);
    setError(null);

    try {
      const conversationId = await createConversation(
        "direct",
        contact.peer_id,
        null,
        contact.display_name || null,
      );
      selectConversation(conversationId);
      onClose();
    } catch (err) {
      setError((err as Error).message || "Failed to create conversation");
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-chat-title"
    >
      <div
        className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-md p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex justify-between items-center mb-4">
          <h2 id="new-chat-title" className="text-xl font-semibold text-white">
            New Conversation
          </h2>
          <button
            onClick={onClose}
            className="p-1 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
            aria-label="Close dialog"
          >
            <svg
              className="w-5 h-5"
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
          </button>
        </div>

        {/* Your Peer ID */}
        {status?.local_peer_id && (
          <div className="mb-4 p-3 bg-bg-primary rounded-lg">
            <p className="text-sm text-slate-400 mb-1">Your Peer ID:</p>
            <p className="font-mono text-xs text-wraith-primary break-all select-all">
              {status.local_peer_id}
            </p>
            <button
              onClick={() =>
                navigator.clipboard.writeText(status.local_peer_id)
              }
              className="mt-2 text-xs text-slate-400 hover:text-white transition"
            >
              Copy to clipboard
            </button>
          </div>
        )}

        {/* Existing Contacts */}
        {contacts.length > 0 && (
          <div className="mb-4">
            <p className="text-sm text-slate-400 mb-2">
              Start chat with contact:
            </p>
            <div className="max-h-32 overflow-y-auto space-y-1">
              {contacts.map((contact) => (
                <button
                  key={contact.id}
                  onClick={() => handleStartWithContact(contact)}
                  disabled={loading}
                  className="w-full p-2 text-left rounded bg-bg-primary hover:bg-wraith-primary/20 transition disabled:opacity-50"
                >
                  <span className="font-medium">
                    {contact.display_name ||
                      contact.peer_id.substring(0, 16) + "..."}
                  </span>
                </button>
              ))}
            </div>
            <div className="my-4 flex items-center">
              <div className="flex-1 border-t border-slate-600" />
              <span className="px-3 text-sm text-slate-400">
                or enter peer ID
              </span>
              <div className="flex-1 border-t border-slate-600" />
            </div>
          </div>
        )}

        {/* New Contact Form */}
        <form onSubmit={handleSubmit}>
          <div className="mb-4">
            <label
              htmlFor="peerId"
              className="block text-sm font-medium text-slate-300 mb-1"
            >
              Peer ID *
            </label>
            <input
              id="peerId"
              type="text"
              value={peerId}
              onChange={(e) => setPeerId(e.target.value)}
              placeholder="Enter peer ID (hex string)"
              className="w-full p-3 rounded-lg bg-bg-primary border border-slate-600 focus:border-wraith-primary focus:outline-none text-white font-mono text-sm"
              disabled={loading}
            />
          </div>

          <div className="mb-4">
            <label
              htmlFor="displayName"
              className="block text-sm font-medium text-slate-300 mb-1"
            >
              Display Name (optional)
            </label>
            <input
              id="displayName"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Enter a friendly name"
              className="w-full p-3 rounded-lg bg-bg-primary border border-slate-600 focus:border-wraith-primary focus:outline-none text-white"
              disabled={loading}
            />
          </div>

          {error && (
            <div className="mb-4 p-3 bg-red-500/20 border border-red-500 rounded-lg text-red-300 text-sm">
              {error}
            </div>
          )}

          <div className="flex gap-3">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 p-3 rounded-lg bg-bg-tertiary hover:bg-slate-600 transition font-medium"
              disabled={loading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="flex-1 p-3 rounded-lg bg-wraith-primary hover:bg-wraith-secondary transition font-medium disabled:opacity-50"
              disabled={loading}
            >
              {loading ? "Creating..." : "Start Chat"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
