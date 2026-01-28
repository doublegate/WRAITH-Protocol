import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../stores/appStore';
import type { StreamEvent } from '../types';
import { Activity, Radio } from 'lucide-react';

const eventTypeColors: Record<string, string> = {
  implant_checkin: 'text-green-400',
  command_complete: 'text-blue-400',
  command_failed: 'text-red-400',
  implant_registered: 'text-yellow-400',
  implant_killed: 'text-red-500',
  listener_started: 'text-cyan-400',
  listener_stopped: 'text-orange-400',
};

function getEventColor(type: string): string {
  return eventTypeColors[type] || 'text-slate-400';
}

function formatTimestamp(): string {
  return new Date().toLocaleTimeString('en-US', { hour12: false });
}

export function EventLogWidget() {
  const events = useAppStore((s) => s.events);
  const recentEvents = events.slice(0, 8);

  return (
    <div className="rounded border border-slate-800 bg-slate-900 overflow-hidden h-full flex flex-col">
      <div className="px-3 py-2 border-b border-slate-800 bg-slate-950 flex items-center gap-2">
        <Radio className="w-3 h-3 text-red-500" />
        <span className="text-[10px] text-slate-500 font-bold uppercase tracking-wider">
          Live Events
        </span>
        <span className="ml-auto text-[10px] text-slate-600">{events.length} total</span>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {recentEvents.length === 0 ? (
          <div className="text-center text-[10px] text-slate-600 py-4 italic">
            Waiting for events...
          </div>
        ) : (
          recentEvents.map((ev) => (
            <div
              key={ev.id}
              className="flex items-start gap-2 text-[10px] py-1 border-b border-slate-800/50 last:border-0"
            >
              <span className="text-slate-600 font-mono shrink-0">{formatTimestamp()}</span>
              <span className={`font-bold uppercase shrink-0 ${getEventColor(ev.type)}`}>
                {ev.type.replace(/_/g, ' ')}
              </span>
              {ev.implant_id && (
                <span className="text-slate-500 font-mono truncate">
                  {ev.implant_id.substring(0, 8)}
                </span>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default function EventLog() {
  const { events, addEvent } = useAppStore();

  useEffect(() => {
    const unlisten = listen<StreamEvent>('server-event', (event) => {
      addEvent(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addEvent]);

  return (
    <div className="flex flex-col h-full">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-bold text-white uppercase tracking-wider flex items-center gap-2">
          <Activity className="w-4 h-4 text-red-500" /> Event Log
        </h2>
        <span className="text-[10px] text-slate-500">{events.length} events</span>
      </div>

      <div className="flex-1 rounded border border-slate-800 bg-slate-900 overflow-hidden">
        <table className="w-full text-left text-xs">
          <thead className="border-b border-slate-800 bg-slate-950 text-slate-500 sticky top-0">
            <tr>
              <th className="px-4 py-2 font-medium w-20">TIME</th>
              <th className="px-4 py-2 font-medium w-36">EVENT</th>
              <th className="px-4 py-2 font-medium w-24">IMPLANT</th>
              <th className="px-4 py-2 font-medium">DATA</th>
            </tr>
          </thead>
          <tbody className="overflow-y-auto">
            {events.length === 0 ? (
              <tr>
                <td className="px-4 py-12 text-center text-slate-600 italic" colSpan={4}>
                  No events recorded. Events will appear once the server stream is active.
                </td>
              </tr>
            ) : (
              events.map((ev) => (
                <tr
                  key={ev.id}
                  className="border-b border-slate-800/50 hover:bg-slate-800/50 transition-colors"
                >
                  <td className="px-4 py-2 font-mono text-slate-500">{formatTimestamp()}</td>
                  <td className="px-4 py-2">
                    <span className={`font-bold uppercase ${getEventColor(ev.type)}`}>
                      {ev.type.replace(/_/g, ' ')}
                    </span>
                  </td>
                  <td className="px-4 py-2 font-mono text-slate-400">
                    {ev.implant_id ? ev.implant_id.substring(0, 8) : '-'}
                  </td>
                  <td className="px-4 py-2 text-slate-500 truncate max-w-xs">
                    {Object.entries(ev.data || {})
                      .map(([k, v]) => `${k}=${v}`)
                      .join(', ') || '-'}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
