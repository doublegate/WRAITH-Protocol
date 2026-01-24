// SecretDetail Component for WRAITH Vault
// Displays detailed information about a selected secret

import { useState } from "react";
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

interface SecretDetailProps {
  secret: SecretInfo;
  onRecover: () => void;
  onClose: () => void;
}

export function SecretDetail({ secret, onRecover, onClose }: SecretDetailProps) {
  const { deleteSecret, loading, error } = useSecretStore();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showRotateConfirm, setShowRotateConfirm] = useState(false);

  const typeInfo = SECRET_TYPE_INFO[secret.secret_type];

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const handleRotate = async () => {
    try {
      // In a real implementation, this would trigger key rotation via Tauri
      // For now, just close the confirmation dialog
      // TODO: Implement key rotation when backend support is added
      setShowRotateConfirm(false);
    } catch (err) {
      console.error("Failed to rotate secret:", err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteSecret(secret.id);
      setShowDeleteConfirm(false);
      onClose();
    } catch (err) {
      console.error("Failed to delete secret:", err);
    }
  };

  return (
    <div className="bg-gray-900 rounded-lg">
      {/* Header */}
      <div className="p-6 border-b border-gray-800">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-3">
              <span className={`text-lg ${typeInfo.color}`}>{typeInfo.label}</span>
              {secret.distribution_complete ? (
                <span className="px-2 py-0.5 text-xs bg-green-900/30 text-green-400 rounded">
                  Distributed
                </span>
              ) : (
                <span className="px-2 py-0.5 text-xs bg-yellow-900/30 text-yellow-400 rounded">
                  Pending Distribution
                </span>
              )}
            </div>
            <h2 className="text-2xl font-bold text-white mt-2">{secret.name}</h2>
            {secret.description && (
              <p className="text-gray-400 mt-1">{secret.description}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="text-gray-500 hover:text-white transition"
          >
            Close
          </button>
        </div>

        {/* Tags */}
        {secret.tags.length > 0 && (
          <div className="flex flex-wrap gap-2 mt-4">
            {secret.tags.map((tag) => (
              <span
                key={tag}
                className="px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div className="mx-6 mt-4 p-3 bg-red-900/20 border border-red-800 rounded text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Actions */}
      <div className="p-6 border-b border-gray-800">
        <div className="flex flex-wrap gap-3">
          <button
            onClick={onRecover}
            disabled={!secret.distribution_complete}
            className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
          >
            Recover Secret
          </button>
          <button
            onClick={() => setShowRotateConfirm(true)}
            disabled={!secret.distribution_complete}
            className="px-4 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
          >
            Rotate Key
          </button>
          <button
            onClick={() => setShowDeleteConfirm(true)}
            className="px-4 py-2 bg-red-900/30 text-red-400 rounded hover:bg-red-900/50 transition"
          >
            Delete
          </button>
        </div>
      </div>

      {/* Details */}
      <div className="p-6 space-y-6">
        {/* Sharing Configuration */}
        <section>
          <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            Sharing Configuration
          </h3>
          <div className="bg-gray-800 rounded-lg p-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-gray-500">Threshold (k)</p>
                <p className="text-xl font-bold text-white">
                  {secret.shamir_config.threshold}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Total Shares (n)</p>
                <p className="text-xl font-bold text-white">
                  {secret.shamir_config.total_shares}
                </p>
              </div>
            </div>
            <div className="mt-4 pt-4 border-t border-gray-700">
              <p className="text-sm text-gray-400">
                This secret requires{" "}
                <span className="text-purple-400 font-medium">
                  {secret.shamir_config.threshold}
                </span>{" "}
                of{" "}
                <span className="text-white font-medium">
                  {secret.shamir_config.total_shares}
                </span>{" "}
                shards to recover.
              </p>
            </div>
          </div>
        </section>

        {/* Guardians */}
        <section>
          <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            Assigned Guardians
          </h3>
          <div className="bg-gray-800 rounded-lg p-4">
            <p className="text-lg font-bold text-white">
              {secret.guardian_ids.length} Guardian(s)
            </p>
            <p className="text-sm text-gray-500 mt-1">
              Shards are distributed across these trusted peers
            </p>
            {secret.guardian_ids.length > 0 && (
              <div className="mt-3 space-y-2">
                {secret.guardian_ids.slice(0, 5).map((id) => (
                  <div
                    key={id}
                    className="text-xs text-gray-500 font-mono truncate"
                  >
                    {id}
                  </div>
                ))}
                {secret.guardian_ids.length > 5 && (
                  <p className="text-xs text-gray-500">
                    +{secret.guardian_ids.length - 5} more
                  </p>
                )}
              </div>
            )}
          </div>
        </section>

        {/* Timestamps */}
        <section>
          <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            Timeline
          </h3>
          <div className="bg-gray-800 rounded-lg p-4 space-y-3">
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Created</span>
              <span className="text-sm text-white">
                {formatDate(secret.created_at)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Last Updated</span>
              <span className="text-sm text-white">
                {formatDate(secret.updated_at)}
              </span>
            </div>
            {secret.last_accessed && (
              <div className="flex justify-between">
                <span className="text-sm text-gray-500">Last Accessed</span>
                <span className="text-sm text-white">
                  {formatDate(secret.last_accessed)}
                </span>
              </div>
            )}
            {secret.rotation_count > 0 && (
              <div className="flex justify-between">
                <span className="text-sm text-gray-500">Rotations</span>
                <span className="text-sm text-white">
                  {secret.rotation_count}
                </span>
              </div>
            )}
          </div>
        </section>

        {/* Technical Details */}
        <section>
          <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            Technical Details
          </h3>
          <div className="bg-gray-800 rounded-lg p-4 space-y-3">
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Secret ID</span>
              <span className="text-sm text-white font-mono truncate max-w-xs">
                {secret.id}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Version</span>
              <span className="text-sm text-white">{secret.version}</span>
            </div>
            {secret.shamir_config.key_salt && (
              <div className="flex justify-between">
                <span className="text-sm text-gray-500">Key Salt</span>
                <span className="text-sm text-white font-mono truncate max-w-xs">
                  {secret.shamir_config.key_salt.substring(0, 16)}...
                </span>
              </div>
            )}
          </div>
        </section>
      </div>

      {/* Delete Confirmation Dialog */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-900 rounded-lg p-6 max-w-md">
            <h3 className="text-lg font-semibold text-white mb-2">
              Delete Secret?
            </h3>
            <p className="text-gray-400 mb-4">
              This will permanently delete "{secret.name}" and revoke all
              distributed shards. This action cannot be undone.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowDeleteConfirm(false)}
                className="px-4 py-2 text-gray-400 hover:text-white transition"
              >
                Cancel
              </button>
              <button
                onClick={handleDelete}
                disabled={loading}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50 transition"
              >
                {loading ? "Deleting..." : "Delete"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Rotate Confirmation Dialog */}
      {showRotateConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-900 rounded-lg p-6 max-w-md">
            <h3 className="text-lg font-semibold text-white mb-2">
              Rotate Secret?
            </h3>
            <p className="text-gray-400 mb-4">
              This will create new shards with a new encryption key and
              redistribute them to guardians. Existing shards will be revoked.
            </p>
            <div className="bg-yellow-900/20 border border-yellow-800 rounded-lg p-3 mb-4">
              <p className="text-sm text-yellow-400">
                All guardians must be online for rotation to complete
                successfully.
              </p>
            </div>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowRotateConfirm(false)}
                className="px-4 py-2 text-gray-400 hover:text-white transition"
              >
                Cancel
              </button>
              <button
                onClick={handleRotate}
                disabled={loading}
                className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 transition"
              >
                {loading ? "Rotating..." : "Rotate"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
