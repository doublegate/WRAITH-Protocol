// CreateSecretModal Component for WRAITH Vault
// Modal for creating new secrets with Shamir configuration

import { useState, useEffect, Fragment } from "react";
import { useSecretStore } from "../stores/secretStore";
import { useGuardianStore } from "../stores/guardianStore";
import type { SecretType } from "../types";

interface CreateSecretModalProps {
  onClose: () => void;
  onCreated: () => void;
}

const SECRET_TYPES: { value: SecretType; label: string; description: string }[] = [
  { value: "generic", label: "Generic", description: "Any type of secret data" },
  { value: "password", label: "Password", description: "Password or passphrase" },
  { value: "crypto_key", label: "Crypto Key", description: "Cryptographic key material" },
  { value: "recovery_phrase", label: "Recovery Phrase", description: "Wallet recovery phrase" },
  { value: "api_key", label: "API Key", description: "API key or token" },
  { value: "ssh_key", label: "SSH Key", description: "SSH private key" },
  { value: "pgp_key", label: "PGP Key", description: "PGP/GPG private key" },
  { value: "certificate", label: "Certificate", description: "X.509 certificate" },
  { value: "document_key", label: "Document Key", description: "Document encryption key" },
];

export function CreateSecretModal({ onClose, onCreated }: CreateSecretModalProps) {
  const { createSecret, loading, error } = useSecretStore();
  const { guardians, availableGuardians, loadGuardians, loadAvailableGuardians } = useGuardianStore();

  const [step, setStep] = useState<"details" | "config" | "guardians" | "confirm">("details");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [secretType, setSecretType] = useState<SecretType>("generic");
  const [secretData, setSecretData] = useState("");
  const [threshold, setThreshold] = useState(3);
  const [totalShares, setTotalShares] = useState(5);
  const [selectedGuardianIds, setSelectedGuardianIds] = useState<string[]>([]);
  const [tags, setTags] = useState("");
  const [autoDistribute, setAutoDistribute] = useState(true);
  const [validationError, setValidationError] = useState<string | null>(null);

  useEffect(() => {
    loadGuardians();
    loadAvailableGuardians();
  }, [loadGuardians, loadAvailableGuardians]);

  useEffect(() => {
    // Auto-adjust threshold if total shares changes
    if (threshold > totalShares) {
      setThreshold(Math.max(2, totalShares - 1));
    }
  }, [totalShares, threshold]);

  const validateStep = (): boolean => {
    setValidationError(null);

    switch (step) {
      case "details":
        if (!name.trim()) {
          setValidationError("Secret name is required");
          return false;
        }
        if (!secretData.trim()) {
          setValidationError("Secret data is required");
          return false;
        }
        return true;

      case "config":
        if (threshold < 2) {
          setValidationError("Threshold must be at least 2");
          return false;
        }
        if (threshold > totalShares) {
          setValidationError("Threshold cannot exceed total shares");
          return false;
        }
        if (totalShares > 20) {
          setValidationError("Maximum 20 shares allowed");
          return false;
        }
        return true;

      case "guardians":
        if (selectedGuardianIds.length < totalShares) {
          setValidationError(
            `Select at least ${totalShares} guardians for ${totalShares} shares`
          );
          return false;
        }
        return true;

      default:
        return true;
    }
  };

  const handleNext = () => {
    if (!validateStep()) return;

    switch (step) {
      case "details":
        setStep("config");
        break;
      case "config":
        setStep("guardians");
        break;
      case "guardians":
        setStep("confirm");
        break;
    }
  };

  const handleBack = () => {
    switch (step) {
      case "config":
        setStep("details");
        break;
      case "guardians":
        setStep("config");
        break;
      case "confirm":
        setStep("guardians");
        break;
    }
  };

  const handleGuardianToggle = (guardianId: string) => {
    setSelectedGuardianIds((prev) =>
      prev.includes(guardianId)
        ? prev.filter((id) => id !== guardianId)
        : [...prev, guardianId]
    );
  };

  const handleCreate = async () => {
    if (!validateStep()) return;

    try {
      const secretBytes = new TextEncoder().encode(secretData);
      const tagList = tags
        .split(",")
        .map((t) => t.trim())
        .filter((t) => t);

      await createSecret(
        name,
        secretBytes,
        secretType,
        description || null,
        threshold,
        totalShares,
        tagList,
        null // password
      );

      onCreated();
    } catch (err) {
      console.error("Failed to create secret:", err);
    }
  };

  const onlineGuardians = availableGuardians.filter((g) => g.status === "online");

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-900 rounded-lg w-full max-w-2xl max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="p-4 border-b border-gray-800">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-white">Create New Secret</h2>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-white transition"
            >
              Close
            </button>
          </div>

          {/* Step Indicator */}
          <div className="flex items-center mt-4 text-sm">
            {["Details", "Configuration", "Guardians", "Confirm"].map(
              (label, index) => {
                const steps: typeof step[] = ["details", "config", "guardians", "confirm"];
                const isActive = step === steps[index];
                const isPast = steps.indexOf(step) > index;

                return (
                  <Fragment key={label}>
                    <div
                      className={`flex items-center gap-2 ${
                        isActive
                          ? "text-purple-400"
                          : isPast
                          ? "text-green-400"
                          : "text-gray-500"
                      }`}
                    >
                      <span
                        className={`w-6 h-6 rounded-full flex items-center justify-center text-xs ${
                          isActive
                            ? "bg-purple-600"
                            : isPast
                            ? "bg-green-600"
                            : "bg-gray-700"
                        }`}
                      >
                        {isPast ? "+" : index + 1}
                      </span>
                      <span className="hidden sm:inline">{label}</span>
                    </div>
                    {index < 3 && (
                      <div
                        className={`flex-1 h-px mx-2 ${
                          isPast ? "bg-green-600" : "bg-gray-700"
                        }`}
                      />
                    )}
                  </Fragment>
                );
              }
            )}
          </div>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto max-h-[60vh]">
          {/* Error Display */}
          {(validationError || error) && (
            <div className="mb-4 p-3 bg-red-900/20 border border-red-800 rounded text-red-400 text-sm">
              {validationError || error}
            </div>
          )}

          {/* Step 1: Details */}
          {step === "details" && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Secret Name *
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="My Important Secret"
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Description
                </label>
                <input
                  type="text"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="Optional description..."
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Secret Type
                </label>
                <select
                  value={secretType}
                  onChange={(e) => setSecretType(e.target.value as SecretType)}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white focus:outline-none focus:ring-2 focus:ring-purple-500"
                >
                  {SECRET_TYPES.map((type) => (
                    <option key={type.value} value={type.value}>
                      {type.label} - {type.description}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Secret Data *
                </label>
                <textarea
                  value={secretData}
                  onChange={(e) => setSecretData(e.target.value)}
                  placeholder="Enter your secret data..."
                  rows={4}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono"
                />
                <p className="text-xs text-gray-500 mt-1">
                  This data will be encrypted and split across guardians
                </p>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">
                  Tags (comma-separated)
                </label>
                <input
                  type="text"
                  value={tags}
                  onChange={(e) => setTags(e.target.value)}
                  placeholder="important, backup, crypto"
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>
            </div>
          )}

          {/* Step 2: Configuration */}
          {step === "config" && (
            <div className="space-y-6">
              <div>
                <h3 className="text-md font-medium mb-4">
                  Shamir Secret Sharing Configuration
                </h3>
                <p className="text-sm text-gray-400 mb-4">
                  Configure how your secret will be split and distributed across
                  guardians.
                </p>
              </div>

              <div className="grid grid-cols-2 gap-6">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">
                    Threshold (k) *
                  </label>
                  <input
                    type="number"
                    value={threshold}
                    onChange={(e) => setThreshold(parseInt(e.target.value) || 2)}
                    min={2}
                    max={totalShares}
                    className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white focus:outline-none focus:ring-2 focus:ring-purple-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    Minimum shards needed for recovery
                  </p>
                </div>

                <div>
                  <label className="block text-sm text-gray-400 mb-1">
                    Total Shares (n) *
                  </label>
                  <input
                    type="number"
                    value={totalShares}
                    onChange={(e) => setTotalShares(parseInt(e.target.value) || 2)}
                    min={threshold}
                    max={20}
                    className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white focus:outline-none focus:ring-2 focus:ring-purple-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    Total shards to create
                  </p>
                </div>
              </div>

              {/* Visual Representation */}
              <div className="bg-gray-800 rounded-lg p-4">
                <div className="flex items-center justify-center gap-2 mb-4">
                  {Array.from({ length: totalShares }).map((_, i) => (
                    <div
                      key={i}
                      className={`w-8 h-8 rounded-full flex items-center justify-center text-xs ${
                        i < threshold
                          ? "bg-purple-600 text-white"
                          : "bg-gray-700 text-gray-400"
                      }`}
                    >
                      {i + 1}
                    </div>
                  ))}
                </div>
                <p className="text-center text-sm text-gray-400">
                  {threshold}-of-{totalShares} scheme: You need any{" "}
                  <span className="text-purple-400">{threshold}</span> of{" "}
                  <span className="text-white">{totalShares}</span> shards to
                  recover your secret.
                </p>
              </div>

              <div className="flex items-center gap-3">
                <input
                  type="checkbox"
                  id="autoDistribute"
                  checked={autoDistribute}
                  onChange={(e) => setAutoDistribute(e.target.checked)}
                  className="h-4 w-4 rounded border-gray-600 bg-gray-800 text-purple-600"
                />
                <label htmlFor="autoDistribute" className="text-sm text-gray-300">
                  Automatically distribute to guardians after creation
                </label>
              </div>
            </div>
          )}

          {/* Step 3: Guardians */}
          {step === "guardians" && (
            <div className="space-y-4">
              <div>
                <h3 className="text-md font-medium mb-2">Select Guardians</h3>
                <p className="text-sm text-gray-400">
                  Select {totalShares} guardian(s) to hold your secret shards.{" "}
                  {onlineGuardians.length} guardian(s) currently online.
                </p>
              </div>

              <div className="text-sm text-gray-400">
                Selected: {selectedGuardianIds.length} / {totalShares} required
              </div>

              <div className="space-y-2 max-h-64 overflow-y-auto">
                {guardians.map((guardian) => {
                  const isSelected = selectedGuardianIds.includes(guardian.id);
                  const isOnline = guardian.status === "online";

                  return (
                    <div
                      key={guardian.id}
                      onClick={() => handleGuardianToggle(guardian.id)}
                      className={`p-3 rounded-lg cursor-pointer transition ${
                        isSelected
                          ? "bg-purple-900/30 border border-purple-500"
                          : "bg-gray-800 hover:bg-gray-700"
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => {}}
                          className="h-4 w-4 rounded border-gray-600 bg-gray-800 text-purple-600"
                        />
                        <div className="flex-1">
                          <div className="flex items-center gap-2">
                            <span
                              className={`w-2 h-2 rounded-full ${
                                isOnline ? "bg-green-500" : "bg-gray-500"
                              }`}
                            />
                            <span className="font-medium text-white">
                              {guardian.name}
                            </span>
                          </div>
                          <p className="text-xs text-gray-500 truncate">
                            {guardian.peer_id}
                          </p>
                        </div>
                        <span
                          className={`text-xs ${
                            isOnline ? "text-green-400" : "text-gray-500"
                          }`}
                        >
                          {isOnline ? "Online" : "Offline"}
                        </span>
                      </div>
                    </div>
                  );
                })}

                {guardians.length === 0 && (
                  <div className="text-center py-8 text-gray-500">
                    <p>No guardians available</p>
                    <p className="text-sm">Add guardians before creating secrets</p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Step 4: Confirm */}
          {step === "confirm" && (
            <div className="space-y-4">
              <h3 className="text-md font-medium mb-4">Review and Confirm</h3>

              <div className="bg-gray-800 rounded-lg p-4 space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Name</span>
                  <span className="text-white">{name}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Type</span>
                  <span className="text-white">
                    {SECRET_TYPES.find((t) => t.value === secretType)?.label}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Threshold</span>
                  <span className="text-white">
                    {threshold}-of-{totalShares}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Guardians</span>
                  <span className="text-white">{selectedGuardianIds.length}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Auto-distribute</span>
                  <span className="text-white">{autoDistribute ? "Yes" : "No"}</span>
                </div>
              </div>

              <div className="bg-yellow-900/20 border border-yellow-800 rounded-lg p-4">
                <p className="text-sm text-yellow-400">
                  Make sure you have selected trusted guardians. Once distributed,
                  you will need {threshold} guardians to recover your secret.
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-gray-800 flex items-center justify-between">
          <button
            onClick={step === "details" ? onClose : handleBack}
            className="px-4 py-2 text-gray-400 hover:text-white transition"
          >
            {step === "details" ? "Cancel" : "Back"}
          </button>

          {step !== "confirm" ? (
            <button
              onClick={handleNext}
              className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 transition"
            >
              Next
            </button>
          ) : (
            <button
              onClick={handleCreate}
              disabled={loading}
              className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 transition"
            >
              {loading ? "Creating..." : "Create Secret"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
