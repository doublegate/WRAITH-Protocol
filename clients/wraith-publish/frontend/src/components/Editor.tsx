import { useEffect, useState, useCallback, useRef } from 'react';
import { useContentStore } from '../stores/contentStore';
import { useUIStore } from '../stores/uiStore';
import type { Article } from '../types';

interface EditorProps {
  article: Article | null;
}

export function Editor({ article }: EditorProps) {
  const { updateDraft, saveDraft } = useContentStore();
  const { editorMode, showNotification, setShowPublishModal } = useUIStore();
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [lastSaved, setLastSaved] = useState<Date | null>(null);
  const saveTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Load article data
  useEffect(() => {
    if (article) {
      setTitle(article.title);
      setContent(article.content);
      setTags(article.tags);
    }
  }, [article?.id]);

  // Auto-save with debounce
  const debouncedSave = useCallback(async () => {
    if (!article || article.status !== 'draft') return;

    setIsSaving(true);
    try {
      await saveDraft(article.id);
      setLastSaved(new Date());
    } catch (error) {
      console.error('Auto-save failed:', error);
    } finally {
      setIsSaving(false);
    }
  }, [article, saveDraft]);

  // Schedule auto-save on content change
  useEffect(() => {
    if (!article || article.status !== 'draft') return;

    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    saveTimeoutRef.current = setTimeout(() => {
      debouncedSave();
    }, 2000);

    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [title, content, tags, debouncedSave, article]);

  // Update draft in store
  const handleTitleChange = (value: string) => {
    setTitle(value);
    if (article) {
      updateDraft(article.id, { title: value });
    }
  };

  const handleContentChange = (value: string) => {
    setContent(value);
    if (article) {
      updateDraft(article.id, { content: value });
    }
  };

  // Tag management
  const handleAddTag = () => {
    const tag = tagInput.trim().toLowerCase();
    if (tag && !tags.includes(tag)) {
      const newTags = [...tags, tag];
      setTags(newTags);
      if (article) {
        updateDraft(article.id, { tags: newTags });
      }
    }
    setTagInput('');
  };

  const handleRemoveTag = (tagToRemove: string) => {
    const newTags = tags.filter((t) => t !== tagToRemove);
    setTags(newTags);
    if (article) {
      updateDraft(article.id, { tags: newTags });
    }
  };

  const handleTagKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddTag();
    }
  };

  // Manual save
  const handleManualSave = async () => {
    if (!article || article.status !== 'draft') return;

    setIsSaving(true);
    try {
      await saveDraft(article.id);
      setLastSaved(new Date());
      showNotification({ type: 'success', message: 'Draft saved' });
    } catch (error) {
      showNotification({ type: 'error', message: 'Failed to save draft' });
    } finally {
      setIsSaving(false);
    }
  };

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        handleManualSave();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleManualSave]);

  if (!article) {
    return (
      <div className="flex-1 flex items-center justify-center bg-bg-primary">
        <div className="text-center text-slate-500">
          <DocumentIcon className="w-16 h-16 mx-auto mb-4" />
          <p className="text-lg">Select an article to edit</p>
          <p className="text-sm mt-1">Or create a new article from the sidebar</p>
        </div>
      </div>
    );
  }

  const isPublished = article.status === 'published';

  return (
    <div className="flex-1 flex flex-col bg-bg-primary h-full overflow-hidden">
      {/* Editor toolbar */}
      <div className="border-b border-slate-700 px-6 py-3 flex items-center justify-between bg-bg-secondary">
        <div className="flex items-center gap-4">
          <span className="text-sm text-slate-400">
            {isPublished ? (
              <span className="flex items-center gap-2">
                <LockIcon className="w-4 h-4" />
                Published (read-only)
              </span>
            ) : (
              <span className="flex items-center gap-2">
                <EditIcon className="w-4 h-4" />
                Editing draft
              </span>
            )}
          </span>

          {lastSaved && (
            <span className="text-xs text-slate-500">
              Last saved: {lastSaved.toLocaleTimeString()}
            </span>
          )}

          {isSaving && (
            <span className="text-xs text-slate-500 flex items-center gap-1">
              <SpinnerIcon className="w-3 h-3 animate-spin" />
              Saving...
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {!isPublished && (
            <>
              <button
                onClick={handleManualSave}
                disabled={isSaving}
                className="px-3 py-1.5 text-sm text-slate-300 hover:text-white transition-colors disabled:opacity-50"
              >
                Save
              </button>
              <button
                onClick={() => setShowPublishModal(true)}
                className="px-4 py-1.5 bg-wraith-primary hover:bg-wraith-secondary text-white text-sm font-medium rounded-lg transition-colors"
              >
                Publish
              </button>
            </>
          )}
        </div>
      </div>

      {/* Editor content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Edit panel */}
        {(editorMode === 'edit' || editorMode === 'split') && (
          <div
            className={`flex flex-col ${
              editorMode === 'split' ? 'w-1/2 border-r border-slate-700' : 'flex-1'
            }`}
          >
            {/* Title input */}
            <div className="p-6 border-b border-slate-700">
              <input
                type="text"
                value={title}
                onChange={(e) => handleTitleChange(e.target.value)}
                placeholder="Article title..."
                disabled={isPublished}
                className="w-full text-2xl font-bold bg-transparent text-white placeholder-slate-500 focus:outline-none disabled:cursor-not-allowed"
              />

              {/* Tags */}
              <div className="mt-4 flex flex-wrap items-center gap-2">
                {tags.map((tag) => (
                  <span
                    key={tag}
                    className="inline-flex items-center gap-1 px-2 py-1 bg-bg-tertiary rounded-full text-sm text-slate-300"
                  >
                    #{tag}
                    {!isPublished && (
                      <button
                        onClick={() => handleRemoveTag(tag)}
                        className="hover:text-red-400 transition-colors"
                      >
                        <CloseIcon className="w-3 h-3" />
                      </button>
                    )}
                  </span>
                ))}
                {!isPublished && (
                  <input
                    type="text"
                    value={tagInput}
                    onChange={(e) => setTagInput(e.target.value)}
                    onKeyDown={handleTagKeyDown}
                    onBlur={handleAddTag}
                    placeholder="Add tag..."
                    className="px-2 py-1 bg-transparent text-sm text-slate-400 placeholder-slate-600 focus:outline-none"
                  />
                )}
              </div>
            </div>

            {/* Content textarea */}
            <div className="flex-1 overflow-auto">
              <textarea
                ref={textareaRef}
                value={content}
                onChange={(e) => handleContentChange(e.target.value)}
                placeholder="Write your article in Markdown..."
                disabled={isPublished}
                className="w-full h-full p-6 bg-transparent text-slate-200 placeholder-slate-500 resize-none focus:outline-none font-mono text-sm disabled:cursor-not-allowed"
              />
            </div>
          </div>
        )}

        {/* Preview panel */}
        {(editorMode === 'preview' || editorMode === 'split') && (
          <div
            className={`flex flex-col ${
              editorMode === 'split' ? 'w-1/2' : 'flex-1'
            } overflow-auto`}
          >
            <div className="p-6">
              <h1 className="text-3xl font-bold text-white mb-4">
                {title || 'Untitled'}
              </h1>

              {tags.length > 0 && (
                <div className="flex flex-wrap gap-2 mb-6">
                  {tags.map((tag) => (
                    <span
                      key={tag}
                      className="px-2 py-1 bg-bg-tertiary rounded-full text-sm text-slate-400"
                    >
                      #{tag}
                    </span>
                  ))}
                </div>
              )}

              <MarkdownPreview content={content} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Safe markdown preview component (text-based rendering)
function MarkdownPreview({ content }: { content: string }) {
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
        <h3 key={key} className="text-lg font-semibold mt-4 mb-2 text-white">
          {line.slice(4)}
        </h3>
      );
    } else if (line.startsWith('## ')) {
      elements.push(
        <h2 key={key} className="text-xl font-semibold mt-6 mb-3 text-white">
          {line.slice(3)}
        </h2>
      );
    } else if (line.startsWith('# ')) {
      elements.push(
        <h1 key={key} className="text-2xl font-bold mt-6 mb-4 text-white">
          {line.slice(2)}
        </h1>
      );
    }
    // Horizontal rule
    else if (line === '---' || line === '***') {
      elements.push(<hr key={key} className="my-6 border-slate-600" />);
    }
    // Blockquote
    else if (line.startsWith('> ')) {
      elements.push(
        <blockquote key={key} className="border-l-4 border-wraith-primary pl-4 italic my-4 text-slate-300">
          {formatInlineText(line.slice(2))}
        </blockquote>
      );
    }
    // Unordered list
    else if (line.startsWith('- ') || line.startsWith('* ')) {
      elements.push(
        <li key={key} className="ml-6 list-disc text-slate-200">
          {formatInlineText(line.slice(2))}
        </li>
      );
    }
    // Ordered list
    else if (/^\d+\. /.test(line)) {
      const match = line.match(/^\d+\. (.*)$/);
      if (match && match[1]) {
        elements.push(
          <li key={key} className="ml-6 list-decimal text-slate-200">
            {formatInlineText(match[1])}
          </li>
        );
      }
    }
    // Code block start
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
          <code className="text-sm text-slate-200">{codeLines.join('\n')}</code>
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
        <p key={key} className="mb-4 text-slate-200">
          {formatInlineText(line)}
        </p>
      );
    }
    i++;
  }

  return <div className="prose-custom">{elements}</div>;
}

