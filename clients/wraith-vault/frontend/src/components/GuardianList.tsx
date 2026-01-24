// GuardianList Component for WRAITH Vault

import type { MouseEvent } from "react";
import { useEffect, useState } from "react";
import { useGuardianStore } from "../stores/guardianStore";
import type { Guardian, GuardianStatus, TrustLevel } from "../types";

// Status display information
const STATUS_INFO: Record<GuardianStatus, { label: string; color: string }> = {
  online: { label: "Online", color: "bg-green-500" },
  offline: { label: "Offline", color: "bg-gray-500" },
  pending: { label: "Pending", color: "bg-yellow-500" },
  failed: { label: "Failed", color: "bg-red-500" },
  revoked: { label: "Revoked", color: "bg-gray-700" },
};

// Trust level display information
const TRUST_INFO: Record<TrustLevel, { label: string; color: string }> = {
  untrusted: { label: "Untrusted", color: "text-red-400" },
  basic: { label: "Basic", color: "text-yellow-400" },
  trusted: { label: "Trusted", color: "text-blue-400" },
  high: { label: "High", color: "text-green-400" },
  ultimate: { label: "Ultimate", color: "text-purple-400" },
};

interface GuardianListProps {
  onGuardianSelect?: (guardian: Guardian) => void;
  onAddNew?: () => void;
  selectable?: boolean;
  selectedIds?: string[];
  onSelectionChange?: (ids: string[]) => void;
}

