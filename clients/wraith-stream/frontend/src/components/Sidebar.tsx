import { useStreamStore } from '../stores/streamStore';
import type { ViewState } from '../types';

// SVG Icons
const HomeIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <polyline points="9,22 9,12 15,12 15,22" />
  </svg>
);

const UploadIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="17,8 12,3 7,8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </svg>
);

const VideoIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18" />
    <line x1="7" y1="2" x2="7" y2="22" />
    <line x1="17" y1="2" x2="17" y2="22" />
    <line x1="2" y1="12" x2="22" y2="12" />
    <line x1="2" y1="7" x2="7" y2="7" />
    <line x1="2" y1="17" x2="7" y2="17" />
    <line x1="17" y1="17" x2="22" y2="17" />
    <line x1="17" y1="7" x2="22" y2="7" />
  </svg>
);

const TrendingIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="23,6 13.5,15.5 8.5,10.5 1,18" />
    <polyline points="17,6 23,6 23,12" />
  </svg>
);

interface NavItem {
  id: ViewState;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { id: 'browse', label: 'Browse', icon: <HomeIcon /> },
  { id: 'upload', label: 'Upload', icon: <UploadIcon /> },
  { id: 'my-streams', label: 'My Streams', icon: <VideoIcon /> },
];

const categories = [
  'Gaming',
  'Music',
  'Education',
  'Technology',
  'Entertainment',
  'Sports',
  'News',
  'Creative',
];

export default function Sidebar() {
  const { currentView, setCurrentView, trendingStreams, selectStream } = useStreamStore();

  return (
    <aside className="sidebar">
      {/* Navigation */}
      <nav className="p-4">
        <ul className="flex flex-col gap-1">
          {navItems.map((item) => (
            <li key={item.id}>
              <button
                onClick={() => setCurrentView(item.id)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors ${
                  currentView === item.id
                    ? 'bg-[var(--color-primary-600)] text-white'
                    : 'text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-hover)] hover:text-[var(--color-text-primary)]'
                }`}
              >
                {item.icon}
                <span className="font-medium">{item.label}</span>
              </button>
            </li>
          ))}
        </ul>
      </nav>

      {/* Divider */}
      <div className="h-px bg-[var(--color-border-primary)] mx-4" />

      {/* Categories */}
      <div className="p-4">
        <h3 className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-3">
          Categories
        </h3>
        <div className="flex flex-wrap gap-2">
          {categories.map((category) => (
            <button
              key={category}
              className="category-pill"
            >
              {category}
            </button>
          ))}
        </div>
      </div>

      {/* Divider */}
      <div className="h-px bg-[var(--color-border-primary)] mx-4" />

      {/* Trending */}
      <div className="flex-1 p-4 overflow-auto">
        <h3 className="flex items-center gap-2 text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-3">
          <TrendingIcon />
          Trending
        </h3>
        <ul className="flex flex-col gap-2">
          {trendingStreams.slice(0, 5).map((stream) => (
            <li key={stream.id}>
              <button
                onClick={() => selectStream(stream.id)}
                className="w-full flex items-start gap-3 p-2 rounded-lg hover:bg-[var(--color-bg-hover)] transition-colors text-left"
              >
                <div className="w-16 h-9 rounded bg-[var(--color-bg-tertiary)] flex-shrink-0 overflow-hidden">
                  {stream.thumbnail_url ? (
                    <img
                      src={stream.thumbnail_url}
                      alt={stream.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-[var(--color-text-muted)]">
                      <VideoIcon />
                    </div>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-[var(--color-text-primary)] truncate">
                    {stream.title}
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    {stream.view_count.toLocaleString()} views
                  </p>
                </div>
                {stream.is_live && (
                  <span className="live-badge text-[10px] py-0.5 px-1.5">LIVE</span>
                )}
              </button>
            </li>
          ))}

          {trendingStreams.length === 0 && (
            <li className="text-center text-sm text-[var(--color-text-muted)] py-4">
              No trending streams
            </li>
          )}
        </ul>
      </div>

      {/* Footer */}
      <div className="p-4 border-t border-[var(--color-border-primary)]">
        <p className="text-xs text-[var(--color-text-muted)] text-center">
          WRAITH Stream v1.7.2
        </p>
      </div>
    </aside>
  );
}
