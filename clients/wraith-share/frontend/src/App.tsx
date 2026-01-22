// Main App Component - WRAITH Share

import { useEffect, useState } from 'react';
import MainLayout from './components/layout/MainLayout';
import FileBrowser from './components/files/FileBrowser';
import MemberList from './components/groups/MemberList';
import FileVersions from './components/files/FileVersions';
import CreateGroupModal from './components/groups/CreateGroupModal';
import InviteMemberModal from './components/groups/InviteMemberModal';
import FileUploadModal from './components/files/FileUpload';
import ShareLinkModal from './components/files/ShareLinkModal';
import { useGroupStore } from './stores/groupStore';
import { useFileStore } from './stores/fileStore';
import { useUiStore } from './stores/uiStore';

type TabId = 'files' | 'members' | 'versions';

export default function App() {
  const [activeTab, setActiveTab] = useState<TabId>('files');

  const { fetchGroups, selectedGroupId, groupInfos } = useGroupStore();
  const { fetchFiles, selectedFileId } = useFileStore();
  const { fetchIdentity } = useUiStore();

  // Initialize app
  useEffect(() => {
    (async () => {
      await fetchIdentity();
      await fetchGroups();
    })();
  }, [fetchIdentity, fetchGroups]);

  // Fetch files when group changes
  useEffect(() => {
    if (selectedGroupId) {
      fetchFiles(selectedGroupId);
    }
  }, [selectedGroupId, fetchFiles]);

  // Periodic refresh
  useEffect(() => {
    const interval = setInterval(() => {
      fetchGroups();
      if (selectedGroupId) {
        fetchFiles(selectedGroupId);
      }
    }, 30000); // Refresh every 30 seconds

    return () => clearInterval(interval);
  }, [fetchGroups, fetchFiles, selectedGroupId]);

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;

  const tabs: { id: TabId; label: string; show: boolean }[] = [
    { id: 'files', label: 'Files', show: true },
    { id: 'members', label: 'Members', show: !!selectedGroupId },
    { id: 'versions', label: 'Versions', show: !!selectedFileId },
  ];

  return (
    <MainLayout>
      {selectedGroupId ? (
        <div className="flex flex-col h-full">
          {/* Group header */}
          <div className="bg-slate-800 border-b border-slate-700 px-4 py-3">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold text-white">
                  {groupInfo?.name || 'Loading...'}
                </h2>
                {groupInfo?.description && (
                  <p className="text-sm text-slate-400">{groupInfo.description}</p>
                )}
              </div>
              <div className="flex items-center gap-2 text-sm text-slate-400">
                <span>{groupInfo?.member_count || 0} members</span>
                <span className="text-slate-600">|</span>
                <span>{groupInfo?.file_count || 0} files</span>
              </div>
            </div>
          </div>

          {/* Tab navigation */}
          <nav className="bg-slate-800 border-b border-slate-700">
            <div className="flex px-4">
              {tabs
                .filter((t) => t.show)
                .map((tab) => (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className={`px-4 py-2 text-sm font-medium transition-colors relative ${
                      activeTab === tab.id
                        ? 'text-violet-400'
                        : 'text-slate-400 hover:text-white'
                    }`}
                  >
                    {tab.label}
                    {activeTab === tab.id && (
                      <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-violet-500" />
                    )}
                  </button>
                ))}
            </div>
          </nav>

          {/* Tab content */}
          <div className="flex-1 overflow-hidden">
            {activeTab === 'files' && <FileBrowser />}
            {activeTab === 'members' && (
              <div className="p-4 overflow-auto h-full">
                <MemberList />
              </div>
            )}
            {activeTab === 'versions' && <FileVersions />}
          </div>
        </div>
      ) : (
        <WelcomeScreen />
      )}

      {/* Modals */}
      <CreateGroupModal />
      <InviteMemberModal />
      <FileUploadModal />
      <ShareLinkModal />
    </MainLayout>
  );
}

function WelcomeScreen() {
  const { openModal } = useUiStore();
  const { groups } = useGroupStore();

  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <div className="text-center max-w-md">
        <svg
          className="w-24 h-24 mx-auto text-slate-600 mb-6"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1}
            d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
          />
        </svg>

        <h1 className="text-2xl font-bold text-white mb-2">
          Welcome to WRAITH Share
        </h1>
        <p className="text-slate-400 mb-8">
          Secure, end-to-end encrypted file sharing with granular access control.
          Share files with groups using the WRAITH protocol.
        </p>

        {groups.length === 0 ? (
          <div className="space-y-4">
            <button
              onClick={() => openModal('createGroup')}
              className="w-full px-6 py-3 bg-violet-600 hover:bg-violet-700 text-white rounded-lg font-medium transition-colors"
            >
              Create Your First Group
            </button>
            <p className="text-sm text-slate-500">
              Or select an existing group from the sidebar
            </p>
          </div>
        ) : (
          <p className="text-slate-400">
            Select a group from the sidebar to view shared files
          </p>
        )}

        {/* Features */}
        <div className="mt-12 grid grid-cols-3 gap-4 text-left">
          <Feature
            icon={
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            }
            title="End-to-End Encryption"
            description="All files encrypted before leaving your device"
          />
          <Feature
            icon={
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
              </svg>
            }
            title="Group Access Control"
            description="Fine-grained permissions per member"
          />
          <Feature
            icon={
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
              </svg>
            }
            title="Share Links"
            description="Secure links with expiration and passwords"
          />
        </div>
      </div>
    </div>
  );
}

function Feature({
  icon,
  title,
  description,
}: {
  icon: JSX.Element;
  title: string;
  description: string;
}) {
  return (
    <div className="p-3">
      <div className="text-violet-400 mb-2">{icon}</div>
      <h3 className="text-sm font-medium text-white mb-1">{title}</h3>
      <p className="text-xs text-slate-500">{description}</p>
    </div>
  );
}
