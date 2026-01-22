// CreateGroupModal Component - Create a new group

import { useState } from 'react';
import Modal, { ModalFooter } from '../ui/Modal';
import Input, { Textarea } from '../ui/Input';
import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';

export default function CreateGroupModal() {
  const { activeModal, closeModal, addToast } = useUiStore();
  const { createGroup } = useGroupStore();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isOpen = activeModal === 'createGroup';

  const handleClose = () => {
    setName('');
    setDescription('');
    setError(null);
    closeModal();
  };

  const handleSubmit = async () => {
    if (!name.trim()) {
      setError('Group name is required');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await createGroup(name.trim(), description.trim() || undefined);
      addToast('success', `Group "${name}" created successfully`);
      handleClose();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      title="Create New Group"
      footer={
        <ModalFooter
          onCancel={handleClose}
          onConfirm={handleSubmit}
          confirmText="Create Group"
          loading={loading}
          disabled={!name.trim()}
        />
      }
    >
      <div className="space-y-4">
        <Input
          label="Group Name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter group name"
          error={error || undefined}
          autoFocus
          maxLength={64}
        />

        <Textarea
          label="Description (optional)"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="What is this group for?"
          maxLength={256}
        />

        <div className="p-3 bg-slate-700/50 rounded-lg">
          <div className="flex items-start gap-2">
            <svg
              className="w-5 h-5 text-cyan-400 flex-shrink-0 mt-0.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="text-sm text-slate-300">
              You will be the admin of this group. You can invite members and manage
              permissions after creation.
            </p>
          </div>
        </div>
      </div>
    </Modal>
  );
}
