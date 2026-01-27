import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ReactFlow, {
  Controls,
  Background,
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
  Node,
  Edge,
  OnNodesChange,
  OnEdgesChange,
  OnConnect,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { Button } from './ui/Button';

const initialNodes: Node[] = [
  {
    id: '1',
    data: { label: 'Start (Initial Access)' },
    position: { x: 250, y: 5 },
    type: 'input',
    style: { background: '#1e293b', color: '#fff', border: '1px solid #334155' },
  },
];

function mapLabelToType(label: string): string {
    if (label.includes("Shell")) return "shell";
    if (label.includes("PowerShell")) return "powershell";
    if (label.includes("Registry")) return "persist";
    if (label.includes("SysInfo")) return "sys_info";
    return "shell";
}

export default function AttackChainEditor() {
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [nodeStatuses, setNodeStatuses] = useState<Record<string, 'pending' | 'running' | 'success' | 'failed'>>({});

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
    []
  );
  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    []
  );
  const onConnect: OnConnect = useCallback(
    (params) => setEdges((eds) => addEdge(params, eds)),
    []
  );

  const onDragStart = (event: React.DragEvent, nodeType: string, label: string) => {
    event.dataTransfer.setData('application/reactflow/type', nodeType);
    event.dataTransfer.setData('application/reactflow/label', label);
    event.dataTransfer.effectAllowed = 'move';
  };

  const handleSave = async () => {
      const sorted = [...nodes].sort((a, b) => a.position.x - b.position.x);
      const steps = sorted.filter(n => n.id !== '1').map((n, i) => ({
          step_order: i + 1,
          technique_id: n.data.label,
          command_type: mapLabelToType(n.data.label),
          payload: "whoami", // Default payload
          description: n.data.label
      }));
      
      try {
          const json = await invoke('create_attack_chain', {
              name: "Chain-" + Date.now(),
              description: "Created via Editor",
              steps: steps
          }) as string;
          const chain = JSON.parse(json);
          alert("Chain saved! ID: " + chain.id);
          return chain.id;
      } catch (e) {
          console.error(e);
          alert("Save failed: " + e);
          return null;
      }
  };

  const handleExecute = async () => {
      const implantId = prompt("Enter Target Implant ID:");
      if (!implantId) return;

      const chainId = await handleSave();
      if (!chainId) return;

      try {
          await invoke('execute_attack_chain', { chainId, implantId });
          const newStatus: Record<string, 'running'> = {};
          nodes.forEach(n => newStatus[n.id] = 'running');
          setNodeStatuses(newStatus);
          alert("Execution started!");
      } catch (e) {
          alert("Execution failed: " + e);
      }
  };

  // Apply styles based on status
  const styledNodes = nodes.map(node => ({
      ...node,
      style: {
          ...node.style,
          borderColor: nodeStatuses[node.id] === 'success' ? '#22c55e' : 
                       nodeStatuses[node.id] === 'failed' ? '#ef4444' : 
                       nodeStatuses[node.id] === 'running' ? '#eab308' : '#334155',
          borderWidth: nodeStatuses[node.id] ? '2px' : '1px',
          boxShadow: nodeStatuses[node.id] === 'running' ? '0 0 10px rgba(234, 179, 8, 0.5)' : 'none'
      }
  }));

  return (
    <div className="flex h-full bg-slate-950 text-white rounded-lg shadow border border-slate-800 overflow-hidden">
        {/* Sidebar Palette */}
        <aside className="w-64 bg-slate-900 border-r border-slate-800 p-4 flex flex-col gap-4">
            <h3 className="font-bold text-red-500 uppercase tracking-wider text-sm">Technique Palette</h3>
            <div className="space-y-2">
                <div className="text-xs text-slate-500 font-bold uppercase">Execution</div>
                <div 
                    className="p-2 bg-slate-800 border border-slate-700 rounded cursor-grab hover:border-red-500 transition-colors"
                    draggable
                    onDragStart={(e) => onDragStart(e, 'default', 'Shell Command')}
                >
                    Shell Command
                </div>
                <div 
                    className="p-2 bg-slate-800 border border-slate-700 rounded cursor-grab hover:border-red-500 transition-colors"
                    draggable
                    onDragStart={(e) => onDragStart(e, 'default', 'PowerShell')}
                >
                    PowerShell
                </div>
                
                <div className="text-xs text-slate-500 font-bold uppercase mt-4">Persistence</div>
                <div 
                    className="p-2 bg-slate-800 border border-slate-700 rounded cursor-grab hover:border-red-500 transition-colors"
                    draggable
                    onDragStart={(e) => onDragStart(e, 'default', 'Registry Run')}
                >
                    Registry Run Key
                </div>
                
                <div className="text-xs text-slate-500 font-bold uppercase mt-4">Discovery</div>
                <div 
                    className="p-2 bg-slate-800 border border-slate-700 rounded cursor-grab hover:border-red-500 transition-colors"
                    draggable
                    onDragStart={(e) => onDragStart(e, 'default', 'SysInfo')}
                >
                    System Info
                </div>
            </div>
            
            <div className="mt-auto space-y-2">
                <Button className="w-full" onClick={handleSave}>Save Chain</Button>
                <Button className="w-full" variant="danger" onClick={handleExecute}>Execute</Button>
            </div>
        </aside>

        {/* Canvas */}
        <div className="flex-1 h-full" 
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
                e.preventDefault();
                const type = e.dataTransfer.getData('application/reactflow/type');
                const label = e.dataTransfer.getData('application/reactflow/label');
                
                // Get drop position (simplified, needs bounds ref)
                // In real app, project to flow coordinates
                const position = { x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY };
                
                const newNode: Node = {
                    id: crypto.randomUUID(),
                    type,
                    position,
                    data: { label: label },
                    style: { background: '#1e293b', color: '#fff', border: '1px solid #334155', minWidth: '150px' },
                };
                
                setNodes((nds) => nds.concat(newNode));
            }}
        >
            <ReactFlow
                nodes={styledNodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                fitView
                className="bg-slate-950"
            >
                <Background color="#334155" gap={16} />
                <Controls className="bg-slate-800 border-slate-700 fill-white text-white" />
            </ReactFlow>
        </div>
    </div>
  );
}
