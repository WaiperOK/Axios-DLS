import { useCallback, useMemo, useState } from "react";
import ReactFlow, {
  Background,
  Connection,
  Controls,
  Edge,
  Node,
  addEdge,
  useEdgesState,
  useNodesState,
} from "reactflow";
import "reactflow/dist/style.css";
import "./styles.css";

import { exportToDsl } from "./dsl";
import { AxionNodeData, NodeKind } from "./types";

type Field = {
  key: string;
  label: string;
  placeholder?: string;
};

const fieldConfig: Record<NodeKind, Field[]> = {
  asset_group: [
    { key: "cidr", label: "CIDR", placeholder: "10.0.0.0/24" },
    { key: "note", label: "Note", placeholder: "Optional comment" },
  ],
  scan: [
    { key: "tool", label: "Tool", placeholder: "nmap" },
    { key: "target", label: "Target", placeholder: "10.0.0.0/24" },
    { key: "flags", label: "Flags", placeholder: "-sV -Pn" },
    { key: "output", label: "Artifact label", placeholder: "corp_findings" },
  ],
  report: [
    {
      key: "includes",
      label: "Includes",
      placeholder: "corp_findings",
    },
  ],
};

const initialNodes: Node<AxionNodeData>[] = [
  {
    id: "asset-1",
    position: { x: 0, y: 0 },
    data: {
      label: "corp",
      kind: "asset_group",
      config: {
        cidr: "10.0.0.0/24",
        note: "Corporate lab segment",
      },
    },
    style: { width: 220 },
  },
  {
    id: "scan-1",
    position: { x: 280, y: 80 },
    data: {
      label: "corp",
      kind: "scan",
      config: {
        tool: "nmap",
        target: "10.0.0.0/24",
        flags: "-sV -Pn",
        output: "corp_findings",
      },
    },
    style: { width: 220 },
  },
  {
    id: "report-1",
    position: { x: 560, y: 160 },
    data: {
      label: "stdout",
      kind: "report",
      config: {
        includes: "corp_findings",
      },
    },
    style: { width: 220 },
  },
];

const initialEdges: Edge[] = [
  { id: "asset-1->scan-1", source: "asset-1", target: "scan-1" },
  { id: "scan-1->report-1", source: "scan-1", target: "report-1" },
];

const defaultEdgeOptions = { animated: true };

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<AxionNodeData>(
    initialNodes,
  );
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(
    "report-1",
  );
  const [copyFeedback, setCopyFeedback] = useState<string>("");

  const onConnect = useCallback(
    (connection: Connection) =>
      setEdges((current) => addEdge({ ...connection, animated: true }, current)),
    [setEdges],
  );

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId),
    [nodes, selectedNodeId],
  );

  const dslPreview = useMemo(
    () => exportToDsl(nodes, edges),
    [nodes, edges],
  );

  const updateNode = useCallback(
    (nodeId: string, updater: (data: AxionNodeData) => AxionNodeData) => {
      setNodes((current) =>
        current.map((node) =>
          node.id === nodeId ? { ...node, data: updater(node.data) } : node,
        ),
      );
    },
    [setNodes],
  );

  const handleLabelChange = useCallback(
    (value: string) => {
      if (!selectedNodeId) {
        return;
      }
      updateNode(selectedNodeId, (data) => ({ ...data, label: value }));
    },
    [selectedNodeId, updateNode],
  );

  const handleConfigChange = useCallback(
    (key: string, value: string) => {
      if (!selectedNodeId) {
        return;
      }
      updateNode(selectedNodeId, (data) => ({
        ...data,
        config: { ...data.config, [key]: value },
      }));
    },
    [selectedNodeId, updateNode],
  );

  const handleCopy = useCallback(async () => {
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(dslPreview);
        setCopyFeedback("Copied to clipboard");
      } else {
        setCopyFeedback("Clipboard API is not available");
      }
    } catch (error) {
      setCopyFeedback("Failed to copy");
      console.error("copy failed", error);
    } finally {
      setTimeout(() => setCopyFeedback(""), 2000);
    }
  }, [dslPreview]);

  return (
    <div className="app">
      <div className="canvas">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          defaultEdgeOptions={defaultEdgeOptions}
          fitView
          onNodeClick={(_, node) => setSelectedNodeId(node.id)}
        >
          <Background />
          <Controls />
        </ReactFlow>
      </div>
      <div className="sidebar">
        <h2>Node editor</h2>
        {selectedNode ? (
          <div className="form">
            <label className="form-row">
              <span>Label</span>
              <input
                value={selectedNode.data.label}
                onChange={(event) => handleLabelChange(event.target.value)}
              />
            </label>
            {fieldConfig[selectedNode.data.kind].map((field) => (
              <label className="form-row" key={field.key}>
                <span>{field.label}</span>
                <input
                  placeholder={field.placeholder}
                  value={selectedNode.data.config[field.key] || ""}
                  onChange={(event) =>
                    handleConfigChange(field.key, event.target.value)
                  }
                />
              </label>
            ))}
          </div>
        ) : (
          <p>Select a node to edit its properties.</p>
        )}

        <h2>DSL Preview</h2>
        <textarea value={dslPreview} readOnly rows={18}></textarea>
        <button type="button" onClick={handleCopy}>
          Copy DSL
        </button>
        {copyFeedback && <p className="feedback">{copyFeedback}</p>}
        <p className="hint">
          Changes in the editor are reflected in the preview immediately. Use
          the copy button to paste the DSL back into your scenario files.
        </p>
      </div>
    </div>
  );
}
