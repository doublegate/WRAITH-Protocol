// useGroup Hook - Convenience hook for group operations

import { useCallback, useMemo } from 'react';
import { useGroupStore } from '../stores/groupStore';
import { useUiStore } from '../stores/uiStore';
import type { MemberRole } from '../types';

export function useGroup(groupId?: string) {
  const {
    groups,
    groupInfos,
    members,
    selectedGroupId,
    loading,
    error,
    selectGroup,
    createGroup,
    deleteGroup,
    fetchGroupInfo,
    inviteMember,
    removeMember,
    setMemberRole,
    clearError,
  } = useGroupStore();

  const { addToast, openModal } = useUiStore();

  const effectiveGroupId = groupId || selectedGroupId;
  const group = useMemo(
    () => groups.find((g) => g.id === effectiveGroupId),
    [groups, effectiveGroupId]
  );
  const info = useMemo(
    () => (effectiveGroupId ? groupInfos.get(effectiveGroupId) : undefined),
    [groupInfos, effectiveGroupId]
  );

  const isSelected = effectiveGroupId === selectedGroupId;

  const handleCreate = useCallback(
    async (name: string, description?: string) => {
      try {
        const newGroup = await createGroup(name, description);
        addToast('success', `Group "${name}" created`);
        return newGroup;
      } catch (err) {
        addToast('error', (err as Error).message);
        throw err;
      }
    },
    [createGroup, addToast]
  );

  const handleDelete = useCallback(async () => {
    if (!effectiveGroupId || !group) return;
    if (!confirm(`Delete group "${group.name}"? This cannot be undone.`)) return;

    try {
      await deleteGroup(effectiveGroupId);
      addToast('success', 'Group deleted');
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  }, [effectiveGroupId, group, deleteGroup, addToast]);

  const handleInvite = useCallback(
    async (peerId: string | null, role: MemberRole) => {
      if (!effectiveGroupId) return;

      try {
        const invitation = await inviteMember(effectiveGroupId, peerId, role);
        addToast('success', 'Invitation created');
        return invitation;
      } catch (err) {
        addToast('error', (err as Error).message);
        throw err;
      }
    },
    [effectiveGroupId, inviteMember, addToast]
  );

  const handleRemoveMember = useCallback(
    async (peerId: string) => {
      if (!effectiveGroupId) return;

      try {
        await removeMember(effectiveGroupId, peerId);
        addToast('success', 'Member removed');
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [effectiveGroupId, removeMember, addToast]
  );

  const handleSetRole = useCallback(
    async (peerId: string, role: MemberRole) => {
      if (!effectiveGroupId) return;

      try {
        await setMemberRole(effectiveGroupId, peerId, role);
        addToast('success', 'Role updated');
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [effectiveGroupId, setMemberRole, addToast]
  );

  const openInviteModal = useCallback(() => {
    if (effectiveGroupId) {
      openModal('inviteMember', effectiveGroupId);
    }
  }, [effectiveGroupId, openModal]);

  return {
    // State
    group,
    info,
    members: isSelected ? members : [],
    isSelected,
    loading,
    error,

    // Actions
    select: () => selectGroup(effectiveGroupId || null),
    create: handleCreate,
    delete: handleDelete,
    invite: handleInvite,
    removeMember: handleRemoveMember,
    setRole: handleSetRole,
    refreshInfo: () => effectiveGroupId && fetchGroupInfo(effectiveGroupId),
    clearError,
    openInviteModal,
  };
}

export function useGroups() {
  const {
    groups,
    groupInfos,
    selectedGroupId,
    loading,
    fetchGroups,
    selectGroup,
  } = useGroupStore();
  const { openModal } = useUiStore();

  const groupsWithInfo = useMemo(
    () =>
      groups.map((group) => ({
        ...group,
        info: groupInfos.get(group.id),
        isSelected: group.id === selectedGroupId,
      })),
    [groups, groupInfos, selectedGroupId]
  );

  return {
    groups: groupsWithInfo,
    selectedGroupId,
    loading,
    refresh: fetchGroups,
    selectGroup,
    openCreateModal: () => openModal('createGroup'),
  };
}
