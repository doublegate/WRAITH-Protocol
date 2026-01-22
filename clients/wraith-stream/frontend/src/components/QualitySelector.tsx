import { usePlayerStore } from '../stores/playerStore';

interface QualitySelectorProps {
  onClose: () => void;
}

export default function QualitySelector({ onClose }: QualitySelectorProps) {
  const { player, playbackInfo, setQuality } = usePlayerStore();

  if (!playbackInfo) return null;

  const handleSelectQuality = async (qualityName: string) => {
    await setQuality(qualityName);
    onClose();
  };

  return (
    <div className="absolute bottom-full right-0 mb-2 quality-selector min-w-[160px]">
      <div className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider px-3 py-2">
        Quality
      </div>

      {/* Auto option */}
      <button
        className={`quality-option w-full text-left ${player.quality === 'auto' ? 'active' : ''}`}
        onClick={() => handleSelectQuality('auto')}
      >
        <span className="text-sm">Auto</span>
      </button>

      {/* Quality options */}
      {playbackInfo.qualities
        .slice()
        .reverse()
        .map((quality) => (
          <button
            key={quality.name}
            className={`quality-option w-full text-left ${player.quality === quality.name ? 'active' : ''}`}
            onClick={() => handleSelectQuality(quality.name)}
          >
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm font-medium">{quality.name}</span>
              <span className="text-xs text-[var(--color-text-muted)]">
                {quality.width}x{quality.height}
              </span>
            </div>
          </button>
        ))}
    </div>
  );
}
