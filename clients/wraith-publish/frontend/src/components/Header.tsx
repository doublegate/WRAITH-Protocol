import { useContentStore } from '../stores/contentStore';
import { useUIStore } from '../stores/uiStore';

export function Header() {
  const { selectedArticle } = useContentStore();
  const {
    viewMode,
    setViewMode,
    editorMode,
    setEditorMode,
    setShowSettingsModal,
  } = useUIStore();

  return (
    <header className="bg-bg-secondary border-b border-slate-700 px-6 py-4">
      <div className="flex items-center justify-between">
        {/* Left: Logo and title */}
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold text-white flex items-center gap-2">
            <PublishIcon className="w-6 h-6 text-wraith-primary" />
            WRAITH Publish
          </h1>
        </div>

        {/* Center: View mode tabs (when in editor) */}
        {viewMode === 'editor' && (
          <div className="flex items-center gap-1 bg-bg-primary rounded-lg p-1">
            <button
              onClick={() => setEditorMode('edit')}
              className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                editorMode === 'edit'
                  ? 'bg-wraith-primary text-white'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              Edit
            </button>
            <button
              onClick={() => setEditorMode('split')}
              className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                editorMode === 'split'
                  ? 'bg-wraith-primary text-white'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              Split
            </button>
            <button
              onClick={() => setEditorMode('preview')}
              className={`px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                editorMode === 'preview'
                  ? 'bg-wraith-primary text-white'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              Preview
            </button>
          </div>
        )}

        {/* Right: Actions */}
        <div className="flex items-center gap-3">
          {/* View mode buttons */}
          <div className="flex items-center gap-1 bg-bg-primary rounded-lg p-1">
            <button
              onClick={() => setViewMode('list')}
              className={`p-2 rounded transition-colors ${
                viewMode === 'list'
                  ? 'bg-wraith-primary text-white'
                  : 'text-slate-400 hover:text-white'
              }`}
              title="List View"
              aria-label="Switch to list view"
            >
              <ListIcon className="w-4 h-4" />
            </button>
            {selectedArticle && (
              <>
                <button
                  onClick={() => setViewMode('editor')}
                  className={`p-2 rounded transition-colors ${
                    viewMode === 'editor'
                      ? 'bg-wraith-primary text-white'
                      : 'text-slate-400 hover:text-white'
                  }`}
                  title="Editor"
                  aria-label="Switch to editor view"
                >
                  <EditIcon className="w-4 h-4" />
                </button>
                <button
                  onClick={() => setViewMode('reader')}
                  className={`p-2 rounded transition-colors ${
                    viewMode === 'reader'
                      ? 'bg-wraith-primary text-white'
                      : 'text-slate-400 hover:text-white'
                  }`}
                  title="Reader"
                  aria-label="Switch to reader view"
                >
                  <BookIcon className="w-4 h-4" />
                </button>
              </>
            )}
          </div>

          {/* Settings button */}
          <button
            onClick={() => setShowSettingsModal(true)}
            className="p-2 text-slate-400 hover:text-white transition-colors"
            title="Settings"
            aria-label="Open settings"
          >
            <SettingsIcon className="w-5 h-5" />
          </button>
        </div>
      </div>
    </header>
  );
}

// Icons
function PublishIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
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

function BookIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
      />
    </svg>
  );
}

function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
      />
    </svg>
  );
}
