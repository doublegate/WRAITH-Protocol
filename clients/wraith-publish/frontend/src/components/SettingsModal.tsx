import { useState, useEffect } from 'react';
import { useUIStore } from '../stores/uiStore';
import { usePropagationStore } from '../stores/propagationStore';
import * as api from '../lib/tauri';

interface SettingsModalProps {
  onClose: () => void;
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const { peerId, displayName, setDisplayName, showNotification } = useUIStore();
  const { storageStats, pinnedCids, fetchStorageStats, fetchPinnedCids } = usePropagationStore();
  const [activeTab, setActiveTab] = useState<'profile' | 'network' | 'storage' | 'about'>('profile');
  const [newDisplayName, setNewDisplayName] = useState(displayName);
  const [isSaving, setIsSaving] = useState(false);

  // Fetch data on mount
  useEffect(() => {
    fetchStorageStats();
    fetchPinnedCids();
  }, [fetchStorageStats, fetchPinnedCids]);

  const handleSaveProfile = async () => {
    if (!newDisplayName.trim()) return;

    setIsSaving(true);
    try {
      await api.setDisplayName(newDisplayName.trim());
      setDisplayName(newDisplayName.trim());
      showNotification({ type: 'success', message: 'Profile updated' });
    } catch (error) {
      showNotification({ type: 'error', message: 'Failed to update profile' });
    } finally {
      setIsSaving(false);
    }
  };

