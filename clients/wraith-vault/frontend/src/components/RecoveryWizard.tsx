// RecoveryWizard Component for WRAITH Vault
// Multi-step wizard for recovering secrets from guardian shards

import React, { useState, useEffect } from "react";
import { useRecoveryStore } from "../stores/recoveryStore";
import { useGuardianStore } from "../stores/guardianStore";
import { useSecretStore } from "../stores/secretStore";
import type { SecretInfo, Guardian, EncryptedShard, RecoveryProgress } from "../types";
import * as tauri from "../lib/tauri";

type WizardStep = "select-secret" | "select-guardians" | "collect-shards" | "recovering" | "complete" | "error";

interface RecoveryWizardProps {
  onComplete?: (secretData: Uint8Array) => void;
  onCancel?: () => void;
  preselectedSecretId?: string;
}

export function RecoveryWizard({
  onComplete,
  onCancel,
  preselectedSecretId,
}: RecoveryWizardProps) {
  const { secrets, loadSecrets } = useSecretStore();
  const { guardians, loadGuardians } = useGuardianStore();
  const {
    activeSessionId,
    progress,
    result,
    loading,
    error,
    startRecovery,
    addShard,
    completeRecovery,
    getProgress,
    cancelRecovery,
    clearResult,
    clearError,
  } = useRecoveryStore();

  const [step, setStep] = useState<WizardStep>("select-secret");
  const [selectedSecret, setSelectedSecret] = useState<SecretInfo | null>(null);
  const [selectedGuardianIds, setSelectedGuardianIds] = useState<string[]>([]);
  const [collectedShards, setCollectedShards] = useState<Map<string, EncryptedShard>>(new Map());
  const [shardEncryptionKeys, setShardEncryptionKeys] = useState<Map<string, string>>(new Map());
  const [currentGuardianIndex, setCurrentGuardianIndex] = useState(0);
  const [manualShardInput, setManualShardInput] = useState("");
  const [manualKeyInput, setManualKeyInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    loadSecrets();
    loadGuardians();
  }, [loadSecrets, loadGuardians]);

  useEffect(() => {
    if (preselectedSecretId) {
      const secret = secrets.find((s) => s.id === preselectedSecretId);
      if (secret) {
        setSelectedSecret(secret);
        setStep("select-guardians");
      }
    }
  }, [preselectedSecretId, secrets]);

  useEffect(() => {
    if (error) {
      setErrorMessage(error);
      setStep("error");
    }
  }, [error]);

  const secretGuardians = selectedSecret
    ? guardians.filter((g) => selectedSecret.guardian_ids.includes(g.id))
    : [];

  const onlineGuardians = secretGuardians.filter((g) => g.status === "online");

  const handleSelectSecret = (secret: SecretInfo) => {
    setSelectedSecret(secret);
    setSelectedGuardianIds([]);
    setCollectedShards(new Map());
    setShardEncryptionKeys(new Map());
    setStep("select-guardians");
  };

  const handleGuardianToggle = (guardianId: string) => {
    setSelectedGuardianIds((prev) =>
      prev.includes(guardianId)
        ? prev.filter((id) => id !== guardianId)
        : [...prev, guardianId]
    );
  };

  const handleStartCollection = async () => {
    if (!selectedSecret) return;

    try {
      const sessionId = await startRecovery(selectedSecret.id);
      setCurrentGuardianIndex(0);
      setStep("collect-shards");
    } catch (err) {
      setErrorMessage((err as Error).message);
      setStep("error");
    }
  };

  const handleRequestShard = async (guardian: Guardian) => {
    if (!activeSessionId) return;

    try {
      // In a real implementation, this would request the shard from the guardian via P2P
      // For now, we'll simulate receiving a shard
      const shardData = await tauri.requestShardFromGuardian(guardian.id, selectedSecret!.id);

      if (shardData) {
        setCollectedShards((prev) => new Map(prev).set(guardian.id, shardData));
      }
    } catch (err) {
      console.error("Failed to request shard from guardian:", err);
    }
  };

  const handleManualShardSubmit = () => {
    if (!manualShardInput || !manualKeyInput) return;

    try {
      // Parse the manual shard input (base64 encoded)
      const shardBytes = Uint8Array.from(atob(manualShardInput), (c) => c.charCodeAt(0));

      const currentGuardian = selectedGuardianIds[currentGuardianIndex];
      if (currentGuardian) {
        const shard: EncryptedShard = {
          id: `manual-${Date.now()}`,
          secret_id: selectedSecret!.id,
          guardian_id: currentGuardian,
          shard_index: currentGuardianIndex,
          encrypted_data: Array.from(shardBytes),
          nonce: Array(24).fill(0), // Placeholder
          created_at: Math.floor(Date.now() / 1000),
        };

        setCollectedShards((prev) => new Map(prev).set(currentGuardian, shard));
        setShardEncryptionKeys((prev) => new Map(prev).set(currentGuardian, manualKeyInput));
      }

      setManualShardInput("");
      setManualKeyInput("");
      setCurrentGuardianIndex((prev) => prev + 1);
    } catch (err) {
      setErrorMessage("Invalid shard data format");
    }
  };

  const handleSubmitShard = async (guardianId: string) => {
    if (!activeSessionId) return;

    const shard = collectedShards.get(guardianId);
    const key = shardEncryptionKeys.get(guardianId);

    if (!shard || !key) return;

    try {
      await addShard(activeSessionId, shard, key);
    } catch (err) {
      console.error("Failed to add shard:", err);
    }
  };

  const handleCompleteRecovery = async () => {
    if (!activeSessionId) return;

    setStep("recovering");

    try {
      const recoveryResult = await completeRecovery(activeSessionId);

      if (recoveryResult.success && recoveryResult.recovered_data) {
        setStep("complete");
        onComplete?.(new Uint8Array(recoveryResult.recovered_data));
      } else {
        setErrorMessage(recoveryResult.error || "Recovery failed");
        setStep("error");
      }
    } catch (err) {
      setErrorMessage((err as Error).message);
      setStep("error");
    }
  };

  const handleCancel = async () => {
    if (activeSessionId) {
      try {
        await cancelRecovery(activeSessionId);
      } catch (err) {
        console.error("Failed to cancel recovery:", err);
      }
    }
    clearResult();
    clearError();
    onCancel?.();
  };

  const handleRetry = () => {
    clearError();
    setErrorMessage(null);
    setStep("select-secret");
    setSelectedSecret(null);
    setSelectedGuardianIds([]);
    setCollectedShards(new Map());
    setShardEncryptionKeys(new Map());
  };

  const canProceedToCollection =
    selectedSecret &&
    selectedGuardianIds.length >= selectedSecret.shamir_config.threshold;

  const canCompleteRecovery =
    progress && progress.shards_collected >= progress.threshold_required;

  return (
    <div className="flex flex-col h-full bg-gray-900 text-white">
      {/* Header */}
      <div className="p-4 border-b border-gray-800">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Recover Secret</h2>
          <button
            onClick={handleCancel}
            className="px-3 py-1 text-sm text-gray-400 hover:text-white transition"
          >
            Cancel
          </button>
        </div>

        {/* Progress Steps */}
        <div className="flex items-center mt-4 text-sm">
          {["Select Secret", "Select Guardians", "Collect Shards", "Complete"].map(
            (label, index) => {
              const stepMap: WizardStep[] = [
                "select-secret",
                "select-guardians",
                "collect-shards",
                "complete",
              ];
              const isActive = step === stepMap[index];
              const isPast =
                stepMap.indexOf(step) > index ||
                step === "recovering" ||
                step === "complete";

              return (
                <React.Fragment key={label}>
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
                      {isPast && !isActive ? "+" : index + 1}
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
                </React.Fragment>
              );
            }
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {/* Step 1: Select Secret */}
        {step === "select-secret" && (
          <div>
            <h3 className="text-md font-medium mb-4">
              Select a secret to recover
            </h3>
            <div className="space-y-2">
              {secrets
                .filter((s) => s.distribution_complete)
                .map((secret) => (
                  <div
                    key={secret.id}
                    onClick={() => handleSelectSecret(secret)}
                    className="p-4 bg-gray-800 rounded-lg cursor-pointer hover:bg-gray-700 transition"
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <h4 className="font-medium">{secret.name}</h4>
                        <p className="text-sm text-gray-400">
                          {secret.description}
                        </p>
                      </div>
                      <div className="text-right text-sm text-gray-400">
                        <p>
                          Threshold: {secret.shamir_config.threshold}/
                          {secret.shamir_config.total_shares}
                        </p>
                        <p>{secret.guardian_ids.length} guardians</p>
                      </div>
                    </div>
                  </div>
                ))}

              {secrets.filter((s) => s.distribution_complete).length === 0 && (
                <div className="text-center py-8 text-gray-500">
                  <p>No distributed secrets available for recovery</p>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Step 2: Select Guardians */}
        {step === "select-guardians" && selectedSecret && (
          <div>
            <h3 className="text-md font-medium mb-2">
              Select guardians to collect shards from
            </h3>
            <p className="text-sm text-gray-400 mb-4">
              You need at least {selectedSecret.shamir_config.threshold} shards
              to recover this secret. {onlineGuardians.length} guardian(s)
              currently online.
            </p>

            <div className="space-y-2">
              {secretGuardians.map((guardian) => {
                const isSelected = selectedGuardianIds.includes(guardian.id);
                const isOnline = guardian.status === "online";

                return (
                  <div
                    key={guardian.id}
                    onClick={() => handleGuardianToggle(guardian.id)}
                    className={`p-4 rounded-lg cursor-pointer transition ${
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
                          <span className="font-medium">{guardian.name}</span>
                        </div>
                        <p className="text-sm text-gray-400 truncate">
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
            </div>

            <div className="mt-6 flex items-center justify-between">
              <button
                onClick={() => setStep("select-secret")}
                className="px-4 py-2 text-gray-400 hover:text-white transition"
              >
                Back
              </button>
              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-400">
                  {selectedGuardianIds.length} /{" "}
                  {selectedSecret.shamir_config.threshold} required
                </span>
                <button
                  onClick={handleStartCollection}
                  disabled={!canProceedToCollection || loading}
                  className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
                >
                  {loading ? "Starting..." : "Start Collection"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Step 3: Collect Shards */}
        {step === "collect-shards" && selectedSecret && (
          <div>
            <h3 className="text-md font-medium mb-2">Collecting shards</h3>
            <p className="text-sm text-gray-400 mb-4">
              {progress
                ? `${progress.shards_collected} of ${progress.threshold_required} shards collected`
                : "Requesting shards from guardians..."}
            </p>

            {/* Progress Bar */}
            {progress && (
              <div className="mb-6">
                <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-purple-600 transition-all duration-300"
                    style={{
                      width: `${
                        (progress.shards_collected / progress.threshold_required) *
                        100
                      }%`,
                    }}
                  />
                </div>
              </div>
            )}

            {/* Guardian Shard Status */}
            <div className="space-y-3">
              {selectedGuardianIds.map((guardianId) => {
                const guardian = guardians.find((g) => g.id === guardianId);
                const hasShard = collectedShards.has(guardianId);
                const hasKey = shardEncryptionKeys.has(guardianId);
                const isSubmitted = progress?.guardian_responses.some(
                  (r) => r.guardian_id === guardianId && r.received
                );

                return (
                  <div
                    key={guardianId}
                    className="p-4 bg-gray-800 rounded-lg"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <span
                          className={`w-3 h-3 rounded-full ${
                            isSubmitted
                              ? "bg-green-500"
                              : hasShard
                              ? "bg-yellow-500"
                              : "bg-gray-600"
                          }`}
                        />
                        <div>
                          <p className="font-medium">
                            {guardian?.name || guardianId}
                          </p>
                          <p className="text-xs text-gray-500">
                            {isSubmitted
                              ? "Shard received"
                              : hasShard
                              ? "Shard ready to submit"
                              : "Waiting for shard"}
                          </p>
                        </div>
                      </div>

                      <div className="flex items-center gap-2">
                        {!hasShard && guardian && (
                          <button
                            onClick={() => handleRequestShard(guardian)}
                            className="px-3 py-1 text-sm bg-gray-700 text-gray-300 rounded hover:bg-gray-600"
                          >
                            Request
                          </button>
                        )}
                        {hasShard && hasKey && !isSubmitted && (
                          <button
                            onClick={() => handleSubmitShard(guardianId)}
                            disabled={loading}
                            className="px-3 py-1 text-sm bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50"
                          >
                            Submit
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Manual Shard Input */}
            <div className="mt-6 p-4 bg-gray-800 rounded-lg">
              <h4 className="text-sm font-medium mb-3">
                Manual Shard Entry (Offline Guardian)
              </h4>
              <div className="space-y-3">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">
                    Encrypted Shard (Base64)
                  </label>
                  <textarea
                    value={manualShardInput}
                    onChange={(e) => setManualShardInput(e.target.value)}
                    placeholder="Paste encrypted shard data..."
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 text-sm"
                    rows={2}
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">
                    Encryption Key
                  </label>
                  <input
                    type="password"
                    value={manualKeyInput}
                    onChange={(e) => setManualKeyInput(e.target.value)}
                    placeholder="Enter shard encryption key..."
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 text-sm"
                  />
                </div>
                <button
                  onClick={handleManualShardSubmit}
                  disabled={!manualShardInput || !manualKeyInput}
                  className="px-4 py-2 text-sm bg-gray-700 text-gray-300 rounded hover:bg-gray-600 disabled:opacity-50"
                >
                  Add Manual Shard
                </button>
              </div>
            </div>

            {/* Actions */}
            <div className="mt-6 flex items-center justify-between">
              <button
                onClick={() => setStep("select-guardians")}
                className="px-4 py-2 text-gray-400 hover:text-white transition"
              >
                Back
              </button>
              <button
                onClick={handleCompleteRecovery}
                disabled={!canCompleteRecovery || loading}
                className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
              >
                {loading ? "Recovering..." : "Complete Recovery"}
              </button>
            </div>
          </div>
        )}

        {/* Recovering State */}
        {step === "recovering" && (
          <div className="flex flex-col items-center justify-center py-12">
            <div className="w-12 h-12 border-4 border-purple-500 border-t-transparent rounded-full animate-spin mb-4" />
            <h3 className="text-lg font-medium mb-2">Recovering Secret</h3>
            <p className="text-sm text-gray-400">
              Reconstructing secret from collected shards...
            </p>
          </div>
        )}

        {/* Complete State */}
        {step === "complete" && result && (
          <div className="flex flex-col items-center justify-center py-12">
            <div className="w-16 h-16 bg-green-900/30 rounded-full flex items-center justify-center mb-4">
              <span className="text-green-400 text-2xl">+</span>
            </div>
            <h3 className="text-lg font-medium mb-2">Recovery Complete</h3>
            <p className="text-sm text-gray-400 mb-6">
              Secret successfully recovered in {result.recovery_time_ms}ms
            </p>
            <div className="flex gap-3">
              <button
                onClick={handleCancel}
                className="px-4 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700"
              >
                Close
              </button>
              <button
                onClick={() => {
                  // Copy or use the recovered data
                  if (result.recovered_data) {
                    navigator.clipboard.writeText(
                      btoa(String.fromCharCode(...result.recovered_data))
                    );
                  }
                }}
                className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700"
              >
                Copy Secret
              </button>
            </div>
          </div>
        )}

        {/* Error State */}
        {step === "error" && (
          <div className="flex flex-col items-center justify-center py-12">
            <div className="w-16 h-16 bg-red-900/30 rounded-full flex items-center justify-center mb-4">
              <span className="text-red-400 text-2xl">!</span>
            </div>
            <h3 className="text-lg font-medium mb-2">Recovery Failed</h3>
            <p className="text-sm text-red-400 mb-6">
              {errorMessage || "An unknown error occurred"}
            </p>
            <div className="flex gap-3">
              <button
                onClick={handleCancel}
                className="px-4 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700"
              >
                Close
              </button>
              <button
                onClick={handleRetry}
                className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700"
              >
                Try Again
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
