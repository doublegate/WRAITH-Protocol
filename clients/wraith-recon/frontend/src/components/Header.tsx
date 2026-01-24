// WRAITH Recon - Header Component

import { useEngagementStore } from '../stores/engagementStore';
import { useNodeStore } from '../stores/nodeStore';

interface HeaderProps {
  onOpenSettings: () => void;
  onOpenRoe: () => void;
}

export function Header({ onOpenSettings, onOpenRoe }: HeaderProps) {
  const { status, engagementId, roe } = useEngagementStore();
  const { status: nodeStatus } = useNodeStore();

  const getStatusColor = () => {
    switch (status) {
      case 'Active':
        return 'bg-green-500';
      case 'Paused':
        return 'bg-yellow-500';
      case 'Terminated':
        return 'bg-red-500';
      case 'Ready':
        return 'bg-blue-500';
      case 'Completed':
        return 'bg-gray-500';
      default:
        return 'bg-gray-600';
    }
  };

  const getStatusText = () => {
    switch (status) {
      case 'NotLoaded':
        return 'No RoE Loaded';
      case 'Ready':
        return 'Ready';
      case 'Active':
        return 'Engagement Active';
      case 'Paused':
        return 'Paused';
      case 'Completed':
        return 'Completed';
      case 'Terminated':
        return 'TERMINATED';
      default:
        return status;
    }
  };

  return (
    <header className="header justify-between">
      <div className="flex items-center gap-4">
        {/* Logo and Title */}
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-red-500 to-orange-600 flex items-center justify-center">
            <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </div>
          <div>
            <h1 className="text-lg font-bold text-text-primary">WRAITH Recon</h1>
            <p className="text-xs text-text-muted">Security Assessment Platform</p>
          </div>
        </div>

        {/* Engagement Info */}
        {roe && (
          <div className="ml-6 px-4 py-1 rounded-lg bg-bg-tertiary border border-border-primary">
            <span className="text-xs text-text-muted">RoE: </span>
            <span className="text-sm text-text-primary font-medium">{roe.title}</span>
            {engagementId && (
              <>
                <span className="text-xs text-text-muted ml-3">ID: </span>
                <span className="text-sm text-text-secondary font-mono">
                  {engagementId.slice(0, 8)}...
                </span>
              </>
            )}
          </div>
        )}
      </div>

      <div className="flex items-center gap-4">
        {/* Status Indicators */}
        <div className="flex items-center gap-3">
          {/* Engagement Status */}
          <div className="flex items-center gap-2">
            <div className={`w-2.5 h-2.5 rounded-full ${getStatusColor()}`} />
            <span className="text-sm text-text-secondary">{getStatusText()}</span>
          </div>

          {/* Node Status */}
          <div className="flex items-center gap-2 ml-3">
            <div className={`w-2.5 h-2.5 rounded-full ${nodeStatus?.running ? 'bg-green-500' : 'bg-gray-500'}`} />
            <span className="text-sm text-text-secondary">
              Node {nodeStatus?.running ? 'Online' : 'Offline'}
            </span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-2">
          {/* Load RoE Button */}
          <button
            onClick={onOpenRoe}
            className="btn btn-secondary text-sm"
            title="Load Rules of Engagement"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            RoE
          </button>

          {/* Settings Button */}
          <button
            onClick={onOpenSettings}
            className="p-2 rounded-lg hover:bg-bg-hover transition-colors"
            title="Settings"
          >
            <svg className="w-5 h-5 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>
    </header>
  );
}
