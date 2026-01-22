import { useContentStore } from '../stores/contentStore';
import { useUIStore } from '../stores/uiStore';
import { VerifyBadgeCompact } from './VerifyBadge';
import type { Article } from '../types';

interface ContentCardProps {
  article: Article;
  variant: 'card' | 'row';
}

export function ContentCard({ article, variant }: ContentCardProps) {
  const { selectArticle } = useContentStore();
  const { setViewMode } = useUIStore();

  const handleClick = () => {
    selectArticle(article);
    setViewMode(article.status === 'published' ? 'reader' : 'editor');
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: date.getFullYear() !== new Date().getFullYear() ? 'numeric' : undefined,
    });
  };

  const getExcerpt = (content: string, maxLength: number = 150) => {
    const stripped = content
      .replace(/^#+ /gm, '')
      .replace(/\*\*([^*]+)\*\*/g, '$1')
      .replace(/\*([^*]+)\*/g, '$1')
      .replace(/`([^`]+)`/g, '$1')
      .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
      .replace(/\n+/g, ' ')
      .trim();

    if (stripped.length <= maxLength) return stripped;
    return stripped.slice(0, maxLength).trim() + '...';
  };

  const getWordCount = (content: string) => {
    return content
      .replace(/[#*`\[\]()]/g, '')
      .split(/\s+/)
      .filter((word) => word.length > 0).length;
  };

  if (variant === 'row') {
    return (
      <button
        onClick={handleClick}
        className="w-full p-4 bg-bg-secondary hover:bg-bg-tertiary border border-slate-700 rounded-lg text-left transition-colors flex items-center gap-4"
      >
        {/* Status indicator */}
        <div className="flex-shrink-0">
          <StatusBadge status={article.status} />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="font-medium text-white truncate">
              {article.title || 'Untitled'}
            </h3>
            {article.status === 'published' && article.cid && (
              <VerifyBadgeCompact isVerified={true} />
            )}
          </div>
          <p className="text-sm text-slate-400 truncate mt-1">
            {getExcerpt(article.content, 100) || 'No content'}
          </p>
        </div>

        {/* Tags */}
        <div className="hidden md:flex items-center gap-1 flex-shrink-0">
          {article.tags.slice(0, 2).map((tag) => (
            <span
              key={tag}
              className="px-2 py-0.5 text-xs bg-bg-tertiary rounded text-slate-400"
            >
              {tag}
            </span>
          ))}
          {article.tags.length > 2 && (
            <span className="text-xs text-slate-500">+{article.tags.length - 2}</span>
          )}
        </div>

        {/* Meta */}
        <div className="flex items-center gap-4 text-xs text-slate-500 flex-shrink-0">
          <span>{getWordCount(article.content)} words</span>
          <span>{formatDate(article.updated_at)}</span>
        </div>
      </button>
    );
  }

  // Card variant
  return (
    <button
      onClick={handleClick}
      className="w-full bg-bg-secondary hover:bg-bg-tertiary border border-slate-700 rounded-lg text-left transition-colors overflow-hidden flex flex-col"
    >
      {/* Header */}
      <div className="p-4 flex-1">
        <div className="flex items-start justify-between gap-2 mb-2">
          <h3 className="font-medium text-white line-clamp-2 flex-1">
            {article.title || 'Untitled'}
          </h3>
          <StatusBadge status={article.status} />
        </div>

        <p className="text-sm text-slate-400 line-clamp-3">
          {getExcerpt(article.content, 120) || 'No content'}
        </p>

        {/* Tags */}
        {article.tags.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-3">
            {article.tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                className="px-2 py-0.5 text-xs bg-bg-tertiary rounded text-slate-400"
              >
                #{tag}
              </span>
            ))}
            {article.tags.length > 3 && (
              <span className="text-xs text-slate-500">+{article.tags.length - 3}</span>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-slate-700 flex items-center justify-between text-xs text-slate-500">
        <span>{formatDate(article.updated_at)}</span>
        <div className="flex items-center gap-3">
          <span>{getWordCount(article.content)} words</span>
          {article.status === 'published' && article.cid && (
            <VerifyBadgeCompact isVerified={true} />
          )}
        </div>
      </div>
    </button>
  );
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const config = {
    draft: { bg: 'bg-slate-600', text: 'text-slate-200', label: 'Draft' },
    published: { bg: 'bg-green-600', text: 'text-white', label: 'Published' },
    archived: { bg: 'bg-yellow-600', text: 'text-white', label: 'Archived' },
  }[status] || { bg: 'bg-slate-600', text: 'text-slate-200', label: status };

  return (
    <span
      className={`px-2 py-0.5 text-xs font-medium rounded-full ${config.bg} ${config.text} flex-shrink-0`}
    >
      {config.label}
    </span>
  );
}
