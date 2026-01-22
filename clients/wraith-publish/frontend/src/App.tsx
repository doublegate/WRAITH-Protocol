import { useEffect } from 'react';
import { Sidebar } from './components/Sidebar';
import { Editor } from './components/Editor';
import { Reader } from './components/Reader';
import { ContentList } from './components/ContentList';
import { Header } from './components/Header';
import { PublishModal } from './components/PublishModal';
import { SettingsModal } from './components/SettingsModal';
import { Notification } from './components/Notification';
import { useContentStore } from './stores/contentStore';
import { useUIStore } from './stores/uiStore';
import * as api from './lib/tauri';

function App() {
  const { fetchArticles, selectedArticle } = useContentStore();
  const {
    viewMode,
    showPublishModal,
    showSettingsModal,
    setPeerId,
    setDisplayName,
    setShowPublishModal,
    setShowSettingsModal,
  } = useUIStore();

  // Initialize app
  useEffect(() => {
    async function init() {
      try {
        const [peerId, displayName] = await Promise.all([
          api.getPeerId(),
          api.getDisplayName(),
        ]);
        setPeerId(peerId);
        setDisplayName(displayName);
        await fetchArticles();
      } catch (error) {
        console.error('Failed to initialize:', error);
      }
    }
    init();
  }, [fetchArticles, setPeerId, setDisplayName]);

  // Render main content based on view mode
  const renderMainContent = () => {
    switch (viewMode) {
      case 'editor':
        return <Editor article={selectedArticle} />;
      case 'reader':
        return <Reader article={selectedArticle} />;
      case 'list':
      default:
        return <ContentList />;
    }
  };

  return (
    <div className="h-screen bg-bg-primary text-slate-200 flex flex-col">
      <Header />

      <main className="flex-1 flex overflow-hidden">
        <Sidebar />
        <div className="flex-1 overflow-hidden">{renderMainContent()}</div>
      </main>

      {/* Modals */}
      {showPublishModal && selectedArticle && (
        <PublishModal
          article={selectedArticle}
          onClose={() => setShowPublishModal(false)}
        />
      )}

      {showSettingsModal && (
        <SettingsModal onClose={() => setShowSettingsModal(false)} />
      )}

      {/* Notifications */}
      <Notification />
    </div>
  );
}

export default App;
