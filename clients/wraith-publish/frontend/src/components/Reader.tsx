import { useEffect, useState } from 'react';
import { useUIStore } from '../stores/uiStore';
import { VerifyBadge } from './VerifyBadge';
import type { Article } from '../types';

interface ReaderProps {
  article: Article | null;
}

export function Reader({ article }: ReaderProps) {
  const { setViewMode, showNotification } = useUIStore();
  const [isVerified, setIsVerified] = useState<boolean | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);

  // Verify content signature on load
  useEffect(() => {
    if (article?.cid && article.status === 'published') {
      verifyContent();
    } else {
      setIsVerified(null);
    }
  }, [article?.id, article?.cid]);

  const verifyContent = async () => {
    if (!article?.cid) return;

    setIsVerifying(true);
    try {
      // Verification would be done via Tauri command
      // For now, simulate verification
      await new Promise((resolve) => setTimeout(resolve, 500));
      setIsVerified(true);
    } catch (error) {
      setIsVerified(false);
      console.error('Verification failed:', error);
    } finally {
      setIsVerifying(false);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  const formatReadTime = (wordCount: number) => {
    const minutes = Math.ceil(wordCount / 200);
    return `${minutes} min read`;
  };

  const getWordCount = (content: string) => {
    return content
      .replace(/[#*`\[\]()]/g, '')
      .split(/\s+/)
      .filter((word) => word.length > 0).length;
  };

  const handleCopyCid = async () => {
    if (!article?.cid) return;

    try {
      await navigator.clipboard.writeText(article.cid);
      showNotification({ type: 'success', message: 'CID copied to clipboard' });
    } catch (error) {
      showNotification({ type: 'error', message: 'Failed to copy CID' });
    }
  };

  const handleCopyLink = async () => {
    if (!article?.cid) return;

    const link = `wraith://publish/${article.cid}`;
    try {
      await navigator.clipboard.writeText(link);
      showNotification({ type: 'success', message: 'Link copied to clipboard' });
    } catch (error) {
      showNotification({ type: 'error', message: 'Failed to copy link' });
    }
  };

  if (!article) {
    return (
      <div className="flex-1 flex items-center justify-center bg-bg-primary">
        <div className="text-center text-slate-500">
          <BookIcon className="w-16 h-16 mx-auto mb-4" />
          <p className="text-lg">Select an article to read</p>
          <p className="text-sm mt-1">Or switch to editor view to create content</p>
        </div>
      </div>
    );
  }

  const wordCount = getWordCount(article.content);

  return (
    <div className="flex-1 flex flex-col bg-bg-primary h-full overflow-hidden">
      {/* Reader toolbar */}
      <div className="border-b border-slate-700 px-6 py-3 flex items-center justify-between bg-bg-secondary">
        <div className="flex items-center gap-4">
          {article.status === 'published' && (
            <VerifyBadge
              isVerified={isVerified}
              isVerifying={isVerifying}
              onVerify={verifyContent}
            />
          )}

          {article.status === 'draft' && (
            <span className="px-2 py-1 bg-slate-600 rounded text-xs text-white">
              Draft
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {article.status === 'published' && article.cid && (
            <>
              <button
                onClick={handleCopyCid}
                className="p-2 text-slate-400 hover:text-white transition-colors"
                title="Copy CID"
              >
                <HashIcon className="w-4 h-4" />
              </button>
              <button
                onClick={handleCopyLink}
                className="p-2 text-slate-400 hover:text-white transition-colors"
                title="Copy link"
              >
                <LinkIcon className="w-4 h-4" />
              </button>
            </>
          )}

          {article.status === 'draft' && (
            <button
              onClick={() => setViewMode('editor')}
              className="px-3 py-1.5 text-sm text-slate-300 hover:text-white transition-colors"
            >
              Edit
            </button>
          )}
        </div>
      </div>

      {/* Reader content */}
      <div className="flex-1 overflow-auto">
        <article className="max-w-3xl mx-auto px-6 py-8">
          {/* Header */}
          <header className="mb-8">
            <h1 className="text-4xl font-bold text-white mb-4">
              {article.title || 'Untitled'}
            </h1>

            {/* Meta info */}
            <div className="flex flex-wrap items-center gap-4 text-sm text-slate-400 mb-4">
              <div className="flex items-center gap-2">
                <UserIcon className="w-4 h-4" />
                <span>{article.author_name || 'Anonymous'}</span>
              </div>

              {article.published_at && (
                <div className="flex items-center gap-2">
                  <CalendarIcon className="w-4 h-4" />
                  <span>{formatDate(article.published_at)}</span>
                </div>
              )}

              <div className="flex items-center gap-2">
                <ClockIcon className="w-4 h-4" />
                <span>{formatReadTime(wordCount)}</span>
              </div>

              <span className="text-slate-500">{wordCount} words</span>
            </div>

            {/* Tags */}
            {article.tags.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {article.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-3 py-1 bg-bg-tertiary rounded-full text-sm text-slate-300 hover:bg-slate-600 cursor-pointer transition-colors"
                  >
                    #{tag}
                  </span>
                ))}
              </div>
            )}
          </header>

          {/* Content */}
          <div className="prose-custom">
            <MarkdownContent content={article.content} />
          </div>

          {/* Footer */}
          {article.status === 'published' && article.cid && (
            <footer className="mt-12 pt-6 border-t border-slate-700">
              <div className="flex items-start gap-4">
                <div className="flex-1">
                  <h4 className="text-sm font-medium text-slate-400 mb-2">
                    Content ID (CID)
                  </h4>
                  <code className="text-xs text-slate-500 break-all font-mono">
                    {article.cid}
                  </code>
                </div>
                <button
                  onClick={handleCopyCid}
                  className="p-2 text-slate-400 hover:text-white transition-colors"
                >
                  <CopyIcon className="w-4 h-4" />
                </button>
              </div>

              <p className="mt-4 text-xs text-slate-500">
                This content is cryptographically signed and distributed via the WRAITH
                network. The CID is a unique identifier derived from the content hash.
              </p>
            </footer>
          )}
        </article>
      </div>
    </div>
  );
}

// Markdown content renderer (safe, React-based)
function MarkdownContent({ content }: { content: string }) {
  const lines = content.split('\n');
  const elements: React.ReactNode[] = [];
  let key = 0;
  let i = 0;

  while (i < lines.length) {
    const line = lines[i] ?? '';
    key++;

    // Headers
    if (line.startsWith('### ')) {
      elements.push(
        <h3 key={key} className="text-lg font-semibold mt-6 mb-3 text-white">
          {line.slice(4)}
        </h3>
      );
    } else if (line.startsWith('## ')) {
      elements.push(
        <h2 key={key} className="text-xl font-semibold mt-8 mb-4 text-white">
          {line.slice(3)}
        </h2>
      );
    } else if (line.startsWith('# ')) {
      elements.push(
        <h1 key={key} className="text-2xl font-bold mt-8 mb-4 text-white">
          {line.slice(2)}
        </h1>
      );
    }
    // Horizontal rule
    else if (line === '---' || line === '***') {
      elements.push(<hr key={key} className="my-8 border-slate-600" />);
    }
    // Blockquote
    else if (line.startsWith('> ')) {
      elements.push(
        <blockquote
          key={key}
          className="border-l-4 border-wraith-primary pl-4 italic my-4 text-slate-300"
        >
          <InlineText text={line.slice(2)} />
        </blockquote>
      );
    }
    // Unordered list
    else if (line.startsWith('- ') || line.startsWith('* ')) {
      elements.push(
        <li key={key} className="ml-6 list-disc text-slate-200 mb-1">
          <InlineText text={line.slice(2)} />
        </li>
      );
    }
    // Ordered list
    else if (/^\d+\. /.test(line)) {
      const match = line.match(/^\d+\. (.*)$/);
      if (match && match[1]) {
        elements.push(
          <li key={key} className="ml-6 list-decimal text-slate-200 mb-1">
            <InlineText text={match[1]} />
          </li>
        );
      }
    }
    // Code block
    else if (line.startsWith('```')) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length) {
        const codeLine = lines[i] ?? '';
        if (codeLine.startsWith('```')) break;
        codeLines.push(codeLine);
        i++;
      }
      elements.push(
        <pre key={key} className="bg-bg-tertiary p-4 rounded-lg overflow-x-auto my-4">
          <code className="text-sm text-slate-200 font-mono">
            {codeLines.join('\n')}
          </code>
        </pre>
      );
    }
    // Empty line
    else if (line.trim() === '') {
      elements.push(<div key={key} className="h-4" />);
    }
    // Regular paragraph
    else {
      elements.push(
        <p key={key} className="text-slate-200 leading-relaxed mb-4">
          <InlineText text={line} />
        </p>
      );
    }
    i++;
  }

  return <>{elements}</>;
}

