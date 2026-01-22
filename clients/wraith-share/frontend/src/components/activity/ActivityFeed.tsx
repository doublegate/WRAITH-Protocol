// ActivityFeed Component - Display activity timeline

import { useEffect, useRef, useCallback } from 'react';
import { useActivityStore } from '../../stores/activityStore';
import { useGroupStore } from '../../stores/groupStore';
import { formatRelativeTime, truncatePeerId } from '../../types';
import type { ActivityInfo, ActivityType } from '../../types';

export default function ActivityFeed() {
  const { selectedGroupId, groups } = useGroupStore();
  const {
    activities,
    stats,
    loading,
    hasMore,
    fetchActivities,
    fetchMoreActivities,
    fetchRecentActivities,
    fetchStats,
    clearActivities,
  } = useActivityStore();

  const observerRef = useRef<IntersectionObserver | null>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (selectedGroupId) {
      clearActivities();
      fetchActivities(selectedGroupId);
      fetchStats(selectedGroupId);
    } else {
      fetchRecentActivities();
    }
  }, [selectedGroupId, fetchActivities, fetchRecentActivities, fetchStats, clearActivities]);

  // Infinite scroll
  const handleLoadMore = useCallback(() => {
    if (selectedGroupId && !loading && hasMore) {
      fetchMoreActivities(selectedGroupId);
    }
  }, [selectedGroupId, loading, hasMore, fetchMoreActivities]);

  useEffect(() => {
    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          handleLoadMore();
        }
      },
      { threshold: 0.1 }
    );

    if (loadMoreRef.current) {
      observerRef.current.observe(loadMoreRef.current);
    }

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [handleLoadMore]);

  const selectedGroup = groups.find((g) => g.id === selectedGroupId);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b border-slate-700">
        <h3 className="font-semibold text-white">Activity</h3>
        {selectedGroup ? (
          <p className="text-xs text-slate-400">{selectedGroup.name}</p>
        ) : (
          <p className="text-xs text-slate-400">Recent activity across all groups</p>
        )}
      </div>

      {/* Stats */}
      {stats && selectedGroupId && (
        <div className="p-3 border-b border-slate-700 grid grid-cols-2 gap-2 text-xs">
          <div className="p-2 bg-slate-800 rounded text-center">
            <span className="block text-lg font-semibold text-white">
              {stats.uploads}
            </span>
            <span className="text-slate-400">Uploads</span>
          </div>
          <div className="p-2 bg-slate-800 rounded text-center">
            <span className="block text-lg font-semibold text-white">
              {stats.downloads}
            </span>
            <span className="text-slate-400">Downloads</span>
          </div>
        </div>
      )}

      {/* Activity list */}
      <div className="flex-1 overflow-y-auto">
        {loading && activities.length === 0 ? (
          <div className="p-4 text-center">
            <div className="animate-spin w-6 h-6 border-2 border-violet-500 border-t-transparent rounded-full mx-auto" />
          </div>
        ) : activities.length === 0 ? (
          <div className="p-4 text-center text-slate-500 text-sm">
            No activity yet
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {activities.map((activity) => (
              <ActivityItem key={activity.id} activity={activity} />
            ))}
            {hasMore && (
              <div ref={loadMoreRef} className="p-2 text-center">
                {loading && (
                  <div className="animate-spin w-4 h-4 border-2 border-violet-500 border-t-transparent rounded-full mx-auto" />
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function ActivityItem({ activity }: { activity: ActivityInfo }) {
  const { icon, color, description } = getActivityDisplay(activity);

  return (
    <div className="flex gap-3 p-2 rounded-lg hover:bg-slate-800/50 transition-colors">
      {/* Icon */}
      <div
        className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${color}`}
      >
        {icon}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <p className="text-sm text-slate-200">
          <span className="font-medium text-white">
            {activity.actor_name || truncatePeerId(activity.actor_id)}
          </span>{' '}
          {description}
        </p>
        {activity.target_name && (
          <p className="text-xs text-slate-400 truncate">{activity.target_name}</p>
        )}
        <p className="text-xs text-slate-500 mt-0.5">
          {formatRelativeTime(activity.created_at)}
        </p>
      </div>
    </div>
  );
}

function getActivityDisplay(activity: ActivityInfo): {
  icon: JSX.Element;
  color: string;
  description: string;
} {
  const iconClass = 'w-4 h-4';

  const displays: Record<
    ActivityType,
    { icon: JSX.Element; color: string; description: string }
  > = {
    file_uploaded: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM6.293 6.707a1 1 0 010-1.414l3-3a1 1 0 011.414 0l3 3a1 1 0 01-1.414 1.414L11 5.414V13a1 1 0 11-2 0V5.414L7.707 6.707a1 1 0 01-1.414 0z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-green-500/20 text-green-400',
      description: 'uploaded a file',
    },
    file_downloaded: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-blue-500/20 text-blue-400',
      description: 'downloaded a file',
    },
    file_deleted: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-red-500/20 text-red-400',
      description: 'deleted a file',
    },
    file_version_restored: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-cyan-500/20 text-cyan-400',
      description: 'restored a file version',
    },
    member_joined: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M8 9a3 3 0 100-6 3 3 0 000 6zM8 11a6 6 0 016 6H2a6 6 0 016-6zM16 7a1 1 0 10-2 0v1h-1a1 1 0 100 2h1v1a1 1 0 102 0v-1h1a1 1 0 100-2h-1V7z" />
        </svg>
      ),
      color: 'bg-emerald-500/20 text-emerald-400',
      description: 'joined the group',
    },
    member_left: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M11 6a3 3 0 11-6 0 3 3 0 016 0zM14 17a6 6 0 00-12 0h12zM13 8a1 1 0 100 2h4a1 1 0 100-2h-4z" />
        </svg>
      ),
      color: 'bg-slate-500/20 text-slate-400',
      description: 'left the group',
    },
    member_invited: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M8 9a3 3 0 100-6 3 3 0 000 6zM8 11a6 6 0 016 6H2a6 6 0 016-6zM16 7a1 1 0 10-2 0v1h-1a1 1 0 100 2h1v1a1 1 0 102 0v-1h1a1 1 0 100-2h-1V7z" />
        </svg>
      ),
      color: 'bg-violet-500/20 text-violet-400',
      description: 'invited a member',
    },
    member_removed: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M11 6a3 3 0 11-6 0 3 3 0 016 0zM14 17a6 6 0 00-12 0h12zM13 8a1 1 0 100 2h4a1 1 0 100-2h-4z" />
        </svg>
      ),
      color: 'bg-red-500/20 text-red-400',
      description: 'removed a member',
    },
    member_role_changed: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-6-3a2 2 0 11-4 0 2 2 0 014 0zm-2 4a5 5 0 00-4.546 2.916A5.986 5.986 0 0010 16a5.986 5.986 0 004.546-2.084A5 5 0 0010 11z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-amber-500/20 text-amber-400',
      description: 'changed a role',
    },
    group_created: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z" />
        </svg>
      ),
      color: 'bg-violet-500/20 text-violet-400',
      description: 'created the group',
    },
    group_updated: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z" />
        </svg>
      ),
      color: 'bg-blue-500/20 text-blue-400',
      description: 'updated group settings',
    },
    share_link_created: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M12.586 4.586a2 2 0 112.828 2.828l-3 3a2 2 0 01-2.828 0 1 1 0 00-1.414 1.414 4 4 0 005.656 0l3-3a4 4 0 00-5.656-5.656l-1.5 1.5a1 1 0 101.414 1.414l1.5-1.5zm-5 5a2 2 0 012.828 0 1 1 0 101.414-1.414 4 4 0 00-5.656 0l-3 3a4 4 0 105.656 5.656l1.5-1.5a1 1 0 10-1.414-1.414l-1.5 1.5a2 2 0 11-2.828-2.828l3-3z" />
        </svg>
      ),
      color: 'bg-cyan-500/20 text-cyan-400',
      description: 'created a share link',
    },
    share_link_accessed: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
          <path fillRule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-green-500/20 text-green-400',
      description: 'accessed a share link',
    },
    share_link_revoked: {
      icon: (
        <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M13.477 14.89A6 6 0 015.11 6.524l8.367 8.368zm1.414-1.414L6.524 5.11a6 6 0 018.367 8.367zM18 10a8 8 0 11-16 0 8 8 0 0116 0z" clipRule="evenodd" />
        </svg>
      ),
      color: 'bg-red-500/20 text-red-400',
      description: 'revoked a share link',
    },
  };

  return displays[activity.action_type] || {
    icon: (
      <svg className={iconClass} fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
      </svg>
    ),
    color: 'bg-slate-500/20 text-slate-400',
    description: activity.action_type.replace(/_/g, ' '),
  };
}
