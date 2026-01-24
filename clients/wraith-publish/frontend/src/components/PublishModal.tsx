import { useState, useEffect } from 'react';
import { useContentStore } from '../stores/contentStore';
import { usePropagationStore } from '../stores/propagationStore';
import { useUIStore } from '../stores/uiStore';
import type { Article, PropagationStatus } from '../types';

interface PublishModalProps {
  article: Article;
  onClose: () => void;
}

export function PublishModal({ article, onClose }: PublishModalProps) {
  const { publishArticle, unpublishArticle, fetchArticles } = useContentStore();
  const { getPropagationStatus } = usePropagationStore();
  const { showNotification } = useUIStore();
  const [step, setStep] = useState<'confirm' | 'publishing' | 'success' | 'error'>('confirm');
  const [publishedCid, setPublishedCid] = useState<string | null>(null);
  const [propagation, setPropagation] = useState<PropagationStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isPublished = article.status === 'published';

  // Poll propagation status after publishing
  useEffect(() => {
    if (step !== 'success' || !publishedCid) return;

    const fetchPropagation = async () => {
      const status = await getPropagationStatus(publishedCid);
      if (status) {
        setPropagation(status);
      }
    };

    fetchPropagation();
    const interval = setInterval(fetchPropagation, 2000);
    return () => clearInterval(interval);
  }, [step, publishedCid, getPropagationStatus]);

  const handlePublish = async () => {
    setStep('publishing');
    setError(null);

    try {
      const cid = await publishArticle(article.id);
      setPublishedCid(cid);
      setStep('success');
      showNotification({ type: 'success', message: 'Article published successfully!' });
      await fetchArticles();
    } catch (e) {
      setError(String(e));
      setStep('error');
      showNotification({ type: 'error', message: 'Failed to publish article' });
    }
  };

  const handleUnpublish = async () => {
    setStep('publishing');
    setError(null);

    try {
      await unpublishArticle(article.id);
      showNotification({ type: 'info', message: 'Article unpublished' });
      await fetchArticles();
      onClose();
    } catch (e) {
      setError(String(e));
      setStep('error');
      showNotification({ type: 'error', message: 'Failed to unpublish article' });
    }
  };

  const handleCopyCid = async () => {
    if (!publishedCid) return;

    try {
      await navigator.clipboard.writeText(publishedCid);
      showNotification({ type: 'success', message: 'CID copied to clipboard' });
    } catch {
      showNotification({ type: 'error', message: 'Failed to copy CID' });
    }
  };

  const handleCopyLink = async () => {
    if (!publishedCid) return;

    const link = `wraith://publish/${publishedCid}`;
    try {
      await navigator.clipboard.writeText(link);
      showNotification({ type: 'success', message: 'Link copied to clipboard' });
    } catch {
      showNotification({ type: 'error', message: 'Failed to copy link' });
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-bg-secondary border border-slate-700 rounded-xl shadow-xl w-full max-w-lg mx-4">
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-700 flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">
            {isPublished ? 'Manage Publication' : 'Publish Article'}
          </h2>
          <button
            onClick={onClose}
            className="p-1 text-slate-400 hover:text-white transition-colors"
          >
            <CloseIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6">
          {step === 'confirm' && (
            <>
              {isPublished ? (
                // Unpublish confirmation
                <div className="space-y-4">
                  <div className="flex items-start gap-3">
                    <WarningIcon className="w-6 h-6 text-yellow-500 flex-shrink-0 mt-0.5" />
                    <div>
                      <h3 className="font-medium text-white">Unpublish this article?</h3>
                      <p className="text-sm text-slate-400 mt-1">
                        The article will be removed from the WRAITH network. Existing copies
                        may still be accessible to peers who have cached the content.
                      </p>
                    </div>
                  </div>

                  <div className="bg-bg-primary rounded-lg p-4">
                    <h4 className="text-sm font-medium text-slate-300 mb-2">Current CID</h4>
                    <code className="text-xs text-slate-500 break-all font-mono">
                      {article.cid}
                    </code>
                  </div>
                </div>
              ) : (
                // Publish confirmation
                <div className="space-y-4">
                  <div className="flex items-start gap-3">
                    <PublishIcon className="w-6 h-6 text-wraith-primary flex-shrink-0 mt-0.5" />
                    <div>
                      <h3 className="font-medium text-white">Ready to publish?</h3>
                      <p className="text-sm text-slate-400 mt-1">
                        Your article will be cryptographically signed and distributed across
                        the WRAITH network. This makes it censorship-resistant and verifiable.
                      </p>
                    </div>
                  </div>

                  <div className="bg-bg-primary rounded-lg p-4 space-y-3">
                    <div>
                      <h4 className="text-sm font-medium text-slate-300">{article.title}</h4>
                      <p className="text-xs text-slate-500 mt-1">
                        {article.content.split(/\s+/).length} words
                        {article.tags.length > 0 && (
                          <span className="ml-2">
                            Tags: {article.tags.join(', ')}
                          </span>
                        )}
                      </p>
                    </div>
                  </div>

                  <div className="bg-bg-primary rounded-lg p-4">
                    <h4 className="text-sm font-medium text-slate-300 mb-2">
                      What happens when you publish:
                    </h4>
                    <ul className="text-sm text-slate-400 space-y-2">
                      <li className="flex items-start gap-2">
                        <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                        Content is signed with your Ed25519 key
                      </li>
                      <li className="flex items-start gap-2">
                        <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                        A unique CID (Content ID) is generated using BLAKE3
                      </li>
                      <li className="flex items-start gap-2">
                        <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                        Content is distributed to DHT peers with 3x replication
                      </li>
                      <li className="flex items-start gap-2">
                        <CheckIcon className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                        Anyone can verify authenticity using the CID
                      </li>
                    </ul>
                  </div>
                </div>
              )}
            </>
          )}

          {step === 'publishing' && (
            <div className="py-8 text-center">
              <SpinnerIcon className="w-12 h-12 animate-spin text-wraith-primary mx-auto" />
              <p className="text-slate-300 mt-4">
                {isPublished ? 'Unpublishing...' : 'Publishing...'}
              </p>
              <p className="text-sm text-slate-500 mt-2">
                {isPublished
                  ? 'Removing from network'
                  : 'Signing and distributing to peers'}
              </p>
            </div>
          )}

          {step === 'success' && publishedCid && (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <SuccessIcon className="w-6 h-6 text-green-500 flex-shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-medium text-white">Published successfully!</h3>
                  <p className="text-sm text-slate-400 mt-1">
                    Your article is now live on the WRAITH network.
                  </p>
                </div>
              </div>

              <div className="bg-bg-primary rounded-lg p-4">
                <div className="flex items-center justify-between mb-2">
                  <h4 className="text-sm font-medium text-slate-300">Content ID (CID)</h4>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={handleCopyCid}
                      className="p-1.5 text-slate-400 hover:text-white transition-colors"
                      title="Copy CID"
                    >
                      <CopyIcon className="w-4 h-4" />
                    </button>
                    <button
                      onClick={handleCopyLink}
                      className="p-1.5 text-slate-400 hover:text-white transition-colors"
                      title="Copy link"
                    >
                      <LinkIcon className="w-4 h-4" />
                    </button>
                  </div>
                </div>
                <code className="text-xs text-wraith-primary break-all font-mono">
                  {publishedCid}
                </code>
              </div>

              {/* Propagation status */}
              {propagation && (
                <div className="bg-bg-primary rounded-lg p-4">
                  <h4 className="text-sm font-medium text-slate-300 mb-3">
                    Network Propagation
                  </h4>
                  <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-slate-400">Replicas</span>
                      <span className="text-white">
                        {propagation.confirmed_replicas} / {propagation.target_replicas}
                      </span>
                    </div>
                    <div className="h-2 bg-bg-tertiary rounded-full overflow-hidden">
                      <div
                        className="h-full bg-wraith-primary transition-all duration-300"
                        style={{
                          width: `${(propagation.confirmed_replicas / propagation.target_replicas) * 100}%`,
                        }}
                      />
                    </div>
                    <p className="text-xs text-slate-500">
                      {propagation.confirmed_replicas >= propagation.target_replicas
                        ? 'Fully propagated'
                        : 'Propagating to peers...'}
                    </p>
                  </div>
                </div>
              )}
            </div>
          )}

          {step === 'error' && (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <ErrorIcon className="w-6 h-6 text-red-500 flex-shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-medium text-white">
                    {isPublished ? 'Failed to unpublish' : 'Failed to publish'}
                  </h3>
                  <p className="text-sm text-slate-400 mt-1">
                    {error || 'An unexpected error occurred'}
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-slate-700 flex items-center justify-end gap-3">
          {step === 'confirm' && (
            <>
              <button
                onClick={onClose}
                className="px-4 py-2 text-slate-300 hover:text-white transition-colors"
              >
                Cancel
              </button>
              {isPublished ? (
                <button
                  onClick={handleUnpublish}
                  className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white font-medium rounded-lg transition-colors"
                >
                  Unpublish
                </button>
              ) : (
                <button
                  onClick={handlePublish}
                  className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary text-white font-medium rounded-lg transition-colors"
                >
                  Publish
                </button>
              )}
            </>
          )}

          {step === 'publishing' && (
            <button
              disabled
              className="px-4 py-2 bg-slate-600 text-white font-medium rounded-lg opacity-50 cursor-not-allowed"
            >
              Please wait...
            </button>
          )}

          {(step === 'success' || step === 'error') && (
            <button
              onClick={onClose}
              className="px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary text-white font-medium rounded-lg transition-colors"
            >
              Done
            </button>
          )}
        </div>
      </div>
    </div>
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

function WarningIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
      />
    </svg>
  );
}

function SpinnerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24">
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
}

function SuccessIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

function ErrorIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
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

function LinkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
      />
    </svg>
  );
}
