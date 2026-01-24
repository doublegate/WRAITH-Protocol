// WRAITH Vault - Main Application Component
// Distributed Secret Storage with Shamir's Secret Sharing

import { useState, useEffect } from "react";
import { SecretList } from "./components/SecretList";
import { GuardianList } from "./components/GuardianList";
import { RecoveryWizard } from "./components/RecoveryWizard";
import { ShardStatus } from "./components/ShardStatus";
import { CreateSecretModal } from "./components/CreateSecretModal";
import { AddGuardianModal } from "./components/AddGuardianModal";
import { SecretDetail } from "./components/SecretDetail";
import { useSecretStore } from "./stores/secretStore";
import { useGuardianStore } from "./stores/guardianStore";
import { useNodeStore } from "./stores/nodeStore";
import type { SecretInfo, Guardian } from "./types";

type View = "secrets" | "guardians" | "recovery" | "settings";
type Modal = "create-secret" | "add-guardian" | null;

export default function App() {
  const [currentView, setCurrentView] = useState<View>("secrets");
  const [activeModal, setActiveModal] = useState<Modal>(null);
  const [selectedSecretForRecovery, setSelectedSecretForRecovery] = useState<string | null>(null);
  const [showSecretDetail, setShowSecretDetail] = useState(false);

  const { selectedSecret, selectSecret } = useSecretStore();
  const { guardians } = useGuardianStore();
  const { status, vaultStats, loadStatus, loadVaultStats, startNode, stopNode } = useNodeStore();

  useEffect(() => {
    loadStatus();
    loadVaultStats();

    // Poll for status updates
    const interval = setInterval(() => {
      loadStatus();
      loadVaultStats();
    }, 30000);

    return () => clearInterval(interval);
  }, [loadStatus, loadVaultStats]);

  const handleSecretSelect = (secret: SecretInfo) => {
    selectSecret(secret);
    setShowSecretDetail(true);
  };

  const handleGuardianSelect = (guardian: Guardian) => {
    // Could open a guardian detail view
    console.log("Selected guardian:", guardian);
  };

  const handleStartRecovery = (secretId: string) => {
    setSelectedSecretForRecovery(secretId);
    setCurrentView("recovery");
  };

  const handleRecoveryComplete = (data: Uint8Array) => {
    console.log("Secret recovered:", data.length, "bytes");
    setCurrentView("secrets");
    setSelectedSecretForRecovery(null);
  };

  const handleRecoveryCancel = () => {
    setCurrentView("secrets");
    setSelectedSecretForRecovery(null);
  };

  const handleToggleNode = async () => {
    try {
      if (status.running) {
        await stopNode();
      } else {
        await startNode();
      }
    } catch (err) {
      console.error("Failed to toggle node:", err);
    }
  };

  return (
    <div className="flex h-screen bg-gray-950 text-white">
      {/* Sidebar */}
      <aside className="w-64 bg-gray-900 border-r border-gray-800 flex flex-col">
        {/* Logo */}
        <div className="p-4 border-b border-gray-800">
          <h1 className="text-xl font-bold text-purple-400">WRAITH Vault</h1>
          <p className="text-xs text-gray-500 mt-1">Distributed Secret Storage</p>
        </div>

        {/* Navigation */}
        <nav className="flex-1 p-4">
          <ul className="space-y-2">
            <li>
              <button
                onClick={() => setCurrentView("secrets")}
                className={`w-full text-left px-4 py-2 rounded-lg transition ${
                  currentView === "secrets"
                    ? "bg-purple-600 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`}
              >
                Secrets
              </button>
            </li>
            <li>
              <button
                onClick={() => setCurrentView("guardians")}
                className={`w-full text-left px-4 py-2 rounded-lg transition ${
                  currentView === "guardians"
                    ? "bg-purple-600 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`}
              >
                Guardians
              </button>
            </li>
            <li>
              <button
                onClick={() => setCurrentView("recovery")}
                className={`w-full text-left px-4 py-2 rounded-lg transition ${
                  currentView === "recovery"
                    ? "bg-purple-600 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`}
              >
                Recovery
              </button>
            </li>
            <li>
              <button
                onClick={() => setCurrentView("settings")}
                className={`w-full text-left px-4 py-2 rounded-lg transition ${
                  currentView === "settings"
                    ? "bg-purple-600 text-white"
                    : "text-gray-400 hover:bg-gray-800 hover:text-white"
                }`}
              >
                Settings
              </button>
            </li>
          </ul>
        </nav>

        {/* Stats */}
        {vaultStats && (
          <div className="p-4 border-t border-gray-800">
            <div className="grid grid-cols-2 gap-2 text-center text-xs">
              <div className="bg-gray-800 rounded p-2">
                <p className="text-lg font-bold text-purple-400">
                  {vaultStats.secret_count}
                </p>
                <p className="text-gray-500">Secrets</p>
              </div>
              <div className="bg-gray-800 rounded p-2">
                <p className="text-lg font-bold text-green-400">
                  {vaultStats.online_guardians}
                </p>
                <p className="text-gray-500">Online</p>
              </div>
            </div>
          </div>
        )}

        {/* Node Status */}
        <div className="p-4 border-t border-gray-800">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span
                className={`w-2 h-2 rounded-full ${
                  status.running ? "bg-green-500" : "bg-gray-500"
                }`}
              />
              <span className="text-sm text-gray-400">
                {status.running ? "Connected" : "Offline"}
              </span>
            </div>
            <button
              onClick={handleToggleNode}
              className={`px-2 py-1 text-xs rounded ${
                status.running
                  ? "bg-red-900/30 text-red-400 hover:bg-red-900/50"
                  : "bg-green-900/30 text-green-400 hover:bg-green-900/50"
              }`}
            >
              {status.running ? "Stop" : "Start"}
            </button>
          </div>
          {status.peer_id && (
            <p className="text-xs text-gray-600 truncate mt-1" title={status.peer_id}>
              {status.peer_id}
            </p>
          )}
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex overflow-hidden">
        {/* List Panel */}
        <div className="w-96 border-r border-gray-800 flex flex-col">
          {currentView === "secrets" && (
            <SecretList
              onSecretSelect={handleSecretSelect}
              onCreateNew={() => setActiveModal("create-secret")}
            />
          )}

          {currentView === "guardians" && (
            <GuardianList
              onGuardianSelect={handleGuardianSelect}
              onAddNew={() => setActiveModal("add-guardian")}
            />
          )}

          {currentView === "recovery" && (
            <RecoveryWizard
              preselectedSecretId={selectedSecretForRecovery || undefined}
              onComplete={handleRecoveryComplete}
              onCancel={handleRecoveryCancel}
            />
          )}

          {currentView === "settings" && (
            <div className="p-4">
              <h2 className="text-lg font-semibold mb-4">Settings</h2>
              <div className="space-y-4">
                <div className="bg-gray-800 rounded-lg p-4">
                  <h3 className="text-sm font-medium mb-2">Node Configuration</h3>
                  <p className="text-xs text-gray-500">
                    Configure WRAITH protocol settings
                  </p>
                </div>
                <div className="bg-gray-800 rounded-lg p-4">
                  <h3 className="text-sm font-medium mb-2">Security</h3>
                  <p className="text-xs text-gray-500">
                    Manage encryption and authentication settings
                  </p>
                </div>
                <div className="bg-gray-800 rounded-lg p-4">
                  <h3 className="text-sm font-medium mb-2">Backup</h3>
                  <p className="text-xs text-gray-500">
                    Export and import vault data
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Detail Panel */}
        <div className="flex-1 bg-gray-950 overflow-y-auto">
          {currentView === "secrets" && selectedSecret && showSecretDetail && (
            <div className="p-6">
              <SecretDetail
                secret={selectedSecret}
                onRecover={() => handleStartRecovery(selectedSecret.id)}
                onClose={() => setShowSecretDetail(false)}
              />
              <div className="mt-6">
                <ShardStatus
                  secret={selectedSecret}
                  guardians={guardians.filter((g) =>
                    selectedSecret.guardian_ids.includes(g.id)
                  )}
                  onRefresh={() => {}}
                />
              </div>
            </div>
          )}

          {currentView === "secrets" && !selectedSecret && (
            <div className="flex items-center justify-center h-full text-gray-500">
              <div className="text-center">
                <p className="text-lg mb-2">Select a secret to view details</p>
                <p className="text-sm">
                  Or create a new secret to get started
                </p>
              </div>
            </div>
          )}

          {currentView === "guardians" && (
            <div className="flex items-center justify-center h-full text-gray-500">
              <div className="text-center">
                <p className="text-lg mb-2">Guardian Management</p>
                <p className="text-sm">
                  Select a guardian to view details and manage trust settings
                </p>
              </div>
            </div>
          )}

          {currentView === "settings" && (
            <div className="p-6">
              <h2 className="text-xl font-semibold mb-6">Vault Settings</h2>

              <div className="space-y-6">
                {/* Node Settings */}
                <section className="bg-gray-900 rounded-lg p-6">
                  <h3 className="text-lg font-medium mb-4">Network</h3>
                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm text-gray-400 mb-1">
                        Listen Address
                      </label>
                      <input
                        type="text"
                        defaultValue="0.0.0.0:9090"
                        className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white"
                      />
                    </div>
                    <div>
                      <label className="block text-sm text-gray-400 mb-1">
                        Bootstrap Nodes
                      </label>
                      <textarea
                        defaultValue=""
                        placeholder="Enter bootstrap node addresses..."
                        className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white"
                        rows={3}
                      />
                    </div>
                  </div>
                </section>

                {/* Security Settings */}
                <section className="bg-gray-900 rounded-lg p-6">
                  <h3 className="text-lg font-medium mb-4">Security</h3>
                  <div className="space-y-4">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">Auto-lock timeout</p>
                        <p className="text-xs text-gray-500">
                          Lock vault after inactivity
                        </p>
                      </div>
                      <select className="px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white">
                        <option value="5">5 minutes</option>
                        <option value="15">15 minutes</option>
                        <option value="30">30 minutes</option>
                        <option value="60">1 hour</option>
                        <option value="0">Never</option>
                      </select>
                    </div>
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">Require verification</p>
                        <p className="text-xs text-gray-500">
                          Verify shards periodically
                        </p>
                      </div>
                      <input
                        type="checkbox"
                        defaultChecked
                        className="h-4 w-4 rounded border-gray-600 bg-gray-800 text-purple-600"
                      />
                    </div>
                  </div>
                </section>

                {/* Default Shamir Settings */}
                <section className="bg-gray-900 rounded-lg p-6">
                  <h3 className="text-lg font-medium mb-4">Default Sharing</h3>
                  <div className="space-y-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <label className="block text-sm text-gray-400 mb-1">
                          Default Threshold (k)
                        </label>
                        <input
                          type="number"
                          defaultValue={3}
                          min={2}
                          max={10}
                          className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white"
                        />
                      </div>
                      <div>
                        <label className="block text-sm text-gray-400 mb-1">
                          Default Total Shares (n)
                        </label>
                        <input
                          type="number"
                          defaultValue={5}
                          min={2}
                          max={20}
                          className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white"
                        />
                      </div>
                    </div>
                    <p className="text-xs text-gray-500">
                      With 3-of-5 sharing, you need at least 3 guardians to
                      recover your secret.
                    </p>
                  </div>
                </section>

                {/* Danger Zone */}
                <section className="bg-red-900/20 border border-red-800 rounded-lg p-6">
                  <h3 className="text-lg font-medium text-red-400 mb-4">
                    Danger Zone
                  </h3>
                  <div className="space-y-4">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">Export Vault</p>
                        <p className="text-xs text-gray-500">
                          Export encrypted backup
                        </p>
                      </div>
                      <button className="px-4 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700">
                        Export
                      </button>
                    </div>
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium text-red-400">
                          Clear All Data
                        </p>
                        <p className="text-xs text-gray-500">
                          Permanently delete all secrets
                        </p>
                      </div>
                      <button className="px-4 py-2 bg-red-900/50 text-red-400 rounded hover:bg-red-900">
                        Clear
                      </button>
                    </div>
                  </div>
                </section>
              </div>
            </div>
          )}
        </div>
      </main>

      {/* Modals */}
      {activeModal === "create-secret" && (
        <CreateSecretModal
          onClose={() => setActiveModal(null)}
          onCreated={() => setActiveModal(null)}
        />
      )}

      {activeModal === "add-guardian" && (
        <AddGuardianModal
          onClose={() => setActiveModal(null)}
          onAdded={() => setActiveModal(null)}
        />
      )}
    </div>
  );
}
