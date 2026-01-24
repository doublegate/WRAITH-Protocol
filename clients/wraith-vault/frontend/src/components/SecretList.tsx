// SecretList Component for WRAITH Vault

import { useEffect, useState } from "react";
import { useSecretStore } from "../stores/secretStore";
import type { SecretInfo, SecretType } from "../types";

// Secret type display information
const SECRET_TYPE_INFO: Record<
  SecretType,
  { label: string; icon: string; color: string }
> = {
  generic: { label: "Generic", icon: "file", color: "text-gray-400" },
  crypto_key: { label: "Crypto Key", icon: "key", color: "text-purple-400" },
  password: { label: "Password", icon: "lock", color: "text-blue-400" },
  recovery_phrase: {
    label: "Recovery Phrase",
    icon: "shield",
    color: "text-green-400",
  },
  certificate: {
    label: "Certificate",
    icon: "certificate",
    color: "text-yellow-400",
  },
  api_key: { label: "API Key", icon: "code", color: "text-orange-400" },
  document_key: { label: "Document Key", icon: "document", color: "text-cyan-400" },
  ssh_key: { label: "SSH Key", icon: "terminal", color: "text-pink-400" },
  pgp_key: { label: "PGP Key", icon: "mail", color: "text-indigo-400" },
};

interface SecretListProps {
  onSecretSelect?: (secret: SecretInfo) => void;
  onCreateNew?: () => void;
}

export function SecretList({ onSecretSelect, onCreateNew }: SecretListProps) {
  const {
    secrets,
    selectedSecret,
    loading,
    error,
    loadSecrets,
    selectSecret,
    searchSecrets,
  } = useSecretStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [filterType, setFilterType] = useState<SecretType | "all">("all");

  useEffect(() => {
    loadSecrets();
  }, [loadSecrets]);

  const handleSearch = async (query: string) => {
    setSearchQuery(query);
    if (query.trim()) {
      await searchSecrets(query);
    } else {
      await loadSecrets();
    }
  };

  const handleSelect = (secret: SecretInfo) => {
    selectSecret(secret);
    onSecretSelect?.(secret);
  };

  const filteredSecrets =
    filterType === "all"
      ? secrets
      : secrets.filter((s) => s.secret_type === filterType);

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  return (
    <div className="flex flex-col h-full bg-gray-900">
      {/* Header */}
      <div className="p-4 border-b border-gray-800">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white">Secrets</h2>
          {onCreateNew && (
            <button
              onClick={onCreateNew}
              className="px-3 py-1 text-sm bg-purple-600 text-white rounded hover:bg-purple-700 transition"
            >
              + New Secret
            </button>
          )}
        </div>

        {/* Search */}
        <input
          type="text"
          placeholder="Search secrets..."
          value={searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
          className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
        />

        {/* Filter by Type */}
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            onClick={() => setFilterType("all")}
            className={`px-2 py-1 text-xs rounded ${
              filterType === "all"
                ? "bg-purple-600 text-white"
                : "bg-gray-800 text-gray-400 hover:bg-gray-700"
            }`}
          >
            All
          </button>
          {(Object.keys(SECRET_TYPE_INFO) as SecretType[]).map((type) => (
            <button
              key={type}
              onClick={() => setFilterType(type)}
              className={`px-2 py-1 text-xs rounded ${
                filterType === type
                  ? "bg-purple-600 text-white"
                  : "bg-gray-800 text-gray-400 hover:bg-gray-700"
              }`}
            >
              {SECRET_TYPE_INFO[type].label}
            </button>
          ))}
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

      {/* Secret List */}
      <div className="flex-1 overflow-y-auto">
        {!loading && filteredSecrets.length === 0 && (
          <div className="flex flex-col items-center justify-center py-12 text-gray-500">
            <p className="mb-2">No secrets found</p>
            {onCreateNew && (
              <button
                onClick={onCreateNew}
                className="text-purple-400 hover:text-purple-300"
              >
                Create your first secret
              </button>
            )}
          </div>
        )}

        {filteredSecrets.map((secret) => {
          const typeInfo = SECRET_TYPE_INFO[secret.secret_type];
          const isSelected = selectedSecret?.id === secret.id;

          return (
            <div
              key={secret.id}
              onClick={() => handleSelect(secret)}
              className={`p-4 border-b border-gray-800 cursor-pointer transition ${
                isSelected
                  ? "bg-purple-900/20 border-l-2 border-l-purple-500"
                  : "hover:bg-gray-800/50"
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className={`text-sm ${typeInfo.color}`}>
                      {typeInfo.label}
                    </span>
                    {secret.distribution_complete ? (
                      <span className="px-1.5 py-0.5 text-xs bg-green-900/30 text-green-400 rounded">
                        Distributed
                      </span>
                    ) : (
                      <span className="px-1.5 py-0.5 text-xs bg-yellow-900/30 text-yellow-400 rounded">
                        Pending
                      </span>
                    )}
                  </div>
                  <h3 className="font-medium text-white truncate mt-1">
                    {secret.name}
                  </h3>
                  {secret.description && (
                    <p className="text-sm text-gray-400 truncate mt-1">
                      {secret.description}
                    </p>
                  )}
                </div>
              </div>

              <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
                <span>
                  {secret.shamir_config.threshold}/{secret.shamir_config.total_shares} threshold
                </span>
                <span>{secret.guardian_ids.length} guardians</span>
                <span>Created {formatDate(secret.created_at)}</span>
              </div>

              {secret.tags.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {secret.tags.slice(0, 3).map((tag) => (
                    <span
                      key={tag}
                      className="px-1.5 py-0.5 text-xs bg-gray-800 text-gray-400 rounded"
                    >
                      {tag}
                    </span>
                  ))}
                  {secret.tags.length > 3 && (
                    <span className="text-xs text-gray-500">
                      +{secret.tags.length - 3} more
                    </span>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
