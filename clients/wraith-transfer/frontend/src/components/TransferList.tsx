// WRAITH Transfer - Transfer List Component

import { useEffect, useRef, useState } from 'react';
import { useTransferStore } from '../stores/transferStore';
import type { TransferInfo } from '../types';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond === 0) return '0 B/s';
  const k = 1024;
  const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bytesPerSecond) / Math.log(k));
  return parseFloat((bytesPerSecond / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatETA(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '--:--';

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function TransferItem({ transfer }: { transfer: TransferInfo }) {
  const { cancelTransfer } = useTransferStore();
  const progressPercent = Math.round(transfer.progress * 100);

  // Track previous state for speed calculation using state for time initialization
  const prevBytesRef = useRef(transfer.transferred_bytes);
  const [prevTime, setPrevTime] = useState(() => Date.now());
  const [speed, setSpeed] = useState(0);

  useEffect(() => {
    const now = Date.now();
    const timeDiff = (now - prevTime) / 1000; // seconds

    if (timeDiff >= 1.0 && transfer.status === 'in_progress') {
      const bytesDiff = transfer.transferred_bytes - prevBytesRef.current;
      setSpeed(bytesDiff / timeDiff);
      prevBytesRef.current = transfer.transferred_bytes;
      setPrevTime(now);
    }
  }, [transfer.transferred_bytes, transfer.status, prevTime]);

  const remainingBytes = transfer.total_bytes - transfer.transferred_bytes;
  const eta = speed > 0 ? remainingBytes / speed : 0;

  const statusColors: Record<string, string> = {
    initializing: 'text-yellow-500',
    in_progress: 'text-blue-500',
    completed: 'text-green-500',
    failed: 'text-red-500',
    cancelled: 'text-slate-500',
  };

  const isActive = transfer.status === 'initializing' || transfer.status === 'in_progress';

  return (
    <div className="bg-bg-secondary rounded-lg p-4 border border-slate-700">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-3">
          <div className={`text-lg ${transfer.direction === 'upload' ? 'rotate-180' : ''}`}>
            {transfer.direction === 'upload' ? '↑' : '↓'}
          </div>
          <div>
            <div className="font-medium text-white">{transfer.file_name}</div>
            <div className="text-sm text-slate-400 font-mono">
              {transfer.peer_id.slice(0, 16)}...
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <span className={`text-sm ${statusColors[transfer.status] || 'text-slate-400'}`}>
            {transfer.status.replace('_', ' ')}
          </span>

          {isActive && (
            <button
              onClick={() => cancelTransfer(transfer.id)}
              className="text-slate-400 hover:text-red-500 transition-colors"
              title="Cancel transfer"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      <div className="space-y-1">
        <div className="flex justify-between text-sm text-slate-400">
          <span>
            {formatBytes(transfer.transferred_bytes)} / {formatBytes(transfer.total_bytes)}
          </span>
          <span>{progressPercent}%</span>
        </div>

        <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${
              transfer.status === 'completed'
                ? 'bg-green-500'
                : transfer.status === 'failed'
                ? 'bg-red-500'
                : 'bg-wraith-accent'
            }`}
            style={{ width: `${progressPercent}%` }}
          />
        </div>

        {isActive && (
          <div className="flex justify-between text-xs text-slate-500">
            <span>
              {speed > 0 ? formatSpeed(speed) : 'Calculating...'}
            </span>
            <span>
              ETA: {eta > 0 && isFinite(eta) ? formatETA(eta) : '--:--'}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

export function TransferList() {
  const { transfers } = useTransferStore();

  if (transfers.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-slate-500">
        <div className="text-center">
          <div className="text-4xl mb-2">↔</div>
          <div>No active transfers</div>
          <div className="text-sm">Start a transfer to see it here</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-4 space-y-3">
      {transfers.map((transfer) => (
        <TransferItem key={transfer.id} transfer={transfer} />
      ))}
    </div>
  );
}
