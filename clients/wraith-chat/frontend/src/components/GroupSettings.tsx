// Group Settings Component - Sprint 17.7

import { useEffect, useState } from 'react';
import {
  useGroupStore,
  formatMemberCount,
  getRoleLabel,
  formatJoinedDate,
  type GroupInfo,
  type GroupMember,
} from '../stores/groupStore';

interface GroupSettingsProps {
  groupId: string;
  onClose: () => void;
}

export default function GroupSettings({ groupId, onClose }: GroupSettingsProps) {
  const {
    loading,
    error,
    currentGroupMembers,
    loadGroupInfo,
    loadGroupMembers,
    updateGroupSettings,
    leaveGroup,
    addMember,
    removeMember,
    promoteToAdmin,
    demoteFromAdmin,
    rotateGroupKeys,
    selectGroup,
    clearError,
  } = useGroupStore();

  const [groupInfo, setGroupInfo] = useState<GroupInfo | null>(null);
  const [activeTab, setActiveTab] = useState<'general' | 'members' | 'security'>('general');

  // Edit states
  const [editName, setEditName] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [newMemberPeerId, setNewMemberPeerId] = useState('');
  const [newMemberName, setNewMemberName] = useState('');

  // Confirmation states
  const [confirmLeave, setConfirmLeave] = useState(false);
  const [memberToRemove, setMemberToRemove] = useState<string | null>(null);

  // Load group data
  useEffect(() => {
    selectGroup(groupId);
    loadGroupInfo(groupId).then((info) => {
      if (info) {
        setGroupInfo(info);
        setEditName(info.name);
        setEditDescription(info.description || '');
      }
    });
    loadGroupMembers(groupId);

    return () => {
      selectGroup(null);
    };
  }, [groupId, selectGroup, loadGroupInfo, loadGroupMembers]);

  const handleSaveSettings = async () => {
    if (!groupInfo) return;

    try {
      const updated = await updateGroupSettings(
        groupId,
        editName !== groupInfo.name ? editName : undefined,
        editDescription !== (groupInfo.description || '') ? editDescription : undefined
      );
      setGroupInfo(updated);
    } catch (err) {
      console.error('Failed to update settings:', err);
    }
  };

  const handleAddMember = async () => {
    if (!newMemberPeerId.trim()) return;

    try {
      await addMember(groupId, newMemberPeerId.trim(), newMemberName.trim() || undefined);
      setNewMemberPeerId('');
      setNewMemberName('');
      // Refresh info
      const info = await loadGroupInfo(groupId);
      if (info) setGroupInfo(info);
    } catch (err) {
      console.error('Failed to add member:', err);
    }
  };

  const handleRemoveMember = async (peerId: string) => {
    try {
      await removeMember(groupId, peerId);
      setMemberToRemove(null);
      const info = await loadGroupInfo(groupId);
      if (info) setGroupInfo(info);
    } catch (err) {
      console.error('Failed to remove member:', err);
    }
  };

  const handleToggleAdmin = async (member: GroupMember) => {
    try {
      if (member.role === 'admin') {
        await demoteFromAdmin(groupId, member.peer_id);
      } else {
        await promoteToAdmin(groupId, member.peer_id);
      }
    } catch (err) {
      console.error('Failed to change member role:', err);
    }
  };

  const handleLeaveGroup = async () => {
    try {
      await leaveGroup(groupId);
      onClose();
    } catch (err) {
      console.error('Failed to leave group:', err);
    }
  };

  const handleRotateKeys = async () => {
    try {
      await rotateGroupKeys(groupId);
    } catch (err) {
      console.error('Failed to rotate keys:', err);
    }
  };

  if (!groupInfo) {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
        <div className="bg-wraith-dark p-8 rounded-lg">
          <p>Loading group...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-wraith-dark w-full max-w-2xl max-h-[90vh] rounded-lg overflow-hidden flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h2 className="text-xl font-semibold">Group Settings</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-700 rounded transition"
          >
            <CloseIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Error Display */}
        {error && (
          <div className="mx-4 mt-4 p-3 bg-red-500/20 text-red-400 rounded flex items-center justify-between">
            <span>{error}</span>
            <button onClick={clearError} className="text-red-300 hover:text-white">
              <CloseIcon className="w-4 h-4" />
            </button>
          </div>
        )}

        {/* Tabs */}
        <div className="flex border-b border-gray-700">
          {(['general', 'members', 'security'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-6 py-3 font-medium transition ${
                activeTab === tab
                  ? 'text-wraith-primary border-b-2 border-wraith-primary'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* General Tab */}
          {activeTab === 'general' && (
            <div className="space-y-6">
              {/* Group Avatar */}
              <div className="flex items-center gap-4">
                <div className="w-20 h-20 rounded-full bg-wraith-primary flex items-center justify-center text-3xl font-bold">
                  {groupInfo.name.charAt(0).toUpperCase()}
                </div>
                {groupInfo.am_i_admin && (
                  <button className="px-4 py-2 border border-gray-600 rounded hover:bg-gray-700 transition">
                    Change Avatar
                  </button>
                )}
              </div>

              {/* Group Name */}
              <div>
                <label className="block text-sm text-gray-400 mb-1">Group Name</label>
                <input
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  disabled={!groupInfo.am_i_admin}
                  className="w-full p-3 bg-wraith-darker border border-gray-600 rounded focus:border-wraith-primary focus:outline-none disabled:opacity-50"
                />
              </div>

              {/* Description */}
              <div>
                <label className="block text-sm text-gray-400 mb-1">Description</label>
                <textarea
                  value={editDescription}
                  onChange={(e) => setEditDescription(e.target.value)}
                  disabled={!groupInfo.am_i_admin}
                  rows={3}
                  className="w-full p-3 bg-wraith-darker border border-gray-600 rounded focus:border-wraith-primary focus:outline-none resize-none disabled:opacity-50"
                />
              </div>

              {/* Save Button */}
              {groupInfo.am_i_admin && (
                <button
                  onClick={handleSaveSettings}
                  disabled={loading}
                  className="px-6 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded font-semibold transition disabled:opacity-50"
                >
                  {loading ? 'Saving...' : 'Save Changes'}
                </button>
              )}

              {/* Group Stats */}
              <div className="pt-4 border-t border-gray-700">
                <p className="text-sm text-gray-400">
                  {formatMemberCount(groupInfo.member_count)} | Created{' '}
                  {formatJoinedDate(groupInfo.created_at)}
                </p>
              </div>
            </div>
          )}

          {/* Members Tab */}
          {activeTab === 'members' && (
            <div className="space-y-6">
              {/* Add Member */}
              {groupInfo.am_i_admin && (
                <div className="p-4 bg-wraith-darker rounded">
                  <h3 className="font-semibold mb-3">Add Member</h3>
                  <div className="space-y-3">
                    <input
                      type="text"
                      value={newMemberPeerId}
                      onChange={(e) => setNewMemberPeerId(e.target.value)}
                      placeholder="Peer ID"
                      className="w-full p-2 bg-wraith-dark border border-gray-600 rounded focus:border-wraith-primary focus:outline-none font-mono text-sm"
                    />
                    <input
                      type="text"
                      value={newMemberName}
                      onChange={(e) => setNewMemberName(e.target.value)}
                      placeholder="Display Name (optional)"
                      className="w-full p-2 bg-wraith-dark border border-gray-600 rounded focus:border-wraith-primary focus:outline-none"
                    />
                    <button
                      onClick={handleAddMember}
                      disabled={!newMemberPeerId.trim() || loading}
                      className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded font-semibold transition disabled:opacity-50"
                    >
                      Add Member
                    </button>
                  </div>
                </div>
              )}

              {/* Member List */}
              <div>
                <h3 className="font-semibold mb-3">
                  Members ({currentGroupMembers.length})
                </h3>
                <div className="space-y-2">
                  {currentGroupMembers.map((member) => (
                    <div
                      key={member.peer_id}
                      className="p-3 bg-wraith-darker rounded flex items-center justify-between"
                    >
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 rounded-full bg-gray-700 flex items-center justify-center">
                          {(member.display_name || member.peer_id).charAt(0).toUpperCase()}
                        </div>
                        <div>
                          <p className="font-medium">
                            {member.display_name || member.peer_id.substring(0, 16)}
                          </p>
                          <p className="text-sm text-gray-400">
                            {getRoleLabel(member.role)} | Joined{' '}
                            {formatJoinedDate(member.joined_at)}
                          </p>
                        </div>
                      </div>

                      {/* Member Actions */}
                      {groupInfo.am_i_admin && (
                        <div className="flex items-center gap-2">
                          {/* Toggle Admin */}
                          <button
                            onClick={() => handleToggleAdmin(member)}
                            className="px-3 py-1 text-sm border border-gray-600 rounded hover:bg-gray-700 transition"
                          >
                            {member.role === 'admin' ? 'Demote' : 'Promote'}
                          </button>

                          {/* Remove */}
                          {memberToRemove === member.peer_id ? (
                            <div className="flex items-center gap-1">
                              <button
                                onClick={() => handleRemoveMember(member.peer_id)}
                                className="px-3 py-1 text-sm bg-red-500 rounded hover:bg-red-600 transition"
                              >
                                Confirm
                              </button>
                              <button
                                onClick={() => setMemberToRemove(null)}
                                className="px-3 py-1 text-sm border border-gray-600 rounded hover:bg-gray-700 transition"
                              >
                                Cancel
                              </button>
                            </div>
                          ) : (
                            <button
                              onClick={() => setMemberToRemove(member.peer_id)}
                              className="px-3 py-1 text-sm text-red-400 border border-red-500/50 rounded hover:bg-red-500/20 transition"
                            >
                              Remove
                            </button>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Security Tab */}
          {activeTab === 'security' && (
            <div className="space-y-6">
              {/* Encryption Info */}
              <div className="p-4 bg-wraith-darker rounded">
                <h3 className="font-semibold mb-2">End-to-End Encryption</h3>
                <p className="text-sm text-gray-400">
                  All messages in this group are encrypted using the Sender Keys protocol.
                  Each member has a unique encryption key that is automatically rotated for security.
                </p>
              </div>

              {/* Key Rotation */}
              {groupInfo.am_i_admin && (
                <div className="p-4 bg-wraith-darker rounded">
                  <h3 className="font-semibold mb-2">Key Rotation</h3>
                  <p className="text-sm text-gray-400 mb-4">
                    Keys are automatically rotated every 7 days. You can manually rotate keys
                    if a member was removed or you suspect key compromise.
                  </p>
                  <button
                    onClick={handleRotateKeys}
                    disabled={loading}
                    className="px-4 py-2 border border-gray-600 rounded hover:bg-gray-700 transition disabled:opacity-50"
                  >
                    {loading ? 'Rotating...' : 'Rotate Keys Now'}
                  </button>
                </div>
              )}

              {/* Group ID */}
              <div className="p-4 bg-wraith-darker rounded">
                <h3 className="font-semibold mb-2">Group ID</h3>
                <p className="text-sm font-mono text-gray-400 break-all">
                  {groupInfo.group_id}
                </p>
              </div>

              {/* Leave Group */}
              <div className="p-4 border border-red-500/50 rounded">
                <h3 className="font-semibold text-red-400 mb-2">Leave Group</h3>
                <p className="text-sm text-gray-400 mb-4">
                  Once you leave this group, you will no longer receive messages and your
                  message history will be archived.
                </p>
                {confirmLeave ? (
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleLeaveGroup}
                      disabled={loading}
                      className="px-4 py-2 bg-red-500 hover:bg-red-600 rounded font-semibold transition disabled:opacity-50"
                    >
                      {loading ? 'Leaving...' : 'Confirm Leave'}
                    </button>
                    <button
                      onClick={() => setConfirmLeave(false)}
                      className="px-4 py-2 border border-gray-600 rounded hover:bg-gray-700 transition"
                    >
                      Cancel
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => setConfirmLeave(true)}
                    className="px-4 py-2 text-red-400 border border-red-500/50 rounded hover:bg-red-500/20 transition"
                  >
                    Leave Group
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// Create Group Dialog
interface CreateGroupDialogProps {
  onClose: () => void;
  onCreated?: (groupInfo: GroupInfo) => void;
}

export function CreateGroupDialog({ onClose, onCreated }: CreateGroupDialogProps) {
  const { createGroup, loading, error, clearError } = useGroupStore();
  const [name, setName] = useState('');

  const handleCreate = async () => {
    if (!name.trim()) return;

    try {
      const group = await createGroup(name.trim());
      onCreated?.(group);
      onClose();
    } catch (err) {
      console.error('Failed to create group:', err);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-wraith-dark w-full max-w-md p-6 rounded-lg">
        <h2 className="text-xl font-semibold mb-4">Create New Group</h2>

        {error && (
          <div className="mb-4 p-3 bg-red-500/20 text-red-400 rounded flex items-center justify-between">
            <span>{error}</span>
            <button onClick={clearError} className="text-red-300 hover:text-white">
              <CloseIcon className="w-4 h-4" />
            </button>
          </div>
        )}

        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Group Name"
          autoFocus
          className="w-full p-3 bg-wraith-darker border border-gray-600 rounded focus:border-wraith-primary focus:outline-none mb-4"
        />

        <div className="flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 border border-gray-600 rounded hover:bg-gray-700 transition"
          >
            Cancel
          </button>
          <button
            onClick={handleCreate}
            disabled={!name.trim() || loading}
            className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary rounded font-semibold transition disabled:opacity-50"
          >
            {loading ? 'Creating...' : 'Create Group'}
          </button>
        </div>
      </div>
    </div>
  );
}

// Close Icon
function CloseIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}