export function GuardianList({
  onGuardianSelect,
  onAddNew,
  selectable = false,
  selectedIds = [],
  onSelectionChange,
}: GuardianListProps) {
  const {
    guardians,
    selectedGuardian,
    loading,
    error,
    loadGuardians,
    selectGuardian,
    recordHealthCheck,
  } = useGuardianStore();

  const [filterStatus, setFilterStatus] = useState<GuardianStatus | "all">(
    "all"
  );
  const [checkingHealth, setCheckingHealth] = useState<string | null>(null);

  useEffect(() => {
    loadGuardians();
  }, [loadGuardians]);

  const handleSelect = (guardian: Guardian) => {
    if (selectable && onSelectionChange) {
      const newSelection = selectedIds.includes(guardian.id)
        ? selectedIds.filter((id) => id !== guardian.id)
        : [...selectedIds, guardian.id];
      onSelectionChange(newSelection);
    } else {
      selectGuardian(guardian);
      onGuardianSelect?.(guardian);
    }
  };

  const handleHealthCheck = async (
    e: MouseEvent,
    guardian: Guardian
  ) => {
    e.stopPropagation();
    setCheckingHealth(guardian.id);

    // Simulate health check (in real implementation, this would ping the guardian)
    const success = Math.random() > 0.3; // 70% success rate for demo
    const responseTime = success ? Math.floor(Math.random() * 200) + 50 : null;
    const errorMsg = success ? null : "Connection timeout";

    await new Promise((resolve) => setTimeout(resolve, 500)); // Simulate network delay

    await recordHealthCheck(guardian.id, success, responseTime, errorMsg);
    setCheckingHealth(null);
  };

  const filteredGuardians =
    filterStatus === "all"
      ? guardians
      : guardians.filter((g) => g.status === filterStatus);

  const formatDate = (timestamp: number | null) => {
    if (!timestamp) return "Never";
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const formatLastSeen = (timestamp: number | null) => {
    if (!timestamp) return "Never";
    const diff = Date.now() / 1000 - timestamp;
    if (diff < 60) return "Just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return formatDate(timestamp);
  };

  return (
    <div className="flex flex-col h-full bg-gray-900">
      {/* Header */}
      <div className="p-4 border-b border-gray-800">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white">Guardians</h2>
          {onAddNew && (
            <button
              onClick={onAddNew}
              className="px-3 py-1 text-sm bg-purple-600 text-white rounded hover:bg-purple-700 transition"
            >
              + Add Guardian
            </button>
          )}
        </div>

        {/* Status Filter */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setFilterStatus("all")}
            className={`px-2 py-1 text-xs rounded ${
              filterStatus === "all"
                ? "bg-purple-600 text-white"
                : "bg-gray-800 text-gray-400 hover:bg-gray-700"
            }`}
          >
            All ({guardians.length})
          </button>
          {(Object.keys(STATUS_INFO) as GuardianStatus[]).map((status) => {
            const count = guardians.filter((g) => g.status === status).length;
            const info = STATUS_INFO[status];
            return (
              <button
                key={status}
                onClick={() => setFilterStatus(status)}
                className={`px-2 py-1 text-xs rounded flex items-center gap-1 ${
                  filterStatus === status
                    ? "bg-purple-600 text-white"
                    : "bg-gray-800 text-gray-400 hover:bg-gray-700"
                }`}
              >
                <span className={`w-2 h-2 rounded-full ${info.color}`}></span>
                {info.label} ({count})
              </button>
            );
          })}
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="px-4 py-2 bg-red-900/20 border-b border-red-800 text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Loading State */}
      {loading && (
        <div className="flex items-center justify-center py-8">
          <div className="w-6 h-6 border-2 border-purple-500 border-t-transparent rounded-full animate-spin"></div>
        </div>
      )}

      {/* Guardian List */}
      <div className="flex-1 overflow-y-auto">
        {!loading && filteredGuardians.length === 0 && (
          <div className="flex flex-col items-center justify-center py-12 text-gray-500">
            <p className="mb-2">No guardians found</p>
            {onAddNew && (
              <button
                onClick={onAddNew}
                className="text-purple-400 hover:text-purple-300"
              >
                Add your first guardian
              </button>
            )}
          </div>
        )}

        {filteredGuardians.map((guardian) => {
          const statusInfo = STATUS_INFO[guardian.status];
          const trustInfo = TRUST_INFO[guardian.trust_level];
          const isSelected = selectable
            ? selectedIds.includes(guardian.id)
            : selectedGuardian?.id === guardian.id;

          return (
            <div
              key={guardian.id}
              onClick={() => handleSelect(guardian)}
              className={`p-4 border-b border-gray-800 cursor-pointer transition ${
                isSelected
                  ? "bg-purple-900/20 border-l-2 border-l-purple-500"
                  : "hover:bg-gray-800/50"
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  {selectable && (
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => {}}
                      className="mt-1 h-4 w-4 rounded border-gray-600 bg-gray-800 text-purple-600 focus:ring-purple-500"
                    />
                  )}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${statusInfo.color}`}
                      ></span>
                      <h3 className="font-medium text-white truncate">
                        {guardian.name}
                      </h3>
                      <span className={`text-xs ${trustInfo.color}`}>
                        {trustInfo.label}
                      </span>
                    </div>
                    <p className="text-sm text-gray-400 truncate mt-1">
                      {guardian.peer_id}
                    </p>
                  </div>
                </div>

                {/* Health Check Button */}
                <button
                  onClick={(e) => handleHealthCheck(e, guardian)}
                  disabled={checkingHealth === guardian.id}
                  className="px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded hover:bg-gray-700 disabled:opacity-50"
                >
                  {checkingHealth === guardian.id ? (
                    <span className="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin inline-block"></span>
                  ) : (
                    "Check"
                  )}
                </button>
              </div>

              <div className="flex items-center gap-4 mt-2 text-xs text-gray-500 ml-7">
                <span>Shares: {guardian.shares_held}</span>
                <span>Recoveries: {guardian.successful_recoveries}</span>
                <span>Last seen: {formatLastSeen(guardian.last_seen)}</span>
              </div>

              {guardian.notes && (
                <p className="text-xs text-gray-500 mt-2 ml-7 truncate">
                  {guardian.notes}
                </p>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
