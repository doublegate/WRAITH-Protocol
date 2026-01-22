import { useStreamStore } from '../stores/streamStore';
import type { StreamSummary } from '../types';

interface StreamCardProps {
  stream: StreamSummary;
}

const PlayIcon = () => (
  <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
    <polygon points="5,3 19,12 5,21" />
  </svg>
);

const formatDuration = (seconds: number | null): string => {
  if (seconds === null) return '';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
};

const formatViewCount = (count: number): string => {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M views`;
  }
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}K views`;
  }
  return `${count} view${count !== 1 ? 's' : ''}`;
};

export default function StreamCard({ stream }: StreamCardProps) {
  const { selectStream } = useStreamStore();

  return (
    <div
      className="card group cursor-pointer"
      onClick={() => selectStream(stream.id)}
    >
      {/* Thumbnail */}
      <div className="thumbnail">
        {stream.thumbnail_url ? (
          <img
            src={stream.thumbnail_url}
            alt={stream.title}
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-[var(--color-bg-tertiary)] to-[var(--color-bg-elevated)]">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-[var(--color-text-muted)]">
              <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
              <line x1="7" y1="2" x2="7" y2="22" />
              <line x1="17" y1="2" x2="17" y2="22" />
            </svg>
          </div>
        )}

        {/* Play overlay */}
        <div className="absolute inset-0 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity">
          <div className="w-14 h-14 rounded-full bg-[var(--color-primary-500)] flex items-center justify-center text-white">
            <PlayIcon />
          </div>
        </div>

        {/* Live badge */}
        {stream.is_live && (
          <span className="absolute top-2 left-2 live-badge">LIVE</span>
        )}

        {/* Duration badge */}
        {!stream.is_live && stream.duration && (
          <span className="duration-badge">{formatDuration(stream.duration)}</span>
        )}
      </div>

      {/* Info */}
      <div className="p-3">
        <h3 className="font-medium text-[var(--color-text-primary)] line-clamp-2 mb-1 group-hover:text-[var(--color-primary-400)] transition-colors">
          {stream.title}
        </h3>
        <div className="flex items-center gap-2 text-sm text-[var(--color-text-secondary)]">
          <span className="truncate">{stream.created_by}</span>
          <span>-</span>
          <span className="view-count">{formatViewCount(stream.view_count)}</span>
        </div>
      </div>
    </div>
  );
}
