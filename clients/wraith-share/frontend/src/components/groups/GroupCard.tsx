// GroupCard Component - Individual group card

import type { Group, GroupInfo } from '../../types';
import { formatRelativeTime } from '../../types';

interface GroupCardProps {
  group: Group;
  info?: GroupInfo;
  isSelected: boolean;
  onSelect: () => void;
}

export default function GroupCard({
  group,
  info,
  isSelected,
  onSelect,
}: GroupCardProps) {
  return (
    <button
      onClick={onSelect}
      className={`w-full text-left p-4 rounded-xl border transition-all ${
        isSelected
          ? 'bg-violet-600/20 border-violet-500 ring-2 ring-violet-500/50'
          : 'bg-slate-800 border-slate-700 hover:border-slate-600 hover:bg-slate-750'
      }`}
      aria-pressed={isSelected}
    >
      {/* Header */}
      <div className="flex items-start gap-3">
        <div
          className={`w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0 ${
            isSelected ? 'bg-violet-600' : 'bg-slate-700'
          }`}
        >
          <svg className="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 20 20">
            <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z" />
          </svg>
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-white truncate">{group.name}</h3>
          {group.description && (
            <p className="text-sm text-slate-400 truncate mt-0.5">
              {group.description}
            </p>
          )}
        </div>
        {info && <RoleBadge role={info.my_role} />}
      </div>

      {/* Stats */}
      {info && (
        <div className="flex items-center gap-4 mt-4 text-sm text-slate-400">
          <div className="flex items-center gap-1.5">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M9 6a3 3 0 11-6 0 3 3 0 016 0zM17 6a3 3 0 11-6 0 3 3 0 016 0zM12.93 17c.046-.327.07-.66.07-1a6.97 6.97 0 00-1.5-4.33A5 5 0 0119 16v1h-6.07zM6 11a5 5 0 015 5v1H1v-1a5 5 0 015-5z" />
            </svg>
            <span>{info.member_count}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path
                fillRule="evenodd"
                d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"
                clipRule="evenodd"
              />
            </svg>
            <span>{info.file_count}</span>
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="mt-3 pt-3 border-t border-slate-700 flex items-center justify-between">
        <span className="text-xs text-slate-500">
          Created {formatRelativeTime(group.created_at)}
        </span>
        {info?.is_admin && (
          <span className="text-xs text-amber-400 flex items-center gap-1">
            <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
              <path
                fillRule="evenodd"
                d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                clipRule="evenodd"
              />
            </svg>
            Admin
          </span>
        )}
      </div>
    </button>
  );
}

function RoleBadge({ role }: { role: string }) {
  const colors = {
    admin: 'bg-amber-500/20 text-amber-400 border-amber-500/30',
    write: 'bg-green-500/20 text-green-400 border-green-500/30',
    read: 'bg-slate-500/20 text-slate-400 border-slate-500/30',
  };

  return (
    <span
      className={`px-2 py-0.5 text-xs rounded border ${colors[role as keyof typeof colors] || colors.read}`}
    >
      {role}
    </span>
  );
}
