import { useToastStore, type ToastType } from '../../stores/toastStore';
import { X, CheckCircle, AlertTriangle, Info, XCircle } from 'lucide-react';

const iconMap: Record<ToastType, React.ReactNode> = {
  success: <CheckCircle className="w-4 h-4 text-green-400" />,
  error: <XCircle className="w-4 h-4 text-red-400" />,
  warning: <AlertTriangle className="w-4 h-4 text-yellow-400" />,
  info: <Info className="w-4 h-4 text-blue-400" />,
};

const bgMap: Record<ToastType, string> = {
  success: 'border-green-900/50 bg-green-950/80',
  error: 'border-red-900/50 bg-red-950/80',
  warning: 'border-yellow-900/50 bg-yellow-950/80',
  info: 'border-blue-900/50 bg-blue-950/80',
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`flex items-start gap-2 rounded border px-3 py-2 text-xs text-slate-200 shadow-lg backdrop-blur-sm animate-in fade-in slide-in-from-bottom-2 ${bgMap[t.type]}`}
        >
          <span className="mt-0.5 shrink-0">{iconMap[t.type]}</span>
          <span className="flex-1 break-words">{t.message}</span>
          <button
            onClick={() => removeToast(t.id)}
            className="shrink-0 text-slate-500 hover:text-white transition-colors"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      ))}
    </div>
  );
}