// Format inline text (bold, italic, code, links)
function formatInlineText(text: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  let remaining = text;
  let key = 0;

  while (remaining.length > 0) {
    // Inline code
    const codeMatch = remaining.match(/^`([^`]+)`/);
    if (codeMatch) {
      parts.push(
        <code key={key++} className="bg-bg-tertiary px-1.5 py-0.5 rounded text-sm">
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
        <strong key={key++}>
          <em>{boldItalicMatch[1]}</em>
        </strong>
      );
      remaining = remaining.slice(boldItalicMatch[0].length);
      continue;
    }

    // Bold
    const boldMatch = remaining.match(/^\*\*([^*]+)\*\*/);
    if (boldMatch) {
      parts.push(<strong key={key++}>{boldMatch[1]}</strong>);
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

    // Regular text - find next special character
    const nextSpecial = remaining.search(/[`*\[]/);
    if (nextSpecial === -1) {
      parts.push(<span key={key++}>{remaining}</span>);
      break;
    } else if (nextSpecial === 0) {
      // Special char didn't match a pattern, treat as regular text
      parts.push(<span key={key++}>{remaining[0]}</span>);
      remaining = remaining.slice(1);
    } else {
      parts.push(<span key={key++}>{remaining.slice(0, nextSpecial)}</span>);
      remaining = remaining.slice(nextSpecial);
    }
  }

  return parts;
}

// Icons
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

function LockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
      />
    </svg>
  );
}

function EditIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
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
