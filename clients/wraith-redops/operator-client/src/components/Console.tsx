import { useEffect, useRef } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { invoke } from '@tauri-apps/api/core';

interface ConsoleProps {
  implantId: string;
}

export const Console = ({ implantId }: ConsoleProps) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      theme: {
        background: '#0f172a', // slate-900
        foreground: '#cbd5e1', // slate-300
        cursor: '#ef4444', // red-500
      },
      fontFamily: 'JetBrains Mono, monospace',
      fontSize: 13,
      cursorBlink: true,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalRef.current);
    fitAddon.fit();

    terminal.writeln(`[1;31mWRAITH::REDOPS[0m Interactive Console`);
    terminal.writeln(`Attached to beacon: [33m${implantId}[0m`);
    terminal.write('\r\n$ ');

    let command = '';
    terminal.onData(async (data) => {
      if (data === '\r') {
        terminal.write('\r\n');
        if (command.trim()) {
          try {
            await invoke('send_command', {
              implantId,
              commandType: 'shell',
              payload: command,
            });
            terminal.writeln(`Sent: ${command}`);
          } catch (e) {
            terminal.writeln(`[31mError:[0m ${e}`);
          }
        }
        command = '';
        terminal.write('$ ');
      } else if (data === '\u007f') { // Backspace
        if (command.length > 0) {
          command = command.slice(0, -1);
          terminal.write('\b \b');
        }
      } else {
        command += data;
        terminal.write(data);
      }
    });

    xtermRef.current = terminal;

    return () => {
      terminal.dispose();
    };
  }, [implantId]);

  return (
    <div className="flex flex-col h-full overflow-hidden rounded border border-slate-800 bg-slate-900">
      <div className="flex items-center justify-between px-3 py-1 bg-slate-800 text-[10px] text-slate-400 uppercase tracking-wider">
        <span>Beacon Console: {implantId.substring(0, 8)}</span>
        <div className="flex gap-1">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
          <span>Online</span>
        </div>
      </div>
      <div ref={terminalRef} className="flex-1 p-2 overflow-hidden" />
    </div>
  );
};
