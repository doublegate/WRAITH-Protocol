// MemberList Component - Display and manage group members

import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';
import Button from '../ui/Button';
// Select component imported but used via native select for simplicity
import { truncatePeerId, formatRelativeTime } from '../../types';
import type { GroupMember, MemberRole } from '../../types';

export default function MemberList() {
  const { members, selectedGroupId, groupInfos, setMemberRole, removeMember } = useGroupStore();
  const { peerId, openModal, addToast } = useUiStore();

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;
  const isAdmin = groupInfo?.is_admin;

  const handleRoleChange = async (member: GroupMember, newRole: MemberRole) => {
    if (!selectedGroupId) return;
    try {
      await setMemberRole(selectedGroupId, member.peer_id, newRole);
      addToast('success', 'Role updated successfully');
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  const handleRemoveMember = async (member: GroupMember) => {
    if (!selectedGroupId) return;
    if (!confirm(`Remove ${member.display_name || truncatePeerId(member.peer_id)} from this group?`)) {
      return;
    }
    try {
      await removeMember(selectedGroupId, member.peer_id);
      addToast('success', 'Member removed');
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  if (members.length === 0) {
    return (
      <div className="p-4 text-center text-slate-400">
        <p>No members yet</p>
      </div>
    );
  }

  const roleOptions = [
    { value: 'read', label: 'Read' },
    { value: 'write', label: 'Write' },
    { value: 'admin', label: 'Admin' },
  ];

  return (
    <div className="space-y-4">
      {/* Invite button */}
      {isAdmin && selectedGroupId && (
        <div className="flex justify-end">
          <Button
            size="sm"
            onClick={() => openModal('inviteMember', selectedGroupId)}
          >
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z"
                />
              </svg>
              Invite Member
            </span>
          </Button>
        </div>
      )}

      {/* Member list */}
      <div className="space-y-2">
        {members.map((member) => {
          const isSelf = member.peer_id === peerId;
          const canManage = isAdmin && !isSelf;

          return (
            <div
              key={member.peer_id}
              className="flex items-center gap-3 p-3 bg-slate-800 rounded-lg"
            >
              {/* Avatar */}
              <div className="w-10 h-10 rounded-full bg-violet-600 flex items-center justify-center text-white font-medium">
                {(member.display_name || member.peer_id).charAt(0).toUpperCase()}
              </div>

              {/* Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-white truncate">
                    {member.display_name || truncatePeerId(member.peer_id)}
                  </span>
                  {isSelf && (
                    <span className="text-xs px-1.5 py-0.5 bg-cyan-500/20 text-cyan-400 rounded">
                      You
                    </span>
                  )}
                </div>
                <p className="text-xs text-slate-500">
                  Joined {formatRelativeTime(member.joined_at)}
                </p>
              </div>

              {/* Role selector or badge */}
              {canManage ? (
                <div className="flex items-center gap-2">
                  <select
                    value={member.role}
                    onChange={(e) =>
                      handleRoleChange(member, e.target.value as MemberRole)
                    }
                    className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-white"
                  >
                    {roleOptions.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => handleRemoveMember(member)}
                    className="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-400/10 rounded transition-colors"
                    aria-label="Remove member"
                  >
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fillRule="evenodd"
                        d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>
                </div>
              ) : (
                <RoleBadge role={member.role} />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function RoleBadge({ role }: { role: string }) {
  const colors = {
    admin: 'bg-amber-500/20 text-amber-400',
    write: 'bg-green-500/20 text-green-400',
    read: 'bg-slate-500/20 text-slate-400',
  };

  return (
    <span
      className={`px-2 py-1 text-xs rounded ${colors[role as keyof typeof colors] || colors.read}`}
    >
      {role}
    </span>
  );
}
