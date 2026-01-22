// GroupList Component - Displays all groups

import { useGroupStore } from '../../stores/groupStore';
import GroupCard from './GroupCard';

export default function GroupList() {
  const { groups, groupInfos, selectedGroupId, selectGroup, loading } = useGroupStore();

  if (loading && groups.length === 0) {
    return (
      <div className="p-8 text-center">
        <div className="animate-spin w-8 h-8 border-2 border-violet-500 border-t-transparent rounded-full mx-auto mb-4" />
        <p className="text-slate-400">Loading groups...</p>
      </div>
    );
  }

  if (groups.length === 0) {
    return (
      <div className="p-8 text-center">
        <svg
          className="w-16 h-16 mx-auto text-slate-600 mb-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
          />
        </svg>
        <h3 className="text-lg font-medium text-white mb-2">No groups yet</h3>
        <p className="text-slate-400 max-w-md mx-auto">
          Create a group to start sharing files securely with others using end-to-end encryption.
        </p>
      </div>
    );
  }

  return (
    <div className="p-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {groups.map((group) => (
        <GroupCard
          key={group.id}
          group={group}
          info={groupInfos.get(group.id)}
          isSelected={selectedGroupId === group.id}
          onSelect={() => selectGroup(group.id)}
        />
      ))}
    </div>
  );
}
