// ShardStatus Component for WRAITH Vault
// Displays the distribution status of shards across guardians

import { useEffect, useState, useCallback } from "react";
import type { SecretInfo, Guardian, ShardAssignment, DistributionStatus } from "../types";
import * as tauri from "../lib/tauri";

interface ShardStatusProps {
  secret: SecretInfo;
  guardians: Guardian[];
  onRefresh?: () => void;
  onDistribute?: () => void;
}

export function ShardStatus({
  secret,
  guardians,
  onRefresh,
  onDistribute,
}: ShardStatusProps) {
  const [assignments, setAssignments] = useState<ShardAssignment[]>([]);
  const [distributionStatus, setDistributionStatus] = useState<DistributionStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadShardData = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [assignmentsData, statusData] = await Promise.all([
        tauri.getShardAssignments(secret.id),
        tauri.getDistributionStatus(secret.id),
      ]);

      setAssignments(assignmentsData);
      setDistributionStatus(statusData);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }, [secret.id]);

  useEffect(() => {
    loadShardData();
  }, [loadShardData]);

  const getGuardianById = (id: string): Guardian | undefined => {
    return guardians.find((g) => g.id === id);
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case "distributed":
        return "bg-green-500";
      case "pending":
        return "bg-yellow-500";
      case "failed":
        return "bg-red-500";
      case "verified":
        return "bg-blue-500";
      default:
        return "bg-gray-500";
    }
  };

  const getStatusLabel = (status: string): string => {
    switch (status) {
      case "distributed":
        return "Distributed";
      case "pending":
        return "Pending";
      case "failed":
        return "Failed";
      case "verified":
        return "Verified";
      default:
        return "Unknown";
    }
  };

  const formatDate = (timestamp: number | null): string => {
    if (!timestamp) return "Never";
    return new Date(timestamp * 1000).toLocaleString();
  };

  const distributedCount = assignments.filter(
    (a) => a.status === "distributed" || a.status === "verified"
  ).length;

  const pendingCount = assignments.filter((a) => a.status === "pending").length;
  const failedCount = assignments.filter((a) => a.status === "failed").length;

  return (
    <div className="bg-gray-900 rounded-lg p-4">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Shard Distribution</h3>
          <p className="text-sm text-gray-400">
            {secret.name} - {secret.shamir_config.threshold}/
            {secret.shamir_config.total_shares} threshold
          </p>
        </div>
        <div className="flex gap-2">
          {onRefresh && (
            <button
              onClick={() => {
                loadShardData();
                onRefresh();
              }}
              disabled={loading}
              className="px-3 py-1 text-sm bg-gray-800 text-gray-300 rounded hover:bg-gray-700 disabled:opacity-50"
            >
              {loading ? "Loading..." : "Refresh"}
            </button>
          )}
          {onDistribute && !secret.distribution_complete && (
            <button
              onClick={onDistribute}
              className="px-3 py-1 text-sm bg-purple-600 text-white rounded hover:bg-purple-700"
            >
              Distribute
            </button>
          )}
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-4 p-3 bg-red-900/20 border border-red-800 rounded text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Distribution Summary */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-gray-800 rounded-lg p-4 text-center">
          <p className="text-2xl font-bold text-green-400">{distributedCount}</p>
          <p className="text-sm text-gray-400">Distributed</p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 text-center">
          <p className="text-2xl font-bold text-yellow-400">{pendingCount}</p>
          <p className="text-sm text-gray-400">Pending</p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 text-center">
          <p className="text-2xl font-bold text-red-400">{failedCount}</p>
          <p className="text-sm text-gray-400">Failed</p>
        </div>
      </div>

      {/* Overall Progress */}
      <div className="mb-6">
        <div className="flex items-center justify-between text-sm mb-2">
          <span className="text-gray-400">Distribution Progress</span>
          <span className="text-white">
            {distributedCount}/{secret.shamir_config.total_shares} shards
          </span>
        </div>
        <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${
              secret.distribution_complete ? "bg-green-500" : "bg-purple-600"
            }`}
            style={{
              width: `${
                (distributedCount / secret.shamir_config.total_shares) * 100
              }%`,
            }}
          />
        </div>
        {secret.distribution_complete && (
          <p className="text-xs text-green-400 mt-2">
            Distribution complete - Secret can be recovered
          </p>
        )}
      </div>

      {/* Guardian Shard List */}
      <div className="space-y-3">
        <h4 className="text-sm font-medium text-gray-300">Guardian Assignments</h4>

        {loading && assignments.length === 0 && (
          <div className="flex items-center justify-center py-8">
            <div className="w-6 h-6 border-2 border-purple-500 border-t-transparent rounded-full animate-spin" />
          </div>
        )}

        {!loading && assignments.length === 0 && (
          <div className="text-center py-8 text-gray-500">
            <p>No shards have been assigned yet</p>
            {onDistribute && (
              <button
                onClick={onDistribute}
                className="mt-2 text-purple-400 hover:text-purple-300"
              >
                Start distribution
              </button>
            )}
          </div>
        )}

        {assignments.map((assignment) => {
          const guardian = getGuardianById(assignment.guardian_id);
          const isOnline = guardian?.status === "online";

          return (
            <div
              key={`${assignment.secret_id}-${assignment.shard_index}`}
              className="bg-gray-800 rounded-lg p-4"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="flex flex-col items-center">
                    <span className="text-xs text-gray-500">Shard</span>
                    <span className="text-lg font-bold text-white">
                      #{assignment.shard_index + 1}
                    </span>
                  </div>
                  <div className="w-px h-10 bg-gray-700" />
                  <div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isOnline ? "bg-green-500" : "bg-gray-500"
                        }`}
                      />
                      <span className="font-medium text-white">
                        {guardian?.name || "Unknown Guardian"}
                      </span>
                    </div>
                    <p className="text-xs text-gray-500 truncate max-w-xs">
                      {guardian?.peer_id || assignment.guardian_id}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <div className="text-right">
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${getStatusColor(
                          assignment.status
                        )}`}
                      />
                      <span className="text-sm text-gray-300">
                        {getStatusLabel(assignment.status)}
                      </span>
                    </div>
                    <p className="text-xs text-gray-500">
                      {formatDate(assignment.assigned_at)}
                    </p>
                  </div>
                </div>
              </div>

              {/* Verification Status */}
              {assignment.status === "distributed" && (
                <div className="mt-3 pt-3 border-t border-gray-700">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-gray-500">
                      Last verified:{" "}
                      {assignment.verified_at
                        ? formatDate(assignment.verified_at)
                        : "Never"}
                    </span>
                    <button className="text-purple-400 hover:text-purple-300">
                      Verify Now
                    </button>
                  </div>
                </div>
              )}

              {/* Error Message for Failed Shards */}
              {assignment.status === "failed" && (
                <div className="mt-3 pt-3 border-t border-gray-700">
                  <p className="text-xs text-red-400">
                    Distribution failed - Guardian may be offline or unreachable
                  </p>
                  <button className="mt-2 text-xs text-purple-400 hover:text-purple-300">
                    Retry Distribution
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Distribution Status Details */}
      {distributionStatus && (
        <div className="mt-6 p-4 bg-gray-800 rounded-lg">
          <h4 className="text-sm font-medium text-gray-300 mb-3">
            Distribution Details
          </h4>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-gray-500">Started</span>
              <p className="text-white">
                {formatDate(distributionStatus.started_at)}
              </p>
            </div>
            <div>
              <span className="text-gray-500">Completed</span>
              <p className="text-white">
                {distributionStatus.completed_at
                  ? formatDate(distributionStatus.completed_at)
                  : "In Progress"}
              </p>
            </div>
            <div>
              <span className="text-gray-500">Total Shards</span>
              <p className="text-white">{distributionStatus.total_shards}</p>
            </div>
            <div>
              <span className="text-gray-500">Successful</span>
              <p className="text-white">
                {distributionStatus.successful_distributions}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Recovery Readiness */}
      <div className="mt-6 p-4 bg-gray-800 rounded-lg">
        <h4 className="text-sm font-medium text-gray-300 mb-3">
          Recovery Readiness
        </h4>
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Threshold Required</span>
            <span className="text-sm text-white">
              {secret.shamir_config.threshold} shards
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Available Shards</span>
            <span className="text-sm text-white">{distributedCount} shards</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Online Guardians</span>
            <span className="text-sm text-white">
              {guardians.filter((g) => g.status === "online").length} /{" "}
              {guardians.length}
            </span>
          </div>
          <div className="pt-2 border-t border-gray-700">
            <div className="flex items-center gap-2">
              {distributedCount >= secret.shamir_config.threshold ? (
                <>
                  <span className="w-3 h-3 rounded-full bg-green-500" />
                  <span className="text-sm text-green-400">
                    Ready for recovery
                  </span>
                </>
              ) : (
                <>
                  <span className="w-3 h-3 rounded-full bg-yellow-500" />
                  <span className="text-sm text-yellow-400">
                    Need {secret.shamir_config.threshold - distributedCount} more
                    shard(s)
                  </span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
