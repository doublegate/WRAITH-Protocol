// WRAITH Recon - Audit Viewer Component

import { useEffect, useState } from 'react';
import { useAuditStore } from '../stores/auditStore';

export function AuditViewer() {
  const {
    entries, chainValid, chainVerificationResult, databaseStats,
    fetchEntries, verifyChain, exportLog, addNote,
    loading, error,
  } = useAuditStore();

  const [noteText, setNoteText] = useState('');
  const [showVerification, setShowVerification] = useState(false);

  useEffect(() => {
    fetchEntries(0, 50);
  }, [fetchEntries]);

  const handleAddNote = async () => {
    if (!noteText.trim()) return;
    await addNote(noteText.trim());
    setNoteText('');
  };

  const handleVerify = async () => {
    await verifyChain();
    setShowVerification(true);
  };

  const handleExport = async () => {
    try {
      const path = await exportLog();
      alert(`Audit log exported to: ${path}`);
    } catch (e) {
      console.error('Export failed:', e);
    }
  };

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'Info':
        return 'text-blue-400 bg-blue-500/10';
      case 'Warning':
        return 'text-yellow-400 bg-yellow-500/10';
      case 'Error':
        return 'text-red-400 bg-red-500/10';
      case 'Emergency':
        return 'text-red-500 bg-red-600/20';
      default:
        return 'text-gray-400 bg-gray-500/10';
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'System':
        return 'M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z';
      case 'RulesOfEngagement':
        return 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z';
      case 'KillSwitch':
        return 'M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636';
      case 'Reconnaissance':
        return 'M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z';
      case 'Channel':
        return 'M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z';
      case 'DataTransfer':
        return 'M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12';
      default:
        return 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z';
    }
  };

  return (
    <div className="card flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b border-border-primary flex justify-between items-center">
        <h2 className="text-lg font-semibold text-text-primary flex items-center gap-2">
          <svg className="w-5 h-5 text-primary-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01" />
          </svg>
          Audit Log
          {chainValid !== null && (
            <span className={`
              text-xs px-2 py-0.5 rounded-full
              ${chainValid ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}
            `}>
              {chainValid ? 'Chain Valid' : 'Chain Invalid'}
            </span>
          )}
        </h2>
        <div className="flex gap-2">
          <button
            onClick={handleVerify}
            disabled={loading}
            className="btn btn-secondary text-sm"
          >
            Verify Chain
          </button>
          <button
            onClick={handleExport}
            disabled={loading}
            className="btn btn-secondary text-sm"
          >
            Export
          </button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="p-3 m-4 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Verification Result */}
      {showVerification && chainVerificationResult && (
        <div className={`
          p-3 m-4 rounded-lg border text-sm
          ${chainVerificationResult.valid
            ? 'bg-green-500/10 border-green-500/30 text-green-400'
            : 'bg-red-500/10 border-red-500/30 text-red-400'}
        `}>
          <div className="flex items-center justify-between">
            <span className="font-medium">
              {chainVerificationResult.valid
                ? 'Audit chain integrity verified'
                : 'Audit chain integrity compromised'}
            </span>
            <button
              onClick={() => setShowVerification(false)}
              className="text-text-muted hover:text-text-primary"
            >
              x
            </button>
          </div>
          <p className="mt-1 text-xs opacity-80">
            Verified {chainVerificationResult.entries_verified} entries
            {chainVerificationResult.first_invalid_sequence && (
              <span> - First invalid at sequence #{chainVerificationResult.first_invalid_sequence}</span>
            )}
          </p>
          {chainVerificationResult.errors.length > 0 && (
            <ul className="mt-2 text-xs list-disc list-inside">
              {chainVerificationResult.errors.map((err, i) => (
                <li key={i}>{err}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {/* Stats */}
      {databaseStats && (
        <div className="px-4 py-2 border-b border-border-primary flex gap-4 text-xs text-text-muted">
          <span>{databaseStats.audit_entries} entries</span>
          <span>{(databaseStats.db_size_bytes / 1024).toFixed(1)} KB</span>
        </div>
      )}

      {/* Entry List */}
      <div className="flex-1 overflow-auto p-4">
        {entries.length === 0 ? (
          <div className="empty-state">
            <svg className="empty-state-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
            <h3 className="empty-state-title">No Audit Entries</h3>
            <p className="empty-state-description">
              Audit entries will appear here as operations are performed.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {entries.map((entry) => (
              <div key={entry.id} className="p-3 rounded-lg bg-bg-tertiary">
                <div className="flex items-start gap-3">
                  {/* Icon */}
                  <div className={`p-1.5 rounded ${getLevelColor(entry.level)}`}>
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={getCategoryIcon(entry.category)} />
                    </svg>
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`text-xs px-1.5 py-0.5 rounded ${getLevelColor(entry.level)}`}>
                        {entry.level}
                      </span>
                      <span className="text-xs text-text-muted">{entry.category}</span>
                      <span className="text-xs text-text-muted ml-auto">
                        #{entry.sequence}
                      </span>
                    </div>

                    <p className="text-sm text-text-primary">{entry.summary}</p>

                    {entry.details && (
                      <p className="mt-1 text-xs text-text-secondary">{entry.details}</p>
                    )}

                    {entry.mitre_technique && (
                      <div className="mt-1.5 inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-primary-500/10 text-primary-400">
                        <span className="font-mono">{entry.mitre_technique}</span>
                        {entry.mitre_tactic && (
                          <>
                            <span className="text-text-muted">|</span>
                            <span>{entry.mitre_tactic}</span>
                          </>
                        )}
                      </div>
                    )}

                    <div className="mt-2 flex items-center gap-3 text-xs text-text-muted">
                      <span>{new Date(entry.timestamp).toLocaleString()}</span>
                      <span>Operator: {entry.operator_id}</span>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Add Note */}
      <div className="p-4 border-t border-border-primary">
        <div className="flex gap-2">
          <input
            type="text"
            value={noteText}
            onChange={(e) => setNoteText(e.target.value)}
            placeholder="Add operator note..."
            className="input flex-1 text-sm"
            onKeyDown={(e) => e.key === 'Enter' && handleAddNote()}
          />
          <button
            onClick={handleAddNote}
            disabled={!noteText.trim() || loading}
            className="btn btn-primary text-sm"
          >
            Add Note
          </button>
        </div>
      </div>
    </div>
  );
}
