import { useEffect } from 'react';
import { useAppStore } from '../stores/appStore';

const TAB_MAP: Record<string, string> = {
  '1': 'dashboard',
  '2': 'campaigns',
  '3': 'attack chains',
  '4': 'beacons',
  '5': 'listeners',
  '6': 'loot',
  '7': 'phishing',
  '8': 'generator',
  '9': 'events',
  '0': 'settings',
};

export function useKeyboardShortcuts() {
  const { setActiveTab, refreshAll } = useAppStore();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ignore when typing in inputs
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      if (e.ctrlKey || e.metaKey) {
        const tab = TAB_MAP[e.key];
        if (tab) {
          e.preventDefault();
          setActiveTab(tab);
          return;
        }

        if (e.key === 'r') {
          e.preventDefault();
          refreshAll();
          return;
        }
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [setActiveTab, refreshAll]);
}
