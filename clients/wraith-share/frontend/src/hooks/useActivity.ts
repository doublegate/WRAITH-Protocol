// useActivity Hook - Convenience hook for activity operations

import { useCallback, useEffect } from 'react';
import { useActivityStore } from '../stores/activityStore';
import { useGroupStore } from '../stores/groupStore';

export function useActivity() {
  const { selectedGroupId } = useGroupStore();
  const {
    activities,
    stats,
    loading,
    error,
    hasMore,
    fetchActivities,
    fetchMoreActivities,
    fetchRecentActivities,
    fetchStats,
    searchActivities,
    clearActivities,
    clearError,
  } = useActivityStore();

  // Fetch activities when group changes
  useEffect(() => {
    if (selectedGroupId) {
      clearActivities();
      fetchActivities(selectedGroupId);
      fetchStats(selectedGroupId);
    } else {
      clearActivities();
      fetchRecentActivities();
    }
  }, [selectedGroupId, fetchActivities, fetchRecentActivities, fetchStats, clearActivities]);

  const loadMore = useCallback(() => {
    if (selectedGroupId && hasMore && !loading) {
      fetchMoreActivities(selectedGroupId);
    }
  }, [selectedGroupId, hasMore, loading, fetchMoreActivities]);

  const search = useCallback(
    (query: string) => {
      if (selectedGroupId) {
        searchActivities(selectedGroupId, query);
      }
    },
    [selectedGroupId, searchActivities]
  );

  const refresh = useCallback(() => {
    if (selectedGroupId) {
      clearActivities();
      fetchActivities(selectedGroupId);
      fetchStats(selectedGroupId);
    } else {
      clearActivities();
      fetchRecentActivities();
    }
  }, [selectedGroupId, fetchActivities, fetchRecentActivities, fetchStats, clearActivities]);

  return {
    activities,
    stats,
    loading,
    error,
    hasMore,
    loadMore,
    search,
    refresh,
    clearError,
  };
}

export function useGroupActivity(groupId: string) {
  const {
    activities,
    stats,
    loading,
    hasMore,
    fetchActivities,
    fetchMoreActivities,
    fetchStats,
    clearActivities,
  } = useActivityStore();

  useEffect(() => {
    clearActivities();
    fetchActivities(groupId);
    fetchStats(groupId);
  }, [groupId, fetchActivities, fetchStats, clearActivities]);

  const loadMore = useCallback(() => {
    if (hasMore && !loading) {
      fetchMoreActivities(groupId);
    }
  }, [groupId, hasMore, loading, fetchMoreActivities]);

  return {
    activities,
    stats,
    loading,
    hasMore,
    loadMore,
    refresh: () => {
      clearActivities();
      fetchActivities(groupId);
      fetchStats(groupId);
    },
  };
}
