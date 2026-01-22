// MainLayout Component - Main application layout

import { ReactNode } from 'react';
import Sidebar from './Sidebar';
import Header from './Header';
import ActivityFeed from '../activity/ActivityFeed';
import ToastContainer from '../ui/Toast';
import { useUiStore } from '../../stores/uiStore';

interface MainLayoutProps {
  children: ReactNode;
}

export default function MainLayout({ children }: MainLayoutProps) {
  const { showActivityPanel } = useUiStore();

  return (
    <div className="flex h-screen bg-slate-900 text-slate-200">
      {/* Sidebar */}
      <Sidebar />

      {/* Main content area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <Header />

        {/* Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Main content */}
          <main className="flex-1 overflow-auto">{children}</main>

          {/* Activity panel */}
          {showActivityPanel && (
            <aside className="w-80 border-l border-slate-700 bg-slate-800 overflow-hidden flex flex-col">
              <ActivityFeed />
            </aside>
          )}
        </div>
      </div>

      {/* Toast notifications */}
      <ToastContainer />
    </div>
  );
}
