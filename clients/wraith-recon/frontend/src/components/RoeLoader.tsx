// WRAITH Recon - RoE Loader Component

import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useEngagementStore } from '../stores/engagementStore';
import * as tauri from '../lib/tauri';

interface RoeLoaderProps {
  isOpen: boolean;
  onClose: () => void;
}

export function RoeLoader({ isOpen, onClose }: RoeLoaderProps) {
  const { loadRoeFromFile, loading, error, clearError } = useEngagementStore();
  const [filePath, setFilePath] = useState('');
  const [validationResult, setValidationResult] = useState<{
    valid: boolean;
    errors: string[];
  } | null>(null);

  const handleBrowse = async () => {
    try {
      const selected = await open({
        filters: [{
          name: 'RoE Files',
          extensions: ['json', 'yaml', 'yml', 'toml'],
        }],
        multiple: false,
      });

      if (selected && typeof selected === 'string') {
        setFilePath(selected);
        setValidationResult(null);
      }
    } catch (e) {
      console.error('File dialog error:', e);
    }
  };

  const handleValidate = async () => {
    if (!filePath) return;

    try {
      // First load the RoE to get the object
      await loadRoeFromFile(filePath);
      const roe = await tauri.getRoe();

      if (roe) {
        const result = await tauri.validateRoe(roe);
        setValidationResult(result);
      }
    } catch (e) {
      setValidationResult({
        valid: false,
        errors: [String(e)],
      });
    }
  };

  const handleLoad = async () => {
    if (!filePath) return;

    try {
      await loadRoeFromFile(filePath);
      onClose();
    } catch (e) {
      console.error('Failed to load RoE:', e);
    }
  };

  const handleClose = () => {
    setFilePath('');
    setValidationResult(null);
    clearError();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal-content w-[500px] p-6" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-xl font-bold text-text-primary mb-4 flex items-center gap-2">
          <svg className="w-6 h-6 text-primary-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          Load Rules of Engagement
        </h2>

        <p className="text-text-secondary mb-4">
          Select a signed Rules of Engagement document. The RoE must be valid and cryptographically signed
          to begin an authorized security assessment.
        </p>

        {/* Error Display */}
        {error && (
          <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
            {error}
          </div>
        )}

        {/* File Selection */}
        <div className="mb-4">
          <label className="block text-sm font-medium text-text-secondary mb-2">
            RoE File Path
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={filePath}
              onChange={(e) => setFilePath(e.target.value)}
              placeholder="/path/to/roe.json"
              className="input flex-1 font-mono text-sm"
            />
            <button onClick={handleBrowse} className="btn btn-secondary">
              Browse
            </button>
          </div>
        </div>

        {/* Validation Result */}
        {validationResult && (
          <div className={`
            mb-4 p-3 rounded-lg border
            ${validationResult.valid
              ? 'bg-green-500/10 border-green-500/30'
              : 'bg-red-500/10 border-red-500/30'}
          `}>
            <div className="flex items-center gap-2 mb-2">
              {validationResult.valid ? (
                <>
                  <svg className="w-5 h-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                  </svg>
                  <span className="font-medium text-green-400">RoE Validated Successfully</span>
                </>
              ) : (
                <>
                  <svg className="w-5 h-5 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                  <span className="font-medium text-red-400">Validation Failed</span>
                </>
              )}
            </div>

            {validationResult.errors.length > 0 && (
              <ul className="text-sm space-y-1">
                {validationResult.errors.map((err, i) => (
                  <li key={i} className={validationResult.valid ? 'text-yellow-400' : 'text-red-400'}>
                    {err}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {/* Security Notice */}
        <div className="mb-4 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/30">
          <div className="flex items-start gap-2">
            <svg className="w-5 h-5 text-yellow-400 flex-shrink-0 mt-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            <div className="text-sm text-yellow-300">
              <p className="font-medium mb-1">Security Notice</p>
              <p className="text-yellow-300/80">
                Only proceed with authorized security testing. All operations will be logged
                with cryptographic signatures for accountability.
              </p>
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="flex gap-3">
          <button onClick={handleClose} className="btn btn-secondary flex-1">
            Cancel
          </button>
          {!validationResult?.valid && filePath && (
            <button
              onClick={handleValidate}
              disabled={loading || !filePath}
              className="btn btn-secondary flex-1"
            >
              Validate
            </button>
          )}
          <button
            onClick={handleLoad}
            disabled={loading || !filePath}
            className="btn btn-primary flex-1"
          >
            {loading ? 'Loading...' : 'Load RoE'}
          </button>
        </div>
      </div>
    </div>
  );
}
