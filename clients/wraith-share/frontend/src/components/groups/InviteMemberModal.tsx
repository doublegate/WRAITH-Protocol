// InviteMemberModal Component - Invite members to a group

import { useState } from 'react';
import Modal from '../ui/Modal';
import Input, { Select } from '../ui/Input';
import Button from '../ui/Button';
import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';
import type { MemberRole, ExportedInvitation } from '../../types';

export default function InviteMemberModal() {
  const { activeModal, modalData, closeModal, addToast } = useUiStore();
  const { inviteMember, groups } = useGroupStore();

  const [peerId, setPeerId] = useState('');
  const [role, setRole] = useState<MemberRole>('read');
  const [loading, setLoading] = useState(false);
  const [invitation, setInvitation] = useState<ExportedInvitation | null>(null);
  const [copied, setCopied] = useState(false);

  const isOpen = activeModal === 'inviteMember';
  const groupId = modalData as string;
  const group = groups.find((g) => g.id === groupId);

  const handleClose = () => {
    setPeerId('');
    setRole('read');
    setInvitation(null);
    setCopied(false);
    closeModal();
  };

  const handleCreateInvitation = async () => {
    if (!groupId) return;

    setLoading(true);
    try {
      const inv = await inviteMember(
        groupId,
        peerId.trim() || null,
        role
      );
      setInvitation(inv);
      addToast('success', 'Invitation created successfully');
    } catch (err) {
      addToast('error', (err as Error).message);
    } finally {
      setLoading(false);
    }
  };

  const handleCopyInvitation = async () => {
    if (!invitation) return;

    const inviteText = JSON.stringify(invitation);
    await navigator.clipboard.writeText(inviteText);
    setCopied(true);
    addToast('success', 'Invitation copied to clipboard');

    setTimeout(() => setCopied(false), 2000);
  };

  const roleOptions = [
    { value: 'read', label: 'Read - Can view and download files' },
    { value: 'write', label: 'Write - Can upload and modify files' },
    { value: 'admin', label: 'Admin - Full access including member management' },
  ];

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      title={`Invite to ${group?.name || 'Group'}`}
      size="lg"
    >
      {!invitation ? (
        <div className="space-y-4">
          <Input
            label="Peer ID (optional)"
            value={peerId}
            onChange={(e) => setPeerId(e.target.value)}
            placeholder="Leave empty to create a reusable link"
            hint="If provided, the invitation will only work for this specific peer"
          />

          <Select
            label="Role"
            value={role}
            onChange={(e) => setRole(e.target.value as MemberRole)}
            options={roleOptions}
          />

          <div className="flex justify-end gap-3 pt-4">
            <Button variant="ghost" onClick={handleClose}>
              Cancel
            </Button>
            <Button onClick={handleCreateInvitation} loading={loading}>
              Create Invitation
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="p-4 bg-slate-700/50 rounded-lg">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium text-slate-300">
                Invitation Created
              </span>
              <span className="text-xs text-slate-400">
                Role: {invitation.role}
              </span>
            </div>
            <div className="p-3 bg-slate-800 rounded font-mono text-xs text-slate-300 break-all max-h-32 overflow-auto">
              {JSON.stringify(invitation, null, 2)}
            </div>
          </div>

          <Button
            onClick={handleCopyInvitation}
            className="w-full"
            variant={copied ? 'secondary' : 'primary'}
          >
            {copied ? (
              <span className="flex items-center justify-center gap-2">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                    clipRule="evenodd"
                  />
                </svg>
                Copied!
              </span>
            ) : (
              <span className="flex items-center justify-center gap-2">
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
                  />
                </svg>
                Copy Invitation
              </span>
            )}
          </Button>

          <p className="text-xs text-slate-400 text-center">
            Share this invitation with the person you want to invite. They can paste it
            in the "Accept Invitation" dialog to join the group.
          </p>

          <Button variant="ghost" onClick={handleClose} className="w-full">
            Done
          </Button>
        </div>
      )}
    </Modal>
  );
}
