import { useState, useEffect } from 'react';
import { Button } from './Button';

interface ModalProps {
  open: boolean;
  title: string;
  message: string;
  placeholder?: string;
  defaultValue?: string;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}

export function Modal({
  open,
  title,
  message,
  placeholder = '',
  defaultValue = '',
  onSubmit,
  onCancel,
}: ModalProps) {
  const [value, setValue] = useState(defaultValue);

  useEffect(() => {
    if (open) setValue(defaultValue);
  }, [open, defaultValue]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-sm rounded border border-slate-700 bg-slate-900 p-6 shadow-2xl">
        <h3 className="text-sm font-bold text-white uppercase tracking-wider mb-2">{title}</h3>
        <p className="text-xs text-slate-400 mb-4">{message}</p>
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder}
          className="w-full bg-slate-950 border border-slate-700 p-2 text-sm text-white focus:outline-none focus:border-red-500 rounded mb-4"
          autoFocus
          onKeyDown={(e) => {
            if (e.key === 'Enter' && value.trim()) onSubmit(value.trim());
            if (e.key === 'Escape') onCancel();
          }}
        />
        <div className="flex justify-end gap-2">
          <Button variant="secondary" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" onClick={() => value.trim() && onSubmit(value.trim())}>
            Submit
          </Button>
        </div>
      </div>
    </div>
  );
}
