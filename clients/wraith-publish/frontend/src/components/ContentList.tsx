import { useMemo, useState } from 'react';
import { useContentStore } from '../stores/contentStore';
import { ContentCard } from './ContentCard';
import type { ArticleFilter } from '../types';

export function ContentList() {
  const { articles, filter, searchQuery, setFilter, setSearchQuery } = useContentStore();
  const [sortBy, setSortBy] = useState<'date' | 'title'>('date');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [viewStyle, setViewStyle] = useState<'grid' | 'list'>('grid');

  // Filter and sort articles
  const filteredArticles = useMemo(() => {
    let result = articles;

    // Apply status filter
    if (filter !== 'all') {
      const statusMap: Record<ArticleFilter, string> = {
        all: '',
        drafts: 'draft',
        published: 'published',
        archived: 'archived',
      };
      result = result.filter((a) => a.status === statusMap[filter]);
    }

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (a) =>
          a.title.toLowerCase().includes(query) ||
          a.content.toLowerCase().includes(query) ||
          a.tags.some((t) => t.toLowerCase().includes(query))
      );
    }

    // Sort
    result = [...result].sort((a, b) => {
      let comparison = 0;
      if (sortBy === 'date') {
        comparison = a.updated_at - b.updated_at;
      } else {
        comparison = a.title.localeCompare(b.title);
      }
      return sortOrder === 'desc' ? -comparison : comparison;
    });

    return result;
  }, [articles, filter, searchQuery, sortBy, sortOrder]);

  // Count by status
  const counts = useMemo(() => {
    return {
      all: articles.length,
      drafts: articles.filter((a) => a.status === 'draft').length,
      published: articles.filter((a) => a.status === 'published').length,
      archived: articles.filter((a) => a.status === 'archived').length,
    };
  }, [articles]);

  return (
    <div className="flex-1 flex flex-col bg-bg-primary h-full overflow-hidden">
      {/* Toolbar */}
      <div className="border-b border-slate-700 px-6 py-4 bg-bg-secondary">
        <div className="flex items-center justify-between gap-4">
          {/* Search */}
          <div className="relative flex-1 max-w-md">
            <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search articles..."
              className="w-full bg-bg-primary border border-slate-600 rounded-lg pl-10 pr-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500"
            />
          </div>

          {/* Filter tabs */}
          <div className="flex items-center gap-1 bg-bg-primary rounded-lg p-1">
            <FilterButton
              label="All"
              count={counts.all}
              active={filter === 'all'}
              onClick={() => setFilter('all')}
            />
            <FilterButton
              label="Drafts"
              count={counts.drafts}
              active={filter === 'drafts'}
              onClick={() => setFilter('drafts')}
            />
            <FilterButton
              label="Published"
              count={counts.published}
              active={filter === 'published'}
              onClick={() => setFilter('published')}
            />
          </div>

          {/* Sort and view controls */}
          <div className="flex items-center gap-2">
            {/* Sort dropdown */}
            <select
              value={`${sortBy}-${sortOrder}`}
              onChange={(e) => {
                const [by, order] = e.target.value.split('-') as ['date' | 'title', 'asc' | 'desc'];
                setSortBy(by);
                setSortOrder(order);
              }}
              className="bg-bg-primary border border-slate-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-cyan-500"
            >
              <option value="date-desc">Newest first</option>
              <option value="date-asc">Oldest first</option>
              <option value="title-asc">A-Z</option>
              <option value="title-desc">Z-A</option>
            </select>

            {/* View style toggle */}
            <div className="flex items-center gap-1 bg-bg-primary rounded-lg p-1">
              <button
                onClick={() => setViewStyle('grid')}
                className={`p-2 rounded transition-colors ${
                  viewStyle === 'grid'
                    ? 'bg-wraith-primary text-white'
                    : 'text-slate-400 hover:text-white'
                }`}
                title="Grid view"
              >
                <GridIcon className="w-4 h-4" />
              </button>
              <button
                onClick={() => setViewStyle('list')}
                className={`p-2 rounded transition-colors ${
                  viewStyle === 'list'
                    ? 'bg-wraith-primary text-white'
                    : 'text-slate-400 hover:text-white'
                }`}
                title="List view"
              >
                <ListIcon className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Content area */}
      <div className="flex-1 overflow-auto p-6">
        {filteredArticles.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-slate-500">
            <DocumentIcon className="w-16 h-16 mb-4" />
            <p className="text-lg">
              {searchQuery
                ? 'No articles match your search'
                : filter !== 'all'
                ? `No ${filter} articles`
                : 'No articles yet'}
            </p>
            {!searchQuery && filter === 'all' && (
              <p className="text-sm mt-2">
                Create your first article from the sidebar
              </p>
            )}
          </div>
        ) : viewStyle === 'grid' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {filteredArticles.map((article) => (
              <ContentCard key={article.id} article={article} variant="card" />
            ))}
          </div>
        ) : (
          <div className="space-y-2">
            {filteredArticles.map((article) => (
              <ContentCard key={article.id} article={article} variant="row" />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// Filter button component
interface FilterButtonProps {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}

function FilterButton({ label, count, active, onClick }: FilterButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
        active
          ? 'bg-wraith-primary text-white'
          : 'text-slate-400 hover:text-white'
      }`}
    >
      {label}
      {count > 0 && (
        <span className={`ml-1.5 ${active ? 'text-white/70' : 'text-slate-500'}`}>
          ({count})
        </span>
      )}
    </button>
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

function GridIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"
      />
    </svg>
  );
}

function ListIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 6h16M4 12h16M4 18h16"
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
