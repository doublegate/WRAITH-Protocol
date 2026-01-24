// WRAITH Recon - Scope Panel Component

import { useState } from 'react';
import { useEngagementStore } from '../stores/engagementStore';
import * as tauri from '../lib/tauri';

export function ScopePanel() {
  const { roe, scopeSummary, status } = useEngagementStore();
  const [targetInput, setTargetInput] = useState('');
  const [validationResult, setValidationResult] = useState<{
    in_scope: boolean;
    reason: string;
  } | null>(null);
  const [validating, setValidating] = useState(false);

  const handleValidateTarget = async () => {
    if (!targetInput.trim()) return;

    setValidating(true);
    try {
      const result = await tauri.validateTarget(targetInput.trim());
      setValidationResult(result);
    } catch (e) {
      setValidationResult({
        in_scope: false,
        reason: String(e),
      });
    }
    setValidating(false);
  };

  if (!roe) {
    return (
      <div className="card p-6">
        <div className="empty-state">
          <svg className="empty-state-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <h3 className="empty-state-title">No RoE Loaded</h3>
          <p className="empty-state-description">
            Load a signed Rules of Engagement document to define the scope.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="p-4 border-b border-border-primary">
        <h2 className="text-lg font-semibold text-text-primary flex items-center gap-2">
          <svg className="w-5 h-5 text-primary-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
          </svg>
          Target Scope
        </h2>
      </div>

      <div className="p-4 space-y-4">
        {/* RoE Info */}
        <div className="p-3 rounded-lg bg-bg-tertiary">
          <div className="flex justify-between items-start mb-2">
            <span className="text-sm font-medium text-text-primary">{roe.title}</span>
            <span className={`
              text-xs px-2 py-0.5 rounded-full
              ${status === 'Active' ? 'bg-green-500/20 text-green-400' :
                status === 'Paused' ? 'bg-yellow-500/20 text-yellow-400' :
                'bg-gray-500/20 text-gray-400'}
            `}>
              {status}
            </span>
          </div>
          <p className="text-xs text-text-muted">{roe.organization}</p>
          <div className="mt-2 text-xs text-text-secondary">
            <span>Valid: {new Date(roe.start_time).toLocaleDateString()}</span>
            <span className="mx-2">-</span>
            <span>{new Date(roe.end_time).toLocaleDateString()}</span>
          </div>
        </div>

        {/* Scope Summary */}
        {scopeSummary && (
          <div className="grid grid-cols-2 gap-2">
            <div className="p-2 rounded bg-green-500/10 border border-green-500/30">
              <p className="text-xs text-green-400">Authorized CIDRs</p>
              <p className="text-lg font-bold text-green-400">{scopeSummary.authorized_cidr_count}</p>
            </div>
            <div className="p-2 rounded bg-green-500/10 border border-green-500/30">
              <p className="text-xs text-green-400">Authorized Domains</p>
              <p className="text-lg font-bold text-green-400">{scopeSummary.authorized_domain_count}</p>
            </div>
            <div className="p-2 rounded bg-red-500/10 border border-red-500/30">
              <p className="text-xs text-red-400">Excluded CIDRs</p>
              <p className="text-lg font-bold text-red-400">{scopeSummary.excluded_cidr_count}</p>
            </div>
            <div className="p-2 rounded bg-red-500/10 border border-red-500/30">
              <p className="text-xs text-red-400">Excluded Domains</p>
              <p className="text-lg font-bold text-red-400">{scopeSummary.excluded_domain_count}</p>
            </div>
          </div>
        )}

        {/* Target Validator */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-text-secondary">Validate Target</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={targetInput}
              onChange={(e) => setTargetInput(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleValidateTarget()}
              placeholder="IP, CIDR, or domain..."
              className="input flex-1"
            />
            <button
              onClick={handleValidateTarget}
              disabled={validating || !targetInput.trim()}
              className="btn btn-primary"
            >
              {validating ? 'Checking...' : 'Check'}
            </button>
          </div>

          {validationResult && (
            <div className={`
              p-3 rounded-lg text-sm
              ${validationResult.in_scope
                ? 'bg-green-500/10 border border-green-500/30 text-green-400'
                : 'bg-red-500/10 border border-red-500/30 text-red-400'}
            `}>
              <div className="flex items-center gap-2">
                {validationResult.in_scope ? (
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                  </svg>
                ) : (
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                )}
                <span className="font-medium">
                  {validationResult.in_scope ? 'In Scope' : 'Out of Scope'}
                </span>
              </div>
              <p className="mt-1 text-xs opacity-80">{validationResult.reason}</p>
            </div>
          )}
        </div>

        {/* Authorized Techniques */}
        {roe.authorized_techniques.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-text-secondary mb-2">Authorized Techniques</h4>
            <div className="flex flex-wrap gap-1">
              {roe.authorized_techniques.map((tech) => (
                <span
                  key={tech}
                  className="text-xs px-2 py-1 rounded bg-primary-500/20 text-primary-400 font-mono"
                >
                  {tech}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Prohibited Techniques */}
        {roe.prohibited_techniques.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-text-secondary mb-2">Prohibited Techniques</h4>
            <div className="flex flex-wrap gap-1">
              {roe.prohibited_techniques.map((tech) => (
                <span
                  key={tech}
                  className="text-xs px-2 py-1 rounded bg-red-500/20 text-red-400 font-mono"
                >
                  {tech}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