  const handleCopyPeerId = async () => {
    if (!peerId) return;

    try {
      await navigator.clipboard.writeText(peerId);
      showNotification({ type: 'success', message: 'Peer ID copied to clipboard' });
    } catch (error) {
      showNotification({ type: 'error', message: 'Failed to copy Peer ID' });
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-bg-secondary border border-slate-700 rounded-xl shadow-xl w-full max-w-2xl mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-700 flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">Settings</h2>
          <button
            onClick={onClose}
            className="p-1 text-slate-400 hover:text-white transition-colors"
          >
            <CloseIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex flex-1 min-h-0">
          {/* Sidebar */}
          <div className="w-48 border-r border-slate-700 p-2">
            <nav className="space-y-1">
              <TabButton
                label="Profile"
                icon={<UserIcon className="w-4 h-4" />}
                active={activeTab === 'profile'}
                onClick={() => setActiveTab('profile')}
              />
              <TabButton
                label="Network"
                icon={<NetworkIcon className="w-4 h-4" />}
                active={activeTab === 'network'}
                onClick={() => setActiveTab('network')}
              />
              <TabButton
                label="Storage"
                icon={<StorageIcon className="w-4 h-4" />}
                active={activeTab === 'storage'}
                onClick={() => setActiveTab('storage')}
              />
              <TabButton
                label="About"
                icon={<InfoIcon className="w-4 h-4" />}
                active={activeTab === 'about'}
                onClick={() => setActiveTab('about')}
              />
            </nav>
          </div>

          {/* Main content */}
          <div className="flex-1 p-6 overflow-y-auto">
            {activeTab === 'profile' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-medium text-white mb-4">Profile</h3>

                  {/* Display name */}
                  <div className="space-y-2">
                    <label className="block text-sm font-medium text-slate-300">
                      Display Name
                    </label>
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        value={newDisplayName}
                        onChange={(e) => setNewDisplayName(e.target.value)}
                        className="flex-1 bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                        placeholder="Enter your display name"
                      />
                      <button
                        onClick={handleSaveProfile}
                        disabled={isSaving || newDisplayName.trim() === displayName}
                        className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary text-white font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isSaving ? 'Saving...' : 'Save'}
                      </button>
                    </div>
                    <p className="text-xs text-slate-500">
                      This name will be shown as the author of your published articles.
                    </p>
                  </div>
                </div>

                {/* Peer ID */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Peer ID
                  </label>
                  <div className="flex items-center gap-2">
                    <code className="flex-1 bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-xs text-slate-400 font-mono truncate">
                      {peerId || 'Not connected'}
                    </code>
                    <button
                      onClick={handleCopyPeerId}
                      disabled={!peerId}
                      className="p-2 text-slate-400 hover:text-white transition-colors disabled:opacity-50"
                      title="Copy Peer ID"
                    >
                      <CopyIcon className="w-4 h-4" />
                    </button>
                  </div>
                  <p className="text-xs text-slate-500 mt-1">
                    Your unique identifier on the WRAITH network.
                  </p>
                </div>
              </div>
            )}

            {activeTab === 'network' && (
              <div className="space-y-6">
                <h3 className="text-lg font-medium text-white mb-4">Network Status</h3>

                <div className="grid grid-cols-2 gap-4">
                  <StatCard
                    label="Connection Status"
                    value={peerId ? 'Connected' : 'Disconnected'}
                    icon={<StatusDotIcon connected={!!peerId} />}
                  />
                  <StatCard
                    label="Pinned Content"
                    value={pinnedCids.length.toString()}
                    icon={<PinIcon className="w-5 h-5 text-wraith-primary" />}
                  />
                </div>

                <div className="bg-bg-primary rounded-lg p-4">
                  <h4 className="text-sm font-medium text-slate-300 mb-3">
                    Network Information
                  </h4>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-slate-400">Protocol</span>
                      <span className="text-white">WRAITH DHT</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-400">Replication Factor</span>
                      <span className="text-white">3x</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-400">Content Addressing</span>
                      <span className="text-white">BLAKE3</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-400">Signing Algorithm</span>
                      <span className="text-white">Ed25519</span>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'storage' && (
              <div className="space-y-6">
                <h3 className="text-lg font-medium text-white mb-4">Storage</h3>

                {storageStats ? (
                  <>
                    <div className="grid grid-cols-2 gap-4">
                      <StatCard
                        label="Total Size"
                        value={formatBytes(storageStats.total_size)}
                        icon={<StorageIcon className="w-5 h-5 text-wraith-primary" />}
                      />
                      <StatCard
                        label="Items Stored"
                        value={storageStats.item_count.toString()}
                        icon={<DocumentIcon className="w-5 h-5 text-wraith-primary" />}
                      />
                    </div>

                    <div className="bg-bg-primary rounded-lg p-4">
                      <h4 className="text-sm font-medium text-slate-300 mb-3">
                        Storage Breakdown
                      </h4>
                      <div className="space-y-3">
                        <div>
                          <div className="flex justify-between text-sm mb-1">
                            <span className="text-slate-400">Articles</span>
                            <span className="text-white">
                              {formatBytes(storageStats.articles_size || 0)}
                            </span>
                          </div>
                          <div className="h-2 bg-bg-tertiary rounded-full overflow-hidden">
                            <div
                              className="h-full bg-wraith-primary"
                              style={{
                                width: `${
                                  ((storageStats.articles_size || 0) / storageStats.total_size) * 100
                                }%`,
                              }}
                            />
                          </div>
                        </div>
                        <div>
                          <div className="flex justify-between text-sm mb-1">
                            <span className="text-slate-400">Images</span>
                            <span className="text-white">
                              {formatBytes(storageStats.images_size || 0)}
                            </span>
                          </div>
                          <div className="h-2 bg-bg-tertiary rounded-full overflow-hidden">
                            <div
                              className="h-full bg-cyan-500"
                              style={{
                                width: `${
                                  ((storageStats.images_size || 0) / storageStats.total_size) * 100
                                }%`,
                              }}
                            />
                          </div>
                        </div>
                        <div>
                          <div className="flex justify-between text-sm mb-1">
                            <span className="text-slate-400">Cache</span>
                            <span className="text-white">
                              {formatBytes(storageStats.cache_size || 0)}
                            </span>
                          </div>
                          <div className="h-2 bg-bg-tertiary rounded-full overflow-hidden">
                            <div
                              className="h-full bg-slate-500"
                              style={{
                                width: `${
                                  ((storageStats.cache_size || 0) / storageStats.total_size) * 100
                                }%`,
                              }}
                            />
                          </div>
                        </div>
                      </div>
                    </div>
                  </>
                ) : (
                  <div className="text-center text-slate-500 py-8">
                    <StorageIcon className="w-12 h-12 mx-auto mb-2 opacity-50" />
                    <p>Loading storage stats...</p>
                  </div>
                )}
              </div>
            )}

            {activeTab === 'about' && (
              <div className="space-y-6">
                <div className="text-center py-4">
                  <div className="w-16 h-16 bg-wraith-primary/20 rounded-xl flex items-center justify-center mx-auto mb-4">
                    <PublishIcon className="w-8 h-8 text-wraith-primary" />
                  </div>
                  <h3 className="text-xl font-bold text-white">WRAITH Publish</h3>
                  <p className="text-slate-400 mt-1">Version 1.0.0</p>
                </div>

                <div className="bg-bg-primary rounded-lg p-4">
                  <p className="text-sm text-slate-400 text-center">
                    A censorship-resistant publishing platform built on the WRAITH Protocol.
                    Create, sign, and distribute content across a decentralized network with
                    cryptographic verification.
                  </p>
                </div>

                <div className="bg-bg-primary rounded-lg p-4">
                  <h4 className="text-sm font-medium text-slate-300 mb-3">Features</h4>
                  <ul className="text-sm text-slate-400 space-y-2">
                    <li className="flex items-start gap-2">
                      <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                      IPFS-like content addressing (BLAKE3 CIDs)
                    </li>
                    <li className="flex items-start gap-2">
                      <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                      Ed25519 digital signatures
                    </li>
                    <li className="flex items-start gap-2">
                      <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                      DHT-based distributed storage
                    </li>
                    <li className="flex items-start gap-2">
                      <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                      3x replication for resilience
                    </li>
                    <li className="flex items-start gap-2">
                      <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                      RSS feed generation
                    </li>
                  </ul>
                </div>

                <div className="text-center text-xs text-slate-500">
                  <p>Part of the WRAITH Protocol suite</p>
                  <p className="mt-1">Built with Tauri, React, and Rust</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// Tab button component
interface TabButtonProps {
  label: string;
  icon: React.ReactNode;
  active: boolean;
  onClick: () => void;
}

function TabButton({ label, icon, active, onClick }: TabButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
        active
          ? 'bg-wraith-primary text-white'
          : 'text-slate-400 hover:text-white hover:bg-bg-primary'
      }`}
    >
      {icon}
      {label}
    </button>
  );
}

// Stat card component
interface StatCardProps {
  label: string;
  value: string;
  icon: React.ReactNode;
}

function StatCard({ label, value, icon }: StatCardProps) {
  return (
    <div className="bg-bg-primary rounded-lg p-4">
      <div className="flex items-center gap-3">
        {icon}
        <div>
          <p className="text-xs text-slate-500">{label}</p>
          <p className="text-lg font-semibold text-white">{value}</p>
        </div>
      </div>
    </div>
  );
}

// Status dot component
function StatusDotIcon({ connected }: { connected: boolean }) {
  return (
    <span
      className={`w-3 h-3 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'}`}
    />
  );
}

// Icons
function CloseIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M6 18L18 6M6 6l12 12"
      />
    </svg>
  );
}

function UserIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
      />
    </svg>
  );
}

function NetworkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"
      />
    </svg>
  );
}

function StorageIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"
      />
    </svg>
  );
}

function InfoIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

function CopyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
      />
    </svg>
  );
}

function PinIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
      />
    </svg>
  );
}

function DocumentIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
      />
    </svg>
  );
}

function PublishIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
      />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M5 13l4 4L19 7"
      />
    </svg>
  );
}