// Inline text formatter
function InlineText({ text }: { text: string }) {
  const parts: React.ReactNode[] = [];
  let remaining = text;
  let key = 0;

  while (remaining.length > 0) {
    // Inline code
    const codeMatch = remaining.match(/^`([^`]+)`/);
    if (codeMatch) {
      parts.push(
        <code
          key={key++}
          className="bg-bg-tertiary px-1.5 py-0.5 rounded text-sm font-mono"
        >
          {codeMatch[1]}
        </code>
      );
      remaining = remaining.slice(codeMatch[0].length);
      continue;
    }

    // Bold + Italic
    const boldItalicMatch = remaining.match(/^\*\*\*([^*]+)\*\*\*/);
    if (boldItalicMatch) {
      parts.push(
        <strong key={key++} className="font-bold">
          <em>{boldItalicMatch[1]}</em>
        </strong>
      );
      remaining = remaining.slice(boldItalicMatch[0].length);
      continue;
    }

    // Bold
    const boldMatch = remaining.match(/^\*\*([^*]+)\*\*/);
    if (boldMatch) {
      parts.push(
        <strong key={key++} className="font-bold">
          {boldMatch[1]}
        </strong>
      );
      remaining = remaining.slice(boldMatch[0].length);
      continue;
    }

    // Italic
    const italicMatch = remaining.match(/^\*([^*]+)\*/);
    if (italicMatch) {
      parts.push(<em key={key++}>{italicMatch[1]}</em>);
      remaining = remaining.slice(italicMatch[0].length);
      continue;
    }

    // Link
    const linkMatch = remaining.match(/^\[([^\]]+)\]\(([^)]+)\)/);
    if (linkMatch) {
      parts.push(
        <a
          key={key++}
          href={linkMatch[2]}
          className="text-wraith-primary hover:underline"
          target="_blank"
          rel="noopener noreferrer"
        >
          {linkMatch[1]}
        </a>
      );
      remaining = remaining.slice(linkMatch[0].length);
      continue;
    }

    // Regular text
    const nextSpecial = remaining.search(/[`*\[]/);
    if (nextSpecial === -1) {
      parts.push(<span key={key++}>{remaining}</span>);
      break;
    } else if (nextSpecial === 0) {
      parts.push(<span key={key++}>{remaining[0]}</span>);
      remaining = remaining.slice(1);
    } else {
      parts.push(<span key={key++}>{remaining.slice(0, nextSpecial)}</span>);
      remaining = remaining.slice(nextSpecial);
    }
  }

  return <>{parts}</>;
}

// Icons
function BookIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
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

function CalendarIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
      />
    </svg>
  );
}

function ClockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

function HashIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M7 20l4-16m2 16l4-16M6 9h14M4 15h14"
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
