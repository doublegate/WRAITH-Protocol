import { useEffect, useRef, useCallback } from 'react';
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
  const commandHistoryRef = useRef<string[]>([]);
  const historyIndexRef = useRef<number>(-1);
  const currentCommandRef = useRef<string>('');

  const clearLine = useCallback((terminal: Terminal, command: string) => {
    // Move cursor to start of line and clear
    for (let i = 0; i < command.length; i++) {
      terminal.write('\b \b');
    }
  }, []);

  useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      theme: {
        background: '#0f172a', // slate-900
        foreground: '#cbd5e1', // slate-300
        cursor: '#ef4444', // red-500
        selectionBackground: '#334155', // slate-700
      },
      fontFamily: 'JetBrains Mono, monospace',
      fontSize: 13,
      cursorBlink: true,
      scrollback: 1000,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalRef.current);
    fitAddon.fit();

    // Handle window resize
    const handleResize = () => fitAddon.fit();
    window.addEventListener('resize', handleResize);

    terminal.writeln(`\x1b[1;31mWRAITH::REDOPS\x1b[0m Interactive Console`);
    terminal.writeln(`Attached to beacon: \x1b[33m${implantId}\x1b[0m`);
    terminal.writeln(`\x1b[90mType 'help' for available commands. Use Up/Down for history.\x1b[0m`);
    terminal.write('\r\n$ ');

    let command = '';

    terminal.onData(async (data) => {
      // Handle special key sequences
      if (data === '\x1b[A') { // Up arrow - previous command
        if (commandHistoryRef.current.length > 0) {
          if (historyIndexRef.current === -1) {
            currentCommandRef.current = command; // Save current input
            historyIndexRef.current = commandHistoryRef.current.length - 1;
          } else if (historyIndexRef.current > 0) {
            historyIndexRef.current--;
          }
          clearLine(terminal, command);
          command = commandHistoryRef.current[historyIndexRef.current];
          terminal.write(command);
        }
        return;
      }

      if (data === '\x1b[B') { // Down arrow - next command
        if (historyIndexRef.current !== -1) {
          clearLine(terminal, command);
          if (historyIndexRef.current < commandHistoryRef.current.length - 1) {
            historyIndexRef.current++;
            command = commandHistoryRef.current[historyIndexRef.current];
          } else {
            historyIndexRef.current = -1;
            command = currentCommandRef.current;
          }
          terminal.write(command);
        }
        return;
      }

      if (data === '\x1b[C' || data === '\x1b[D') { // Left/Right arrows - ignore for now
        return;
      }

      if (data === '\r') { // Enter
        terminal.write('\r\n');
        if (command.trim()) {
          // Add to history (avoid duplicates)
          const trimmed = command.trim();
          const lastCmd = commandHistoryRef.current[commandHistoryRef.current.length - 1];
          if (trimmed !== lastCmd) {
            commandHistoryRef.current.push(trimmed);
            // Limit history to 100 entries
            if (commandHistoryRef.current.length > 100) {
              commandHistoryRef.current.shift();
            }
          }

          // Handle local commands
          if (trimmed === 'help') {
            terminal.writeln('\x1b[33mAvailable Commands:\x1b[0m');
            terminal.writeln('  help     - Show this help message');
            terminal.writeln('  clear    - Clear the terminal');
            terminal.writeln('  history  - Show command history');
            terminal.writeln('  [command] - Send command to beacon');
          } else if (trimmed === 'clear') {
            terminal.clear();
          } else if (trimmed === 'history') {
            terminal.writeln('\x1b[33mCommand History:\x1b[0m');
            commandHistoryRef.current.forEach((cmd, i) => {
              terminal.writeln(`  ${i + 1}. ${cmd}`);
            });
          } else {
            try {
              await invoke('send_command', {
                implantId,
                commandType: 'shell',
                payload: command,
              });
              terminal.writeln(`\x1b[32mQueued:\x1b[0m ${command}`);
            } catch (e) {
              terminal.writeln(`\x1b[31mError:\x1b[0m ${e}`);
            }
          }
        }
        command = '';
        historyIndexRef.current = -1;
        currentCommandRef.current = '';
        terminal.write('$ ');
      } else if (data === '\u007f') { // Backspace
        if (command.length > 0) {
          command = command.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (data === '\x03') { // Ctrl+C
        terminal.write('^C\r\n$ ');
        command = '';
        historyIndexRef.current = -1;
      } else if (data === '\x0c') { // Ctrl+L - clear screen
        terminal.clear();
        terminal.write('$ ');
      } else if (data.charCodeAt(0) >= 32) { // Printable characters only
        command += data;
        terminal.write(data);
      }
    });

    xtermRef.current = terminal;

    return () => {
      window.removeEventListener('resize', handleResize);
      terminal.dispose();
    };
  }, [implantId, clearLine]);

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
