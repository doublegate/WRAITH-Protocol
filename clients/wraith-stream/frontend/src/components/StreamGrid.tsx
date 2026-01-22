import { useStreamStore } from '../stores/streamStore';
import StreamCard from './StreamCard';

export default function StreamGrid() {
  const { streams, searchResults, isLoading, error } = useStreamStore();

  // Use search results if available, otherwise use regular streams
  const displayStreams = searchResults?.streams || streams;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex flex-col items-center gap-4">
          <div className="w-10 h-10 border-4 border-[var(--color-primary-500)] border-t-transparent rounded-full animate-spin" />
          <p className="text-[var(--color-text-secondary)]">Loading streams...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <p className="text-[var(--color-error)] mb-2">Error loading streams</p>
          <p className="text-sm text-[var(--color-text-muted)]">{error}</p>
        </div>
      </div>
    );
  }

  if (displayStreams.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
            <line x1="7" y1="2" x2="7" y2="22" />
            <line x1="17" y1="2" x2="17" y2="22" />
            <line x1="2" y1="12" x2="22" y2="12" />
            <line x1="2" y1="7" x2="7" y2="7" />
            <line x1="2" y1="17" x2="7" y2="17" />
            <line x1="17" y1="17" x2="22" y2="17" />
            <line x1="17" y1="7" x2="22" y2="7" />
          </svg>
        </div>
        <p className="empty-state-title">No streams available</p>
        <p className="empty-state-description">
          {searchResults ? 'No streams match your search. Try different keywords.' : 'Be the first to upload a video!'}
        </p>
      </div>
    );
  }

  return (
    <div>
      {searchResults && (
        <div className="mb-6">
          <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-1">
            Search Results
          </h2>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Found {searchResults.total} streams for "{searchResults.query}"
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
        {displayStreams.map((stream) => (
          <StreamCard
            key={stream.id}
            stream={{
              id: stream.id,
              title: stream.title,
              thumbnail_url: 'thumbnail_url' in stream ? stream.thumbnail_url : null,
              duration: stream.duration,
              is_live: stream.is_live,
              view_count: stream.view_count,
              created_by: stream.created_by,
            }}
          />
        ))}
      </div>
    </div>
  );
}
