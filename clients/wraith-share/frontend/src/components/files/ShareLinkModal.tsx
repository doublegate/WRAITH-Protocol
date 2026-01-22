// ShareLinkModal Component - Create and manage share links

import { useState, useEffect } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import Modal from '../ui/Modal';
import Input from '../ui/Input';
import Button from '../ui/Button';
import { useFileStore } from '../../stores/fileStore';
import { useUiStore } from '../../stores/uiStore';
import { formatRelativeTime } from '../../types';

export default function ShareLinkModal() {
  const { activeModal, modalData, closeModal, addToast } = useUiStore();
  const { files, shareLinks, fetchShareLinks, createShareLink, revokeShareLink } =
    useFileStore();

  const [expiresInHours, setExpiresInHours] = useState<string>('24');
  const [password, setPassword] = useState('');
  const [maxDownloads, setMaxDownloads] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const isOpen = activeModal === 'shareLink';
  const fileId = modalData as string;
  const file = files.find((f) => f.id === fileId);

  useEffect(() => {
    if (isOpen && fileId) {
      fetchShareLinks(fileId);
    }
  }, [isOpen, fileId, fetchShareLinks]);

  const handleClose = () => {
    setShowCreateForm(false);
    setExpiresInHours('24');
    setPassword('');
    setMaxDownloads('');
    closeModal();
  };

  const handleCreate = async () => {
    if (!fileId) return;

    setLoading(true);
    try {
      await createShareLink(
        fileId,
        expiresInHours ? parseInt(expiresInHours) : undefined,
        password || undefined,
        maxDownloads ? parseInt(maxDownloads) : undefined
      );
      addToast('success', 'Share link created');
      setShowCreateForm(false);
      setPassword('');
      setMaxDownloads('');
    } catch (err) {
      addToast('error', (err as Error).message);
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async (linkId: string) => {
    if (!confirm('Revoke this share link? It will no longer be accessible.')) return;

    try {
      await revokeShareLink(linkId);
      addToast('success', 'Share link revoked');
    } catch (err) {
      addToast('error', (err as Error).message);
    }
  };

  const handleCopyLink = async (linkId: string) => {
    // In a real app, this would construct the full share URL
    const shareUrl = `wraith://share/${linkId}`;
    await navigator.clipboard.writeText(shareUrl);
    setCopiedId(linkId);
    addToast('success', 'Link copied to clipboard');
    setTimeout(() => setCopiedId(null), 2000);
  };

  const activeLinks = shareLinks.filter((l) => l.is_active);

  const expirationOptions = [
    { value: '1', label: '1 hour' },
    { value: '24', label: '24 hours' },
    { value: '168', label: '7 days' },
    { value: '720', label: '30 days' },
    { value: '', label: 'Never' },
  ];

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      title={`Share "${file?.name || 'File'}"`}
      size="lg"
    >
      <div className="space-y-4">
        {/* Existing links */}
        {activeLinks.length > 0 && !showCreateForm && (
          <div className="space-y-3">
            <h4 className="text-sm font-medium text-slate-300">Active Share Links</h4>
            {activeLinks.map((link) => (
              <div
                key={link.id}
                className="p-3 bg-slate-800 rounded-lg"
              >
                <div className="flex items-start gap-3">
                  {/* QR Code */}
                  <div className="flex-shrink-0 p-1 bg-white rounded">
                    <QRCodeSVG value={`wraith://share/${link.id}`} size={64} />
                  </div>

                  {/* Link info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-white truncate">
                        {link.id.slice(0, 16)}...
                      </span>
                      {link.has_password && (
                        <span className="px-1.5 py-0.5 bg-amber-500/20 text-amber-400 text-xs rounded">
                          Password
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-slate-400 mt-1">
                      {link.download_count} downloads
                      {link.max_downloads && ` / ${link.max_downloads} max`}
                    </p>
                    <p className="text-xs text-slate-500">
                      {link.expires_at
                        ? `Expires ${formatRelativeTime(link.expires_at)}`
                        : 'Never expires'}
                    </p>
                  </div>

                  {/* Actions */}
                  <div className="flex gap-1">
                    <button
                      onClick={() => handleCopyLink(link.id)}
                      className={`p-1.5 rounded transition-colors ${
                        copiedId === link.id
                          ? 'bg-green-500 text-white'
                          : 'text-slate-400 hover:text-white hover:bg-slate-700'
                      }`}
                      aria-label="Copy link"
                    >
                      {copiedId === link.id ? (
                        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                          <path
                            fillRule="evenodd"
                            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                            clipRule="evenodd"
                          />
                        </svg>
                      ) : (
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
                          />
                        </svg>
                      )}
                    </button>
                    <button
                      onClick={() => handleRevoke(link.id)}
                      className="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-400/10 rounded transition-colors"
                      aria-label="Revoke link"
                    >
                      <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                        <path
                          fillRule="evenodd"
                          d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                          clipRule="evenodd"
                        />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Create new link form */}
        {showCreateForm ? (
          <div className="space-y-4">
            <h4 className="text-sm font-medium text-slate-300">Create New Share Link</h4>

            <div>
              <label className="block text-sm font-medium text-slate-300 mb-2">
                Expiration
              </label>
              <select
                value={expiresInHours}
                onChange={(e) => setExpiresInHours(e.target.value)}
                className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white"
              >
                {expirationOptions.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>

            <Input
              label="Password (optional)"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Leave empty for no password"
            />

            <Input
              label="Max Downloads (optional)"
              type="number"
              value={maxDownloads}
              onChange={(e) => setMaxDownloads(e.target.value)}
              placeholder="Leave empty for unlimited"
              min="1"
            />

            <div className="flex justify-end gap-3">
              <Button variant="ghost" onClick={() => setShowCreateForm(false)}>
                Cancel
              </Button>
              <Button onClick={handleCreate} loading={loading}>
                Create Link
              </Button>
            </div>
          </div>
        ) : (
          <Button onClick={() => setShowCreateForm(true)} className="w-full">
            <span className="flex items-center justify-center gap-2">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 4v16m8-8H4"
                />
              </svg>
              Create New Share Link
            </span>
          </Button>
        )}
      </div>
    </Modal>
  );
}
