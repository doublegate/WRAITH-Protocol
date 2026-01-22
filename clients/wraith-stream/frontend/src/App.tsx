import { useEffect } from 'react';
import { useAppStore } from './stores/appStore';
import { useStreamStore } from './stores/streamStore';
import Header from './components/Header';
import Sidebar from './components/Sidebar';
import StreamGrid from './components/StreamGrid';
import VideoPlayer from './components/VideoPlayer';
import UploadPanel from './components/UploadPanel';
import MyStreams from './components/MyStreams';
import SettingsModal from './components/SettingsModal';

function App() {
  const { initialize, isInitialized, isSettingsOpen } = useAppStore();
  const { currentView, fetchStreams, fetchTrendingStreams } = useStreamStore();

  useEffect(() => {
    initialize();
    fetchStreams();
    fetchTrendingStreams();
  }, [initialize, fetchStreams, fetchTrendingStreams]);

  if (!isInitialized) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-[var(--color-bg-primary)]">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 border-4 border-[var(--color-primary-500)] border-t-transparent rounded-full animate-spin" />
          <p className="text-[var(--color-text-secondary)]">Initializing WRAITH Stream...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen flex flex-col overflow-hidden bg-[var(--color-bg-primary)]">
      <Header />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar />

        <main className="flex-1 overflow-auto p-6">
          {currentView === 'browse' && <StreamGrid />}
          {currentView === 'player' && <VideoPlayer />}
          {currentView === 'upload' && <UploadPanel />}
          {currentView === 'my-streams' && <MyStreams />}
        </main>
      </div>

      {isSettingsOpen && <SettingsModal />}
    </div>
  );
}

export default App;
