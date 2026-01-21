// WRAITH Chat - Settings Modal Component

import { useState, useEffect } from 'react';
import { useNodeStore } from '../stores/nodeStore';
import { useCallStore } from '../stores/callStore';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type SettingsTab = 'profile' | 'privacy' | 'notifications' | 'appearance' | 'audio-video' | 'security' | 'about';

export default function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const [activeTab, setActiveTab] = useState<SettingsTab>('profile');
  const { status } = useNodeStore();
  const {
    inputDevices,
    outputDevices,
    selectedInputDevice,
    selectedOutputDevice,
    loadAudioDevices,
    setInputDevice,
    setOutputDevice,
  } = useCallStore();

  // Settings state
  const [displayName, setDisplayName] = useState('');
  const [readReceipts, setReadReceipts] = useState(true);
  const [typingIndicators, setTypingIndicators] = useState(true);
  const [notifications, setNotifications] = useState(true);
  const [notificationSound, setNotificationSound] = useState(true);
  const [theme, setTheme] = useState<'dark' | 'light' | 'system'>('dark');
  const [videoQuality, setVideoQuality] = useState<'auto' | 'low' | 'medium' | 'high'>('auto');

  // Load audio devices when audio-video tab is active
  useEffect(() => {
    if (isOpen && activeTab === 'audio-video') {
      loadAudioDevices();
    }
  }, [isOpen, activeTab, loadAudioDevices]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    }
  };

  if (!isOpen) return null;

  const tabs: { id: SettingsTab; label: string; icon: React.ReactNode }[] = [
    { id: 'profile', label: 'Profile', icon: <UserIcon className="w-5 h-5" /> },
    { id: 'privacy', label: 'Privacy', icon: <ShieldIcon className="w-5 h-5" /> },
    { id: 'notifications', label: 'Notifications', icon: <BellIcon className="w-5 h-5" /> },
    { id: 'appearance', label: 'Appearance', icon: <PaletteIcon className="w-5 h-5" /> },
    { id: 'audio-video', label: 'Voice & Video', icon: <VideoIcon className="w-5 h-5" /> },
    { id: 'security', label: 'Security', icon: <LockIcon className="w-5 h-5" /> },
    { id: 'about', label: 'About', icon: <InfoIcon className="w-5 h-5" /> },
  ];

  return (
    <div
      className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      onClick={onClose}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <div
        className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-4xl max-h-[85vh] overflow-hidden flex"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Sidebar */}
        <div className="w-56 bg-bg-primary border-r border-slate-700 flex flex-col">
          <div className="p-4 border-b border-slate-700">
            <h2 id="settings-title" className="text-lg font-semibold text-white">
              Settings
            </h2>
          </div>
          <nav className="flex-1 p-2 space-y-1">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-colors ${
                  activeTab === tab.id
                    ? 'bg-wraith-primary text-white'
                    : 'text-slate-400 hover:text-white hover:bg-bg-tertiary'
                }`}
              >
                {tab.icon}
                <span className="text-sm font-medium">{tab.label}</span>
              </button>
            ))}
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1 flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700">
            <h3 className="text-lg font-semibold text-white">
              {tabs.find((t) => t.id === activeTab)?.label}
            </h3>
            <button
              onClick={onClose}
              className="p-2 text-slate-400 hover:text-white hover:bg-bg-tertiary rounded-lg transition-colors"
              aria-label="Close settings"
            >
              <CloseIcon className="w-5 h-5" />
            </button>
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-y-auto p-6">
            {/* Profile Tab */}
            {activeTab === 'profile' && (
              <div className="space-y-6">
                {/* Avatar */}
                <div className="flex items-center gap-4">
                  <div className="w-20 h-20 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-3xl font-bold text-white">
                    {displayName ? displayName[0].toUpperCase() : 'U'}
                  </div>
                  <button className="px-4 py-2 border border-slate-600 rounded-lg text-slate-300 hover:bg-bg-tertiary transition-colors">
                    Change Avatar
                  </button>
                </div>

                {/* Display Name */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Display Name
                  </label>
                  <input
                    type="text"
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                    placeholder="Enter your display name"
                    className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 text-white placeholder-slate-500 focus:outline-none focus:border-wraith-primary"
                  />
                  <p className="text-xs text-slate-500 mt-1">
                    This name will be shown to your contacts
                  </p>
                </div>

                {/* Peer ID (read-only) */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Your Peer ID
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={status?.local_peer_id || 'Not connected'}
                      readOnly
                      className="flex-1 bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 text-slate-400 font-mono text-sm"
                    />
                    <button
                      onClick={() => status?.local_peer_id && navigator.clipboard.writeText(status.local_peer_id)}
                      className="px-4 py-2 bg-bg-tertiary hover:bg-slate-600 rounded-lg text-white transition-colors"
                    >
                      Copy
                    </button>
                  </div>
                  <p className="text-xs text-slate-500 mt-1">
                    Share this ID with others so they can contact you
                  </p>
                </div>
              </div>
            )}

            {/* Privacy Tab */}
            {activeTab === 'privacy' && (
              <div className="space-y-6">
                <ToggleSetting
                  label="Read Receipts"
                  description="Let others know when you've read their messages"
                  checked={readReceipts}
                  onChange={setReadReceipts}
                />
                <ToggleSetting
                  label="Typing Indicators"
                  description="Show when you're typing a message"
                  checked={typingIndicators}
                  onChange={setTypingIndicators}
                />
                <div className="pt-4 border-t border-slate-700">
                  <h4 className="text-sm font-medium text-slate-300 mb-4">Blocked Contacts</h4>
                  <p className="text-sm text-slate-500">
                    No blocked contacts. Blocked contacts cannot send you messages or call you.
                  </p>
                </div>
              </div>
            )}

            {/* Notifications Tab */}
            {activeTab === 'notifications' && (
              <div className="space-y-6">
                <ToggleSetting
                  label="Enable Notifications"
                  description="Receive notifications for new messages"
                  checked={notifications}
                  onChange={setNotifications}
                />
                <ToggleSetting
                  label="Notification Sounds"
                  description="Play a sound when receiving new messages"
                  checked={notificationSound}
                  onChange={setNotificationSound}
                />
              </div>
            )}

            {/* Appearance Tab */}
            {activeTab === 'appearance' && (
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-3">
                    Theme
                  </label>
                  <div className="flex gap-3">
                    {(['dark', 'light', 'system'] as const).map((t) => (
                      <button
                        key={t}
                        onClick={() => setTheme(t)}
                        className={`flex-1 px-4 py-3 rounded-lg border transition-colors ${
                          theme === t
                            ? 'bg-wraith-primary border-wraith-primary text-white'
                            : 'bg-bg-primary border-slate-600 text-slate-400 hover:border-slate-500'
                        }`}
                      >
                        {t.charAt(0).toUpperCase() + t.slice(1)}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}

            {/* Audio & Video Tab */}
            {activeTab === 'audio-video' && (
              <div className="space-y-6">
                {/* Microphone */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Microphone
                  </label>
                  <select
                    value={selectedInputDevice || ''}
                    onChange={(e) => setInputDevice(e.target.value || null)}
                    className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 text-white focus:outline-none focus:border-wraith-primary"
                  >
                    <option value="">System Default</option>
                    {inputDevices.map((device) => (
                      <option key={device.id} value={device.id}>
                        {device.name} {device.is_default && '(Default)'}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Speaker */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Speaker
                  </label>
                  <select
                    value={selectedOutputDevice || ''}
                    onChange={(e) => setOutputDevice(e.target.value || null)}
                    className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 text-white focus:outline-none focus:border-wraith-primary"
                  >
                    <option value="">System Default</option>
                    {outputDevices.map((device) => (
                      <option key={device.id} value={device.id}>
                        {device.name} {device.is_default && '(Default)'}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Video Quality */}
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Video Quality
                  </label>
                  <select
                    value={videoQuality}
                    onChange={(e) => setVideoQuality(e.target.value as typeof videoQuality)}
                    className="w-full bg-bg-primary border border-slate-600 rounded-lg px-4 py-2.5 text-white focus:outline-none focus:border-wraith-primary"
                  >
                    <option value="auto">Auto (Recommended)</option>
                    <option value="low">Low (360p)</option>
                    <option value="medium">Medium (720p)</option>
                    <option value="high">High (1080p)</option>
                  </select>
                  <p className="text-xs text-slate-500 mt-1">
                    Higher quality uses more bandwidth
                  </p>
                </div>

                {/* Test Buttons */}
                <div className="pt-4 border-t border-slate-700">
                  <div className="flex gap-3">
                    <button className="px-4 py-2 bg-bg-tertiary hover:bg-slate-600 rounded-lg text-white transition-colors">
                      Test Microphone
                    </button>
                    <button className="px-4 py-2 bg-bg-tertiary hover:bg-slate-600 rounded-lg text-white transition-colors">
                      Test Speaker
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Security Tab */}
            {activeTab === 'security' && (
              <div className="space-y-6">
                {/* Encryption Info */}
                <div className="p-4 bg-bg-primary rounded-lg border border-slate-700">
                  <div className="flex items-center gap-3 mb-3">
                    <div className="w-10 h-10 rounded-full bg-green-500/20 flex items-center justify-center">
                      <LockIcon className="w-5 h-5 text-green-500" />
                    </div>
                    <div>
                      <h4 className="font-medium text-white">End-to-End Encrypted</h4>
                      <p className="text-sm text-slate-400">All messages are encrypted</p>
                    </div>
                  </div>
                  <p className="text-sm text-slate-400">
                    WRAITH Chat uses the Signal Protocol with Double Ratchet encryption.
                    Your messages can only be read by you and the intended recipient.
                  </p>
                </div>

                {/* Session Info */}
                <div>
                  <h4 className="text-sm font-medium text-slate-300 mb-3">Active Sessions</h4>
                  <div className="p-4 bg-bg-primary rounded-lg border border-slate-700">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 rounded-lg bg-wraith-primary/20 flex items-center justify-center">
                          <DesktopIcon className="w-5 h-5 text-wraith-primary" />
                        </div>
                        <div>
                          <p className="font-medium text-white">This Device</p>
                          <p className="text-sm text-slate-400">Active now</p>
                        </div>
                      </div>
                      <span className="px-2 py-1 bg-green-500/20 text-green-400 text-xs rounded">
                        Current
                      </span>
                    </div>
                  </div>
                </div>

                {/* Export Keys */}
                <div className="pt-4 border-t border-slate-700">
                  <button className="px-4 py-2 border border-slate-600 rounded-lg text-slate-300 hover:bg-bg-tertiary transition-colors">
                    Export Encryption Keys
                  </button>
                  <p className="text-xs text-slate-500 mt-2">
                    Backup your keys to restore your messages on a new device
                  </p>
                </div>
              </div>
            )}

            {/* About Tab */}
            {activeTab === 'about' && (
              <div className="space-y-6">
                {/* App Info */}
                <div className="text-center py-6">
                  <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center mx-auto mb-4">
                    <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                    </svg>
                  </div>
                  <h3 className="text-xl font-semibold text-white mb-1">WRAITH Chat</h3>
                  <p className="text-sm text-slate-400">Version 1.6.3</p>
                </div>

                {/* Info Grid */}
                <div className="grid grid-cols-2 gap-4">
                  <InfoCard label="Protocol" value="WRAITH Protocol v1.6" />
                  <InfoCard label="Encryption" value="Double Ratchet + Sender Keys" />
                  <InfoCard label="Connection" value={status?.running ? 'Connected' : 'Disconnected'} />
                  <InfoCard label="Sessions" value={String(status?.session_count || 0)} />
                </div>

                {/* Links */}
                <div className="pt-4 border-t border-slate-700 space-y-2">
                  <button className="w-full text-left px-4 py-3 bg-bg-primary rounded-lg text-slate-300 hover:bg-bg-tertiary transition-colors flex items-center justify-between">
                    <span>Documentation</span>
                    <ExternalLinkIcon className="w-4 h-4" />
                  </button>
                  <button className="w-full text-left px-4 py-3 bg-bg-primary rounded-lg text-slate-300 hover:bg-bg-tertiary transition-colors flex items-center justify-between">
                    <span>Report an Issue</span>
                    <ExternalLinkIcon className="w-4 h-4" />
                  </button>
                  <button className="w-full text-left px-4 py-3 bg-bg-primary rounded-lg text-slate-300 hover:bg-bg-tertiary transition-colors flex items-center justify-between">
                    <span>Privacy Policy</span>
                    <ExternalLinkIcon className="w-4 h-4" />
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// Toggle Setting Component
interface ToggleSettingProps {
  label: string;
  description: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

function ToggleSetting({ label, description, checked, onChange }: ToggleSettingProps) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <p className="text-sm font-medium text-slate-300">{label}</p>
        <p className="text-xs text-slate-500 mt-0.5">{description}</p>
      </div>
      <button
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
          checked ? 'bg-wraith-primary' : 'bg-slate-600'
        }`}
        role="switch"
        aria-checked={checked}
      >
        <span
          className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
            checked ? 'translate-x-6' : 'translate-x-1'
          }`}
        />
      </button>
    </div>
  );
}

// Info Card Component
function InfoCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-4 bg-bg-primary rounded-lg border border-slate-700">
      <p className="text-xs text-slate-500 mb-1">{label}</p>
      <p className="text-sm font-medium text-white">{value}</p>
    </div>
  );
}

// Icons
function UserIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
    </svg>
  );
}

function ShieldIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
    </svg>
  );
}

function BellIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
    </svg>
  );
}

function PaletteIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
    </svg>
  );
}

function VideoIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
    </svg>
  );
}

function LockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
    </svg>
  );
}

function InfoIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function CloseIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function DesktopIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
    </svg>
  );
}

function ExternalLinkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
    </svg>
  );
}
