// WRAITH Chat - New Group Dialog Component

import { useState } from "react";
import { useGroupStore } from "../stores/groupStore";
import { useContactStore } from "../stores/contactStore";
import { useConversationStore } from "../stores/conversationStore";

interface NewGroupDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function NewGroupDialog({
  isOpen,
  onClose,
}: NewGroupDialogProps) {
  const [step, setStep] = useState<"name" | "members">("name");
  const [groupName, setGroupName] = useState("");
  const [selectedMembers, setSelectedMembers] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { createGroup, loading } = useGroupStore();
  const { contacts } = useContactStore();
  const { createConversation, selectConversation } = useConversationStore();

  const handleNext = () => {
    if (!groupName.trim()) {
      setError("Group name is required");
      return;
    }
    setError(null);
    setStep("members");
  };

  const handleBack = () => {
    setStep("name");
  };

  const toggleMember = (peerId: string) => {
    setSelectedMembers((prev) =>
      prev.includes(peerId)
        ? prev.filter((id) => id !== peerId)
        : [...prev, peerId],
    );
  };

  const handleCreate = async () => {
    if (!groupName.trim()) {
      setError("Group name is required");
      return;
    }

    try {
      const group = await createGroup(groupName.trim(), selectedMembers);

      // Create a conversation for this group
      const conversationId = await createConversation(
        "group",
        null,
        group.group_id,
        group.name,
      );

      selectConversation(conversationId);
      handleClose();
    } catch (err) {
      setError((err as Error).message || "Failed to create group");
    }
  };

  const handleClose = () => {
    setStep("name");
    setGroupName("");
    setSelectedMembers([]);
    setError(null);
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      handleClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={handleClose}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-group-title"
    >
      <div
        className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-md overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700">
          <div className="flex items-center gap-3">
            {step === "members" && (
              <button
                onClick={handleBack}
                className="p-1 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded transition-colors"
                aria-label="Go back"
              >
                <BackIcon className="w-5 h-5" />
              </button>
            )}
            <h2
              id="new-group-title"
              className="text-lg font-semibold text-white"
            >
              {step === "name" ? "Create New Group" : "Add Members"}
            </h2>
          </div>
          <button
            onClick={handleClose}
            className="p-1 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded transition-colors"
            aria-label="Close dialog"
          >
            <CloseIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Step Indicator */}
        <div className="flex items-center gap-2 px-6 py-3 border-b border-slate-700/50">
          <StepIndicator
            number={1}
            active={step === "name"}
            completed={step === "members"}
            label="Name"
          />
          <div className="flex-1 h-px bg-slate-700" />
          <StepIndicator
            number={2}
            active={step === "members"}
            completed={false}
            label="Members"
          />
        </div>

        {/* Content */}
        <div className="p-6">
          {step === "name" ? (
            <div className="space-y-4">
              {/* Group Icon */}
              <div className="flex justify-center">
                <div className="relative">
                  <div className="w-24 h-24 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
                    {groupName ? (
                      <span className="text-4xl font-bold text-white">
                        {groupName[0].toUpperCase()}
                      </span>
                    ) : (
                      <UsersIcon className="w-10 h-10 text-white" />
                    )}
                  </div>
                  <button
                    className="absolute bottom-0 right-0 w-8 h-8 bg-wraith-primary rounded-full flex items-center justify-center border-2 border-bg-secondary"
                    title="Add photo (coming soon)"
                  >
                    <CameraIcon className="w-4 h-4 text-white" />
                  </button>
                </div>
              </div>

              {/* Group Name Input */}
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Group Name
                </label>
                <input
                  type="text"
                  value={groupName}
                  onChange={(e) => setGroupName(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleNext()}
                  placeholder="Enter group name"
                  autoFocus
                  className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-3 text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary"
                />
              </div>

              {error && (
                <div className="p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-red-400 text-sm">
                  {error}
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              {/* Selected Count */}
              <div className="flex items-center justify-between">
                <span className="text-sm text-slate-400">
                  {selectedMembers.length} member
                  {selectedMembers.length !== 1 ? "s" : ""} selected
                </span>
                {selectedMembers.length > 0 && (
                  <button
                    onClick={() => setSelectedMembers([])}
                    className="text-sm text-wraith-primary hover:text-wraith-secondary"
                  >
                    Clear all
                  </button>
                )}
              </div>

              {/* Contact List */}
              <div className="max-h-64 overflow-y-auto space-y-2 -mx-2 px-2">
                {contacts.length === 0 ? (
                  <div className="text-center py-8">
                    <UsersIcon className="w-12 h-12 text-slate-600 mx-auto mb-3" />
                    <p className="text-slate-400">No contacts yet</p>
                    <p className="text-sm text-slate-500 mt-1">
                      Add contacts to invite them to groups
                    </p>
                  </div>
                ) : (
                  contacts.map((contact) => (
                    <label
                      key={contact.id}
                      className={`flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors ${
                        selectedMembers.includes(contact.peer_id)
                          ? "bg-wraith-primary/20 border border-wraith-primary/50"
                          : "bg-bg-primary hover:bg-bg-tertiary border border-transparent"
                      }`}
                    >
                      <input
                        type="checkbox"
                        checked={selectedMembers.includes(contact.peer_id)}
                        onChange={() => toggleMember(contact.peer_id)}
                        className="sr-only"
                      />
                      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-sm font-semibold text-white">
                        {(contact.display_name ||
                          contact.peer_id)[0].toUpperCase()}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-white truncate">
                          {contact.display_name ||
                            `${contact.peer_id.substring(0, 16)}...`}
                        </p>
                      </div>
                      <div
                        className={`w-5 h-5 rounded-full border-2 flex items-center justify-center transition-colors ${
                          selectedMembers.includes(contact.peer_id)
                            ? "bg-wraith-primary border-wraith-primary"
                            : "border-slate-600"
                        }`}
                      >
                        {selectedMembers.includes(contact.peer_id) && (
                          <CheckIcon className="w-3 h-3 text-white" />
                        )}
                      </div>
                    </label>
                  ))
                )}
              </div>

              {error && (
                <div className="p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-red-400 text-sm">
                  {error}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
          >
            Cancel
          </button>
          {step === "name" ? (
            <button
              onClick={handleNext}
              disabled={!groupName.trim()}
              className="px-6 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Next
            </button>
          ) : (
            <button
              onClick={handleCreate}
              disabled={loading}
              className="px-6 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? "Creating..." : "Create Group"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// Step Indicator Component
interface StepIndicatorProps {
  number: number;
  active: boolean;
  completed: boolean;
  label: string;
}

function StepIndicator({
  number,
  active,
  completed,
  label,
}: StepIndicatorProps) {
  return (
    <div className="flex items-center gap-2">
      <div
        className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium transition-colors ${
          completed
            ? "bg-green-500 text-white"
            : active
              ? "bg-wraith-primary text-white"
              : "bg-bg-tertiary text-slate-400"
        }`}
      >
        {completed ? <CheckIcon className="w-3.5 h-3.5" /> : number}
      </div>
      <span
        className={`text-sm ${
          active ? "text-white font-medium" : "text-slate-400"
        }`}
      >
        {label}
      </span>
    </div>
  );
}

// Icons
function BackIcon({ className }: { className?: string }) {
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
        d="M15 19l-7-7 7-7"
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

function CameraIcon({ className }: { className?: string }) {
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
        d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M15 13a3 3 0 11-6 0 3 3 0 016 0z"
      />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
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
        strokeWidth={2.5}
        d="M5 13l4 4L19 7"
      />
    </svg>
  );
}
