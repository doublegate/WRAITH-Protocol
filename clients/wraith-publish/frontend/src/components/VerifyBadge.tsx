interface VerifyBadgeProps {
  isVerified: boolean | null;
  isVerifying: boolean;
  onVerify?: () => void;
}

export function VerifyBadge({ isVerified, isVerifying, onVerify }: VerifyBadgeProps) {
  if (isVerifying) {
    return (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-slate-600 rounded-full text-xs text-white">
        <SpinnerIcon className="w-3 h-3 animate-spin" />
        Verifying...
      </span>
    );
  }

  if (isVerified === null) {
    return (
      <button
        onClick={onVerify}
        className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-slate-600 hover:bg-slate-500 rounded-full text-xs text-white transition-colors"
      >
        <QuestionIcon className="w-3 h-3" />
        Verify
      </button>
    );
  }

  if (isVerified) {
    return (
      <span
        className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-green-600/20 border border-green-500/50 rounded-full text-xs text-green-400"
        title="Content signature verified"
      >
        <CheckIcon className="w-3 h-3" />
        Verified
      </span>
    );
  }

  return (
    <span
      className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-red-600/20 border border-red-500/50 rounded-full text-xs text-red-400"
      title="Content signature invalid or tampered"
    >
      <WarningIcon className="w-3 h-3" />
      Unverified
    </span>
  );
}

// Compact badge variant for cards
export function VerifyBadgeCompact({
  isVerified,
}: {
  isVerified: boolean | null;
}) {
  if (isVerified === null) {
    return null;
  }

  if (isVerified) {
    return (
      <span
        className="inline-flex items-center justify-center w-5 h-5 bg-green-600/20 border border-green-500/50 rounded-full"
        title="Verified"
      >
        <CheckIcon className="w-3 h-3 text-green-400" />
      </span>
    );
  }

  return (
    <span
      className="inline-flex items-center justify-center w-5 h-5 bg-red-600/20 border border-red-500/50 rounded-full"
      title="Unverified"
    >
      <WarningIcon className="w-3 h-3 text-red-400" />
    </span>
  );
}

// Icons
function SpinnerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24">
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M5 13l4 4L19 7"
      />
    </svg>
  );
}

function WarningIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
      />
    </svg>
  );
}

function QuestionIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}
