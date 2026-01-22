import { useMemo, useState } from 'react';
import { useContentStore } from '../stores/contentStore';
import { useUIStore } from '../stores/uiStore';
import type { Article } from '../types';

export function Sidebar() {
  const {
    articles,
    selectedArticle,
    filter,
    searchQuery,
    selectArticle,
    setFilter,
    setSearchQuery,
    createDraft,
  } = useContentStore();
  const { sidebarCollapsed, setViewMode } = useUIStore();
  const [isCreating, setIsCreating] = useState(false);

  // Filter articles
  const filteredArticles = useMemo(() => {
    let result = articles;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (a) =>
          a.title.toLowerCase().includes(query) ||
          a.content.toLowerCase().includes(query) ||
          a.tags.some((t) => t.toLowerCase().includes(query))
      );
    }

    return result;
  }, [articles, searchQuery]);

  // Count by status
  const counts = useMemo(() => {
    return {
      all: articles.length,
      drafts: articles.filter((a) => a.status === 'draft').length,
      published: articles.filter((a) => a.status === 'published').length,
      archived: articles.filter((a) => a.status === 'archived').length,
    };
  }, [articles]);

  // Handle new article
  const handleNewArticle = async () => {
    setIsCreating(true);
    try {
      const article = await createDraft('Untitled', '', []);
      selectArticle(article);
      setViewMode('editor');
    } catch (error) {
      console.error('Failed to create draft:', error);
    } finally {
      setIsCreating(false);
    }
  };

  // Handle article selection
  const handleSelectArticle = (article: Article) => {
    selectArticle(article);
    setViewMode(article.status === 'published' ? 'reader' : 'editor');
  };

  if (sidebarCollapsed) {
    return null;
  }

  return (
    <div className="w-80 bg-bg-secondary border-r border-slate-700 flex flex-col h-full">
      {/* Search and new button */}
      <div className="p-4 space-y-3 border-b border-slate-700">
        {/* Search input */}
        <div className="relative">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search articles..."
            className="w-full bg-bg-primary border border-slate-600 rounded-lg pl-10 pr-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500"
          />
        </div>

        {/* New article button */}
        <button
          onClick={handleNewArticle}
          disabled={isCreating}
          className="w-full px-4 py-2 bg-wraith-primary hover:bg-wraith-secondary text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          <PlusIcon className="w-4 h-4" />
          {isCreating ? 'Creating...' : 'New Article'}
        </button>
      </div>

      {/* Filter tabs */}
      <div className="flex border-b border-slate-700">
        <FilterTab
          label="All"
          count={counts.all}
          active={filter === 'all'}
          onClick={() => setFilter('all')}
        />
        <FilterTab
          label="Drafts"
          count={counts.drafts}
          active={filter === 'drafts'}
          onClick={() => setFilter('drafts')}
        />
        <FilterTab
          label="Published"
          count={counts.published}
          active={filter === 'published'}
          onClick={() => setFilter('published')}
        />
      </div>

      {/* Article list */}
      <div className="flex-1 overflow-y-auto">
        {filteredArticles.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-slate-500 p-4">
            <DocumentIcon className="w-12 h-12 mb-2" />
            <p className="text-center">
              {searchQuery ? 'No articles match your search' : 'No articles yet'}
            </p>
            {!searchQuery && (
              <p className="text-sm text-center mt-1">
                Click "New Article" to get started
              </p>
            )}
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {filteredArticles.map((article) => (
              <ArticleItem
                key={article.id}
                article={article}
                isSelected={selectedArticle?.id === article.id}
                onClick={() => handleSelectArticle(article)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// Filter tab component
interface FilterTabProps {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}

function FilterTab({ label, count, active, onClick }: FilterTabProps) {
  return (
    <button
      onClick={onClick}
      className={`flex-1 py-2.5 text-sm font-medium transition-colors relative ${
        active ? 'text-wraith-primary' : 'text-slate-400 hover:text-slate-200'
      }`}
    >
      {label}
      {count > 0 && (
        <span
          className={`ml-1.5 text-xs ${active ? 'text-wraith-primary' : 'text-slate-500'}`}
        >
          ({count})
        </span>
      )}
      {active && (
        <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-wraith-primary" />
      )}
    </button>
  );
}

// Article item component
interface ArticleItemProps {
  article: Article;
  isSelected: boolean;
  onClick: () => void;
}

function ArticleItem({ article, isSelected, onClick }: ArticleItemProps) {
  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    });
  };

  return (
    <button
      onClick={onClick}
      className={`w-full p-3 rounded-lg text-left transition-colors ${
        isSelected
          ? 'bg-wraith-primary/20 border-l-2 border-wraith-primary'
          : 'hover:bg-bg-primary border-l-2 border-transparent'
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <h3 className="font-medium text-white truncate flex-1">
          {article.title || 'Untitled'}
        </h3>
        <StatusBadge status={article.status} />
      </div>
      <p className="text-sm text-slate-400 truncate mt-1">
        {article.content.slice(0, 100).replace(/^#\s*/, '') || 'No content'}
      </p>
      <div className="flex items-center justify-between mt-2">
        <span className="text-xs text-slate-500">
          {formatDate(article.updated_at)}
        </span>
        {article.tags.length > 0 && (
          <div className="flex gap-1">
            {article.tags.slice(0, 2).map((tag) => (
              <span
                key={tag}
                className="px-1.5 py-0.5 text-xs bg-bg-tertiary rounded text-slate-400"
              >
                {tag}
              </span>
            ))}
            {article.tags.length > 2 && (
              <span className="text-xs text-slate-500">
                +{article.tags.length - 2}
              </span>
            )}
          </div>
        )}
      </div>
    </button>
  );
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const config = {
    draft: { bg: 'bg-slate-600', text: 'text-slate-200' },
    published: { bg: 'bg-green-600', text: 'text-white' },
    archived: { bg: 'bg-yellow-600', text: 'text-white' },
  }[status] || { bg: 'bg-slate-600', text: 'text-slate-200' };

  return (
    <span
      className={`px-2 py-0.5 text-xs font-medium rounded-full ${config.bg} ${config.text}`}
    >
      {status}
    </span>
  );
}

// Icons
function SearchIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>
  );
}

function PlusIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 4v16m8-8H4"
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
