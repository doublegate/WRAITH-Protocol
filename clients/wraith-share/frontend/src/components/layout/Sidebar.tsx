// Sidebar Component - Group navigation and management

import { useGroupStore } from '../../stores/groupStore';
import { useUiStore } from '../../stores/uiStore';
import Button from '../ui/Button';

export default function Sidebar() {
  const { groups, groupInfos, selectedGroupId, selectGroup, loading } = useGroupStore();
  const { sidebarCollapsed, toggleSidebar, openModal } = useUiStore();

  const handleCreateGroup = () => {
    openModal('createGroup');
  };

  return (
    <aside
      className={`bg-slate-800 border-r border-slate-700 flex flex-col transition-all duration-300 ${
        sidebarCollapsed ? 'w-16' : 'w-64'
      }`}
    >
      {/* Header */}
      <div className="h-14 flex items-center justify-between px-4 border-b border-slate-700">
        {!sidebarCollapsed && (
          <h1 className="text-lg font-bold text-white">WRAITH Share</h1>
        )}
        <button
          onClick={toggleSidebar}
          className="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded transition-colors"
          aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <svg
            className={`w-5 h-5 transition-transform ${sidebarCollapsed ? 'rotate-180' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M11 19l-7-7 7-7m8 14l-7-7 7-7"
            />
          </svg>
        </button>
      </div>

      {/* Create group button */}
      <div className="p-3">
        {sidebarCollapsed ? (
          <button
            onClick={handleCreateGroup}
            className="w-10 h-10 rounded-lg bg-violet-600 hover:bg-violet-700 text-white flex items-center justify-center transition-colors"
            aria-label="Create group"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
          </button>
        ) : (
          <Button onClick={handleCreateGroup} className="w-full">
            <span className="flex items-center justify-center gap-2">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
              </svg>
              Create Group
            </span>
          </Button>
        )}
      </div>

      {/* Groups list */}
      <div className="flex-1 overflow-y-auto">
        <nav className="p-2 space-y-1" aria-label="Groups navigation">
          {loading && groups.length === 0 ? (
            <div className="p-4 text-center text-slate-400 text-sm">
              Loading groups...
            </div>
          ) : groups.length === 0 ? (
            <div className="p-4 text-center text-slate-400 text-sm">
              {sidebarCollapsed ? '' : 'No groups yet. Create one to get started!'}
            </div>
          ) : (
            groups.map((group) => {
              const info = groupInfos.get(group.id);
              const isSelected = selectedGroupId === group.id;

              return (
                <button
                  key={group.id}
                  onClick={() => selectGroup(group.id)}
                  className={`w-full flex items-center gap-3 p-2 rounded-lg transition-colors ${
                    isSelected
                      ? 'bg-violet-600 text-white'
                      : 'text-slate-300 hover:bg-slate-700 hover:text-white'
                  }`}
                  aria-current={isSelected ? 'true' : undefined}
                >
                  {/* Group icon */}
                  <div
                    className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${
                      isSelected ? 'bg-violet-500' : 'bg-slate-600'
                    }`}
                  >
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                      <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z" />
                    </svg>
                  </div>

                  {/* Group info */}
                  {!sidebarCollapsed && (
                    <div className="flex-1 min-w-0 text-left">
                      <p className="font-medium truncate">{group.name}</p>
                      {info && (
                        <p className="text-xs text-slate-400 truncate">
                          {info.member_count} members, {info.file_count} files
                        </p>
                      )}
                    </div>
                  )}

                  {/* Role badge */}
                  {!sidebarCollapsed && info && (
                    <RoleBadge role={info.my_role} />
                  )}
                </button>
              );
            })
          )}
        </nav>
      </div>

      {/* Footer */}
      {!sidebarCollapsed && (
        <div className="p-4 border-t border-slate-700">
          <p className="text-xs text-slate-500 text-center">
            WRAITH Protocol v1.7.2
          </p>
        </div>
      )}
    </aside>
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
      className={`px-2 py-0.5 text-xs rounded ${colors[role as keyof typeof colors] || colors.read}`}
    >
      {role}
    </span>
  );
}
