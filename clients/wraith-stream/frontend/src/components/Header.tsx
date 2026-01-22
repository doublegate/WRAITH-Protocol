import { useAppStore } from '../stores/appStore';
import { useStreamStore } from '../stores/streamStore';

// SVG Icons
const SearchIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

const SettingsIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const LogoIcon = () => (
  <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
    <rect width="32" height="32" rx="8" fill="url(#gradient)" />
    <path d="M8 10L16 6L24 10V22L16 26L8 22V10Z" fill="white" fillOpacity="0.2" />
    <path d="M16 6L24 10L16 14L8 10L16 6Z" fill="white" />
    <path d="M16 14V26L8 22V10L16 14Z" fill="white" fillOpacity="0.7" />
    <path d="M16 14V26L24 22V10L16 14Z" fill="white" fillOpacity="0.4" />
    <defs>
      <linearGradient id="gradient" x1="0" y1="0" x2="32" y2="32" gradientUnits="userSpaceOnUse">
        <stop stopColor="#00a3ff" />
        <stop offset="1" stopColor="#00d4aa" />
      </linearGradient>
    </defs>
  </svg>
);

export default function Header() {
  const { searchQuery, setSearchQuery, setSettingsOpen } = useAppStore();
  const { searchStreams, fetchStreams } = useStreamStore();

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      searchStreams(searchQuery);
    } else {
      fetchStreams();
    }
  };

  return (
    <header className="header">
      <div className="flex items-center gap-4 flex-1">
        {/* Logo */}
        <div className="flex items-center gap-3">
          <LogoIcon />
          <span className="text-lg font-bold text-[var(--color-text-primary)]">
            WRAITH Stream
          </span>
        </div>

        {/* Search Bar */}
        <form onSubmit={handleSearch} className="flex-1 max-w-xl ml-8">
          <div className="search-bar">
            <SearchIcon />
            <input
              type="text"
              placeholder="Search streams..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </form>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2">
        <button
          onClick={() => setSettingsOpen(true)}
          className="player-button hover:bg-[var(--color-bg-hover)]"
          title="Settings"
        >
          <SettingsIcon />
        </button>
      </div>
    </header>
  );
}
