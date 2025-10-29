import type { KeyboardEvent, ChangeEvent } from "react";
import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import ReactFlow, {
  Background,
  Connection,
  Controls,
  Edge,
  Node,
  type NodeProps,
  addEdge,
  useEdgesState,
  useNodesState,
  useReactFlow,
  Handle,
  MiniMap,
  Position,
  type ReactFlowInstance,
} from "reactflow";
import dagre from "dagre";
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
  import: [{ key: "path", label: "Path", placeholder: "modules/web.ax" }],
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
  script: [
    { key: "run", label: "Run", placeholder: "gobuster" },
    { key: "args", label: "Args", placeholder: "dir -u ..." },
    { key: "output", label: "Artifact label", placeholder: "corp_dirs" },
  ],
  report: [
    {
      key: "includes",
      label: "Includes",
      placeholder: "corp_findings",
    },
  ],
};

const requiredFields: Partial<Record<NodeKind, string[]>> = {
  import: ["path"],
  asset_group: ["cidr"],
  scan: ["target", "tool"],
  script: ["run"],
  report: ["includes"],
};

const defaultConfig: Record<NodeKind, () => Record<string, string>> = {
  import: () => ({ path: "" }),
  asset_group: () => ({ cidr: "", note: "" }),
  scan: () => ({ tool: "nmap", target: "", flags: "", output: "" }),
  script: () => ({ run: "echo \"Hello from Axion\"", args: "", output: "" }),
  report: () => ({ includes: "" }),
};

const labelPrefix: Record<NodeKind, string> = {
  import: "import",
  asset_group: "asset",
  scan: "scan",
  script: "script",
  report: "report",
};

const allowedConnections: Record<NodeKind, NodeKind[]> = {
  import: [],
  asset_group: ["scan", "report"],
  scan: ["script", "report"],
  script: ["report"],
  report: [],
};

const INITIAL_SCENARIO = `# Axion example
import "modules/web.ax"

asset_group corp {
  cidr "10.0.0.0/24"
  note "Corporate lab segment"
}

scan corp using nmap {
  target "10.0.0.0/24"
  flags "-sV -Pn"
} -> corp_ports

script gobuster_web {
  run "gobuster"
  args "dir -u http://10.0.0.10 -w wordlists/common.txt -t 50"
  output "corp_dirs"
}

report stdout {
  include corp_ports
  include corp_dirs
}`;

type LogLevel = "info" | "warn" | "error";

interface LogEntry {
  level: LogLevel;
  message: string;
  timestamp: string;
}

const PROFILE_KEY = "axion-profile";
const SETTINGS_KEY = "axion-settings";

const DEFAULT_PREFERENCES = {
  autoLayoutOnImport: false,
  autoRunOnPreset: false,
  rememberLastScenario: true,
  cliCommand: "cargo run -p axion-cli --",
  defaultPort: "9001",
  showHotkeys: true,
} as const;

function cloneGraphState(
  nodes: Node<AxionNodeData>[],
  edges: Edge[],
): { nodes: Node<AxionNodeData>[]; edges: Edge[] } {
  return {
    nodes: nodes.map((node) => ({
      ...node,
      position: { ...node.position },
      data: {
        ...node.data,
        config: { ...node.data.config },
      },
    })),
    edges: edges.map((edge) => ({ ...edge })),
  };
}

const EditableNode = memo(({ id, data, selected }: NodeProps<AxionNodeData>) => {
  const { setNodes } = useReactFlow();
  const [isEditing, setIsEditing] = useState(false);
  const [draftLabel, setDraftLabel] = useState(data.label);
  const [draftConfig, setDraftConfig] = useState<Record<string, string>>({
    ...data.config,
  });
  const labelRef = useRef<HTMLInputElement | null>(null);
  const fields = fieldConfig[data.kind] ?? [];
  const required = requiredFields[data.kind] ?? [];
  const missing = required.filter(
    (key) => !data.config[key] || data.config[key].trim().length === 0,
  );
  const cardClasses = ["node-card"];
  if (selected) {
    cardClasses.push("is-selected");
  }
  if (missing.length > 0) {
    cardClasses.push("has-warning");
  }

  useEffect(() => {
    setDraftLabel(data.label);
  }, [data.label]);

  useEffect(() => {
    setDraftConfig({ ...data.config });
  }, [data.config]);

  useEffect(() => {
    if (isEditing) {
      labelRef.current?.focus();
    }
  }, [isEditing]);

  const commit = useCallback(() => {
    const trimmedLabel = draftLabel.trim();
    setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id !== id) {
          return node;
        }
        const updatedConfig = { ...node.data.config };
        fields.forEach(({ key }) => {
          updatedConfig[key] = (draftConfig[key] ?? "").trim();
        });
        return {
          ...node,
          data: {
            ...node.data,
            label: trimmedLabel.length > 0 ? trimmedLabel : node.data.label,
            config: updatedConfig,
          },
        };
      }),
    );
    setIsEditing(false);
  }, [draftConfig, draftLabel, fields, id, setNodes]);

  const cancel = useCallback(() => {
    setDraftLabel(data.label);
    setDraftConfig({ ...data.config });
    setIsEditing(false);
  }, [data.config, data.label]);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.key === "Enter") {
        event.preventDefault();
        commit();
      } else if (event.key === "Escape") {
        event.preventDefault();
        cancel();
      }
    },
    [cancel, commit],
  );

  return (
    <div
      className={cardClasses.join(" ")}
      onDoubleClick={(event) => {
        event.stopPropagation();
        setIsEditing(true);
      }}
    >
      <Handle type="target" position={Position.Top} className="handle target" />
      {isEditing ? (
        <div className="node-editor">
          <label className="node-editor-row">
            <span>Label</span>
            <input
              ref={labelRef}
              value={draftLabel}
              onChange={(event) => setDraftLabel(event.target.value)}
              onKeyDown={handleKeyDown}
            />
          </label>
          {fields.map((field) => (
            <label className="node-editor-row" key={field.key}>
              <span>{field.label}</span>
              <input
                value={draftConfig[field.key] ?? ""}
                placeholder={field.placeholder}
                onChange={(event) =>
                  setDraftConfig((current) => ({
                    ...current,
                    [field.key]: event.target.value,
                  }))
                }
                onKeyDown={handleKeyDown}
              />
              {required.includes(field.key) &&
                (draftConfig[field.key]?.trim().length ?? 0) === 0 && (
                  <span className="field-warning">Required</span>
                )}
            </label>
          ))}
          <div className="node-editor-actions">
            <button type="button" onClick={commit}>
              Save
            </button>
            <button type="button" onClick={cancel}>
              Cancel
            </button>
          </div>
          {missing.length > 0 && (
            <p className="editor-warning">
              Missing: {missing.join(", ")}
            </p>
          )}
        </div>
      ) : (
        <div className="node-summary">
          <div className="node-label">{data.label}</div>
          <div className="node-kind">{data.kind.replace("_", " ")}</div>
          <ul className="node-pills">
            {fields.map((field) => (
              <li key={field.key}>
                <span>{field.label}</span>
                <code>{(data.config[field.key] || "").trim() || "—"}</code>
              </li>
            ))}
          </ul>
          <p className="node-hint">
            {missing.length > 0
              ? `Fill required: ${missing.join(", ")}`
              : "Double-click to edit"}
          </p>
        </div>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        className="handle source"
        isConnectable={data.kind !== "report"}
      />
    </div>
  );
});
EditableNode.displayName = "EditableNode";

const createNode = (id: string, kind: NodeKind, index: number): Node<AxionNodeData> => {
  const column = index % 3;
  const row = Math.floor(index / 3);
  const position = { x: column * 260, y: row * 160 };

  return {
    id,
    position,
    type: "axion",
    data: {
      label: `${labelPrefix[kind]}_${index + 1}`,
      kind,
      config: { ...defaultConfig[kind]() },
    },
    style: { width: 220 },
  };
};

const initialNodes: Node<AxionNodeData>[] = [
  createNode("import-1", "import", 0),
  {
    id: "asset-1",
    position: { x: 0, y: 140 },
    type: "axion",
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
    position: { x: 280, y: 220 },
    type: "axion",
    data: {
      label: "corp",
      kind: "scan",
      config: {
        tool: "nmap",
        target: "10.0.0.0/24",
        flags: "-sV -Pn",
        output: "corp_ports",
      },
    },
    style: { width: 220 },
  },
  {
    id: "script-1",
    position: { x: 520, y: 320 },
    type: "axion",
    data: {
      label: "gobuster_web",
      kind: "script",
      config: {
        run: "gobuster",
        args: "dir -u http://10.0.0.10 -w wordlists/common.txt -t 50",
        output: "corp_dirs",
      },
    },
    style: { width: 220 },
  },
  {
    id: "report-1",
    position: { x: 780, y: 420 },
    type: "axion",
    data: {
      label: "stdout",
      kind: "report",
      config: {
        includes: "corp_ports, corp_dirs",
      },
    },
    style: { width: 220 },
  },
];

const initialEdges: Edge[] = [
  { id: "asset-1->scan-1", source: "asset-1", target: "scan-1" },
  { id: "scan-1->script-1", source: "scan-1", target: "script-1" },
  { id: "script-1->report-1", source: "script-1", target: "report-1" },
];

const defaultEdgeOptions = { animated: true };

interface ParseResult {
  nodes: Node<AxionNodeData>[];
  edges: Edge[];
  errors: string[];
}

function parseScenario(source: string): ParseResult {
  const trimmed = source.replace(/\r\n?/g, "\n").trim();
  if (!trimmed) {
    return { nodes: [], edges: [], errors: [] };
  }

  const nodes: Node<AxionNodeData>[] = [];
  const edges: Edge[] = [];
  const errors: string[] = [];

  const importRegex = /^\s*import\s+"([^"]+)"\s*$/gm;
  let match: RegExpExecArray | null;
  let importIndex = 1;
  while ((match = importRegex.exec(trimmed)) !== null) {
    const path = match[1].trim();
    const id = `import-${importIndex++}`;
    nodes.push({
      id,
      type: "axion",
      position: { x: 0, y: (importIndex - 1) * 100 },
      data: {
        label: id,
        kind: "import",
        config: { path },
      },
      style: { width: 220 },
    });
  }

  const blockRegex =
    /(asset_group|group|scan|script|report)\s+([A-Za-z0-9_-]+)(.*?)\{([\s\S]*?)\}(?:\s*->\s*([A-Za-z0-9_-]+))?/g;

  const seenNames = new Map<string, { kind: NodeKind; id: string }>();

  let blockMatch: RegExpExecArray | null;
  let index = importIndex;
  while ((blockMatch = blockRegex.exec(trimmed)) !== null) {
    const [, rawKind, name, attributes, body, output] = blockMatch;
    let kind: NodeKind;
    switch (rawKind) {
      case "group":
      case "asset_group":
        kind = "asset_group";
        break;
      case "scan":
        kind = "scan";
        break;
      case "script":
        kind = "script";
        break;
      case "report":
        kind = "report";
        break;
      default:
        errors.push(`Unknown block type: ${rawKind}`);
        continue;
    }

    const id = `${kind}-${index++}`;
    const config: Record<string, string> = {};

    const lineRegex = /^\s*([A-Za-z0-9_-]+)\s+"([^"]*)"\s*$/gm;
    let lineMatch: RegExpExecArray | null;
    while ((lineMatch = lineRegex.exec(body)) !== null) {
      const key = lineMatch[1];
      const value = lineMatch[2];
      config[key] = value;
    }

    if (kind === "scan") {
      if (!config.target && attributes) {
        const targetMatch = /target\s+"([^"]+)"/.exec(attributes);
        if (targetMatch) {
          config.target = targetMatch[1];
        }
      }
    }

    if (output) {
      config.output = output.trim();
    }

    nodes.push({
      id,
      type: "axion",
      position: { x: (index % 3) * 260, y: Math.floor(index / 3) * 150 },
      data: {
        label: name,
        kind,
        config,
      },
      style: { width: 220 },
    });

    seenNames.set(name, { kind, id });
  }

  seenNames.forEach((sourceMeta) => {
    if (sourceMeta.kind === "report") {
      const node = nodes.find((candidate) => candidate.id === sourceMeta.id);
      if (!node) {
        return;
      }
      const includes = (node.data.config.includes || "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
      includes.forEach((include) => {
        const source = seenNames.get(include);
        if (source) {
          edges.push({
            id: `${source.id}->${node.id}`,
            source: source.id,
            target: node.id,
            animated: true,
          });
        }
      });
    }
  });

  return { nodes, edges, errors };
}

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<AxionNodeData>(
    initialNodes,
  );
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(
    "report-1",
  );
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [idCounter, setIdCounter] = useState<number>(initialNodes.length + 1);
  const [feedback, setFeedback] = useState<string>("");
  const [dslInput, setDslInput] = useState<string>(INITIAL_SCENARIO);
  const [importErrors, setImportErrors] = useState<string[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logFilter, setLogFilter] = useState<"all" | LogLevel>("all");
  const [cliInput, setCliInput] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [profile, setProfile] = useState(() => ({
    displayName: "",
    email: "",
  }));
  const [preferences, setPreferences] = useState(() => ({
    ...DEFAULT_PREFERENCES,
  }));
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isLogCenterOpen, setIsLogCenterOpen] = useState(false);
  const [logDetail, setLogDetail] = useState<LogEntry | null>(null);
  const [history, setHistory] = useState(() => [
    cloneGraphState(initialNodes, initialEdges),
  ]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const historyIndexRef = useRef(0);
  const skipHistoryRef = useRef(false);
  const historySnapshotRef = useRef(
    JSON.stringify({ nodes: initialNodes, edges: initialEdges }),
  );
  const reactFlowRef = useRef<ReactFlowInstance | null>(null);
  const [hoveredModuleId, setHoveredModuleId] = useState<string | null>(null);
  const nodeTypes = useMemo(() => ({ axion: EditableNode }), []);

  useEffect(() => {
    try {
      const storedProfile = localStorage.getItem(PROFILE_KEY);
      if (storedProfile) {
        const parsed = JSON.parse(storedProfile) as Partial<typeof profile>;
        setProfile((prev) => ({ ...prev, ...parsed }));
      }
    } catch (error) {
      console.warn("[settings] failed to restore profile", error);
    }
    try {
      const storedSettings = localStorage.getItem(SETTINGS_KEY);
      if (storedSettings) {
        const parsed = JSON.parse(storedSettings) as Partial<typeof preferences>;
        setPreferences((prev) => ({ ...prev, ...parsed }));
      }
    } catch (error) {
      console.warn("[settings] failed to restore preferences", error);
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(PROFILE_KEY, JSON.stringify(profile));
    } catch (error) {
      console.warn("[settings] failed to persist profile", error);
    }
  }, [profile]);

  useEffect(() => {
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(preferences));
    } catch (error) {
      console.warn("[settings] failed to persist preferences", error);
    }
  }, [preferences]);

  useEffect(() => {
    if (!preferences.rememberLastScenario) {
      localStorage.removeItem("axion-flow-dsl");
    }
  }, [preferences.rememberLastScenario]);

  const appendLog = useCallback(
    (level: LogLevel, message: string) => {
      const entry: LogEntry = {
        level,
        message,
        timestamp: new Date().toISOString(),
      };
      setLogs((current) => [entry, ...current].slice(0, 200));
    },
    [setLogs],
  );

  const filteredLogs = useMemo(
    () =>
      logs.filter((entry) => logFilter === "all" || entry.level === logFilter),
    [logs, logFilter],
  );

  const modules = useMemo(
    () =>
      nodes
        .filter((node) => node.data.kind === "import")
        .map((node) => {
          const pathValue = node.data.config.path ?? "";
          return {
            id: node.id,
            label: node.data.label,
            path: pathValue,
            isMissing: pathValue.trim().length === 0,
          };
        }),
    [nodes],
  );

  const documentationLinks = useMemo(
    () => [
      {
        title: "Language overview",
        description: "Syntax reference and directive semantics",
        href: "docs/language/overview.md",
      },
      {
        title: "Hello world guide",
        description: "Step-by-step walkthrough for the CLI",
        href: "docs/guide/hello-world.md",
      },
      {
        title: "SDK reference",
        description: "Embedding Axion in external tooling",
        href: "docs/reference/sdk.md",
      },
      {
        title: "UI builder guide",
        description: "How to use the React Flow prototype",
        href: "docs/guide/ui-builder.md",
      },
      {
        title: "UI builder (UA)",
        description: "Українська версія інструкції для UI",
        href: "docs/guide/ui-builder.uk.md",
      },
    ],
    [],
  );

  useEffect(() => {
    if (!preferences.rememberLastScenario) {
      return;
    }
    const stored = localStorage.getItem("axion-flow-dsl");
    if (!stored) {
      return;
    }
    setDslInput(stored);
    const result = parseScenario(stored);
    if (result.nodes.length === 0) {
      return;
    }
    skipHistoryRef.current = true;
    setNodes(result.nodes);
    setEdges(result.edges);
    setHistory([cloneGraphState(result.nodes, result.edges)]);
    historyIndexRef.current = 0;
    setHistoryIndex(0);
    historySnapshotRef.current = JSON.stringify({
      nodes: result.nodes,
      edges: result.edges,
    });
    if (result.errors.length > 0) {
      result.errors.forEach((err) => appendLog("warn", err));
    } else {
      appendLog("info", "Loaded scenario from local storage");
    }
  }, [appendLog, preferences.rememberLastScenario, setEdges, setNodes]);

  const onConnect = useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target) {
        return;
      }
      const sourceNode = nodes.find((node) => node.id === connection.source);
      const targetNode = nodes.find((node) => node.id === connection.target);
      if (!sourceNode || !targetNode) {
        return;
      }
      const allowedTargets = allowedConnections[sourceNode.data.kind] ?? [];
      if (!allowedTargets.includes(targetNode.data.kind)) {
        setFeedback(
          `Connection ${sourceNode.data.kind} → ${targetNode.data.kind} is not allowed`,
        );
        setTimeout(() => setFeedback(""), 2200);
        return;
      }
      setEdges((current) =>
        addEdge(
          {
            ...connection,
            animated: true,
          },
          current,
        ),
      );
    },
    [nodes, setEdges],
  );

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId),
    [nodes, selectedNodeId],
  );

  const dslPreview = useMemo(
    () => exportToDsl(nodes, edges),
    [nodes, edges],
  );

  useEffect(() => {
    const snapshot = JSON.stringify({ nodes, edges });
    if (skipHistoryRef.current) {
      skipHistoryRef.current = false;
      historySnapshotRef.current = snapshot;
      return;
    }
    if (snapshot === historySnapshotRef.current) {
      return;
    }
    historySnapshotRef.current = snapshot;
    setHistory((prev) => {
      const trimmed = prev.slice(0, historyIndexRef.current + 1);
      const next = [...trimmed, cloneGraphState(nodes, edges)];
      const limited = next.slice(-50);
      historyIndexRef.current = limited.length - 1;
      setHistoryIndex(historyIndexRef.current);
      return limited;
    });
    localStorage.setItem("axion-flow-dsl", dslPreview);
  }, [dslPreview, edges, nodes, setHistoryIndex]);

  const canUndo = historyIndex > 0;
  const canRedo = historyIndex < history.length - 1;

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

  const handleAddNode = useCallback(
    (kind: NodeKind) => {
      setIdCounter((current) => {
        const newId = `${kind}-${current}`;
        setNodes((existing) => [
          ...existing,
          createNode(newId, kind, existing.length),
        ]);
        setSelectedNodeId(newId);
        setSelectedEdgeId(null);
        return current + 1;
      });
    },
    [setNodes],
  );

  const handleRemoveNode = useCallback(() => {
    if (!selectedNodeId) {
      return;
    }
    setNodes((current) => current.filter((node) => node.id !== selectedNodeId));
    setEdges((current) =>
      current.filter(
        (edge) =>
          edge.source !== selectedNodeId && edge.target !== selectedNodeId,
      ),
    );
    setSelectedNodeId(null);
    setSelectedEdgeId(null);
  }, [selectedNodeId, setEdges, setNodes]);

  const handleRemoveEdge = useCallback(() => {
    if (!selectedEdgeId) {
      return;
    }
    setEdges((current) => current.filter((edge) => edge.id !== selectedEdgeId));
    setSelectedEdgeId(null);
  }, [selectedEdgeId, setEdges]);

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
        setFeedback("Copied to clipboard");
        appendLog("info", "DSL copied to clipboard");
      } else {
        setFeedback("Clipboard API is not available");
        appendLog("warn", "Clipboard API is not available");
      }
    } catch (error) {
      setFeedback("Failed to copy");
      console.error("copy failed", error);
      appendLog(
        "error",
        `Failed to copy DSL: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    } finally {
      setTimeout(() => setFeedback(""), 2000);
    }
  }, [appendLog, dslPreview]);

  const handleRun = useCallback(async () => {
    if (isRunning) {
      return;
    }
    setIsRunning(true);
    appendLog("info", `Run triggered (${nodes.length} nodes, ${edges.length} edges)`);
    try {
      const response = await fetch("/api/run", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ scenario: dslPreview }),
      });
      if (!response.ok) {
        const text = await response.text();
        try {
          const payload = JSON.parse(text);
          throw new Error(
            typeof payload.error === "string" ? payload.error : "Run failed",
          );
        } catch {
          throw new Error(text || "Run failed");
        }
      }
      const data = await response.json();
      const stdoutLines: string[] = Array.isArray(data.stdout)
        ? data.stdout
        : [];
      const stderrLines: string[] = Array.isArray(data.stderr)
        ? data.stderr
        : [];
      const exitCode =
        typeof data.exitCode === "number" ? data.exitCode : undefined;
      stdoutLines.forEach((line) => appendLog("info", line));
      stderrLines.forEach((line) => appendLog("error", line));
      if (exitCode !== undefined && exitCode !== 0) {
        const message = `Run failed with exit code ${exitCode}`;
        appendLog("error", message);
        setFeedback(message);
      } else if (stdoutLines.length === 0 && stderrLines.length === 0) {
        appendLog("info", "Run completed (no output received)");
        setFeedback("Run finished");
      } else {
        const message =
          exitCode !== undefined ? "Run completed (exit 0)" : "Run completed";
        appendLog("info", message);
        setFeedback("Run finished");
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Run failed unexpectedly";
      appendLog("error", message);
      setFeedback(message);
    } finally {
      setIsRunning(false);
      setTimeout(() => setFeedback(""), 2500);
    }
  }, [appendLog, dslPreview, edges.length, isRunning, nodes.length]);

  const handleDownloadLogs = useCallback(() => {
    if (logs.length === 0) {
      return;
    }
    const content = logs
      .map(
        (entry) =>
          `[${entry.timestamp}] [${entry.level.toUpperCase()}] ${entry.message}`,
      )
      .reverse()
      .join("\n");
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `axion-logs-${Date.now()}.txt`;
    link.click();
    URL.revokeObjectURL(url);
  }, [logs]);

  const openLogDetail = useCallback(
    (entry: LogEntry) => {
      setLogDetail(entry);
      setIsLogCenterOpen(true);
    },
    [],
  );

  const openLogCenter = useCallback(() => {
    setLogDetail((current) => current ?? filteredLogs[0] ?? null);
    setIsLogCenterOpen(true);
  }, [filteredLogs]);

  const closeLogCenter = useCallback(() => {
    setIsLogCenterOpen(false);
    setLogDetail(null);
  }, []);

  const focusModule = useCallback(
    (moduleId: string) => {
      setSelectedNodeId(moduleId);
      setSelectedEdgeId(null);
      setHoveredModuleId(null);
      const instance = reactFlowRef.current;
      if (!instance) {
        return;
      }
      const node = instance.getNode(moduleId);
      if (!node) {
        return;
      }
      const width = node.width ?? node.measured?.width ?? 220;
      const height = node.height ?? node.measured?.height ?? 160;
      instance.setCenter(
        node.position.x + width / 2,
        node.position.y + height / 2,
        { zoom: 1.1, duration: 400 },
      );
    },
    [],
  );

  const updateProfile = useCallback(
    (key: keyof SettingsProfile, value: string) => {
      setProfile((current) => ({ ...current, [key]: value }));
    },
    [],
  );

  const updatePreference = useCallback(
    (key: keyof SettingsPreferences, value: boolean | string) => {
      setPreferences((current) => ({ ...current, [key]: value }));
    },
    [],
  );

  const handleCliSubmit = useCallback(async () => {
    const command = cliInput.trim();
    if (!command) {
      return;
    }
    appendLog("info", `cli> ${command}`);
    setCliInput("");
    try {
      const response = await fetch("/api/cli", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ command }),
      });
      if (!response.ok) {
        const text = await response.text();
        try {
          const payload = JSON.parse(text);
          throw new Error(
            typeof payload.error === "string"
              ? payload.error
              : "CLI command failed",
          );
        } catch {
          throw new Error(text || "CLI command failed");
        }
      }
      const data = await response.json();
      const stdoutLines: string[] = Array.isArray(data.stdout)
        ? data.stdout
        : [];
      const stderrLines: string[] = Array.isArray(data.stderr)
        ? data.stderr
        : [];
      const exitCode =
        typeof data.exitCode === "number" ? data.exitCode : undefined;
      stdoutLines.forEach((line) => appendLog("info", line));
      stderrLines.forEach((line) => appendLog("error", line));
      if (exitCode !== undefined) {
        const level = exitCode === 0 ? "info" : "error";
        appendLog(level, `CLI exited with code ${exitCode}`);
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "CLI backend unavailable";
      appendLog("warn", message);
    }
  }, [appendLog, cliInput]);

  const handleLayout = useCallback(() => {
    const graph = new dagre.graphlib.Graph();
    graph.setDefaultEdgeLabel(() => ({}));
    graph.setGraph({ rankdir: "LR", nodesep: 160, ranksep: 180 });
    nodes.forEach((node) =>
      graph.setNode(node.id, { width: 240, height: 160 }),
    );
    edges.forEach((edge) => graph.setEdge(edge.source, edge.target));
    dagre.layout(graph);
    const laidOut = nodes.map((node) => {
      const nodeWithPos = graph.node(node.id);
      if (!nodeWithPos) {
        return node;
      }
      return {
        ...node,
        position: {
          x: nodeWithPos.x - 120,
          y: nodeWithPos.y - 80,
        },
        targetPosition: Position.Left,
        sourcePosition: Position.Right,
      };
    });
    skipHistoryRef.current = true;
    setNodes(laidOut);
    appendLog("info", "Auto layout applied");
  }, [appendLog, edges, nodes, setNodes]);

  const handleLoadPreset = useCallback(() => {
    const presetNodes: Node<AxionNodeData>[] = [
      {
        id: "asset_preset",
        type: "axion",
        position: { x: -220, y: 80 },
        data: {
          label: "scan_target",
          kind: "asset_group",
          config: {
            cidr: "scanme.nmap.org",
            note: "Nmap public test host",
          },
        },
        style: { width: 220 },
      },
      {
        id: "scan_preset",
        type: "axion",
        position: { x: 120, y: 60 },
        data: {
          label: "nmap_scan",
          kind: "scan",
          config: {
            tool: "nmap",
            target: "scanme.nmap.org",
            flags: "-sV -Pn",
            output: "scanme_ports",
          },
        },
        style: { width: 220 },
      },
      {
        id: "report_preset",
        type: "axion",
        position: { x: 420, y: 80 },
        data: {
          label: "stdout",
          kind: "report",
          config: {
            includes: "scanme_ports",
          },
        },
        style: { width: 220 },
      },
    ];

    const presetEdges: Edge[] = [
      {
        id: "edge_asset_scan",
        source: "asset_preset",
        target: "scan_preset",
        animated: true,
      },
      {
        id: "edge_scan_report",
        source: "scan_preset",
        target: "report_preset",
        animated: true,
      },
    ];

    skipHistoryRef.current = true;
    setNodes(presetNodes);
    setEdges(presetEdges);
    setSelectedNodeId(null);
    setSelectedEdgeId(null);

    const dsl = exportToDsl(presetNodes, presetEdges);
    setDslInput(dsl);
    setHistory([cloneGraphState(presetNodes, presetEdges)]);
    historyIndexRef.current = 0;
    setHistoryIndex(0);
    historySnapshotRef.current = JSON.stringify({
      nodes: presetNodes,
      edges: presetEdges,
    });
    if (preferences.rememberLastScenario) {
      localStorage.setItem("axion-flow-dsl", dsl);
    }
    appendLog("info", "Loaded preset: nmap test scan");

    if (preferences.autoLayoutOnImport) {
      setTimeout(() => handleLayout(), 0);
    }
    if (preferences.autoRunOnPreset) {
      setTimeout(() => {
        if (!isRunning) {
          handleRun();
        }
      }, 250);
    }
  }, [
    appendLog,
    handleLayout,
    handleRun,
    isRunning,
    preferences.autoLayoutOnImport,
    preferences.autoRunOnPreset,
    preferences.rememberLastScenario,
    setEdges,
    setNodes,
  ]);

  const applySnapshot = useCallback(
    (snapshot: { nodes: Node<AxionNodeData>[]; edges: Edge[] }) => {
      skipHistoryRef.current = true;
      const clone = cloneGraphState(snapshot.nodes, snapshot.edges);
      setNodes(clone.nodes);
      setEdges(clone.edges);
      historySnapshotRef.current = JSON.stringify({
        nodes: clone.nodes,
        edges: clone.edges,
      });
    },
    [setEdges, setNodes],
  );

  const handleUndo = useCallback(() => {
    if (historyIndexRef.current <= 0) {
      return;
    }
    const nextIndex = historyIndexRef.current - 1;
    const snapshot = history[nextIndex];
    if (!snapshot) {
      return;
    }
    applySnapshot(snapshot);
    historyIndexRef.current = nextIndex;
    setHistoryIndex(nextIndex);
    appendLog("info", "Undo");
  }, [appendLog, applySnapshot, history]);

  const handleRedo = useCallback(() => {
    if (historyIndexRef.current >= history.length - 1) {
      return;
    }
    const nextIndex = historyIndexRef.current + 1;
    const snapshot = history[nextIndex];
    if (!snapshot) {
      return;
    }
    applySnapshot(snapshot);
    historyIndexRef.current = nextIndex;
    setHistoryIndex(nextIndex);
    appendLog("info", "Redo");
  }, [appendLog, applySnapshot, history]);

  const handleClearLogs = useCallback(() => {
    setLogs([]);
    setLogDetail(null);
    setIsLogCenterOpen(false);
  }, []);

  const handleImport = useCallback(() => {
    const { nodes: parsedNodes, edges: parsedEdges, errors } =
      parseScenario(dslInput);
    localStorage.setItem("axion-flow-dsl", dslInput);
    if (errors.length > 0) {
      setImportErrors(errors);
      errors.forEach((err) => appendLog("warn", err));
      setFeedback("Parsed with warnings, see below.");
    } else {
      setImportErrors([]);
      setFeedback("Scenario imported successfully");
      appendLog("info", "Scenario imported");
    }
    if (parsedNodes.length === 0) {
      return;
    }
    skipHistoryRef.current = true;
    setNodes(parsedNodes);
    setEdges(parsedEdges);
    setHistory([cloneGraphState(parsedNodes, parsedEdges)]);
    historyIndexRef.current = 0;
    setHistoryIndex(0);
    historySnapshotRef.current = JSON.stringify({
      nodes: parsedNodes,
      edges: parsedEdges,
    });
    setSelectedNodeId(parsedNodes[parsedNodes.length - 1]?.id ?? null);
    setSelectedEdgeId(null);
    setIdCounter(parsedNodes.length + 1);
    setTimeout(() => setFeedback(""), 2500);
  }, [appendLog, dslInput, setEdges, setNodes]);

  const handleDslChange = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
    setDslInput(event.target.value);
  }, []);

  useEffect(() => {
    setEdges((current) =>
      current.map((edge) => {
        const isSelected = edge.id === selectedEdgeId;
        const baseStyle = edge.style ?? {};
        return {
          ...edge,
          style: {
            ...baseStyle,
            stroke: isSelected ? "#67b1ff" : baseStyle.stroke ?? "#62718f",
            strokeWidth: isSelected ? 2.5 : baseStyle.strokeWidth ?? 1.5,
            opacity: isSelected ? 1 : 0.9,
          },
        };
      }),
    );
  }, [selectedEdgeId, setEdges]);

  useEffect(() => {
    if (
      selectedEdgeId &&
      !edges.some((edge) => edge.id === selectedEdgeId)
    ) {
      setSelectedEdgeId(null);
    }
  }, [edges, selectedEdgeId]);

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
          onInit={(instance) => {
            reactFlowRef.current = instance;
          }}
          onNodeClick={(_, node) => {
            setSelectedNodeId(node.id);
            setSelectedEdgeId(null);
          }}
          onEdgeClick={(_, edge) => {
            setSelectedEdgeId(edge.id);
            setSelectedNodeId(null);
          }}
          onPaneClick={() => {
            setSelectedNodeId(null);
            setSelectedEdgeId(null);
            setHoveredModuleId(null);
          }}
          nodeTypes={nodeTypes}
        >
          <Background />
          <Controls />
          <MiniMap pannable zoomable />
        </ReactFlow>
        <div className="node-toolbar">
          <span className="toolbar-label">Add</span>
          <div className="toolbar-buttons">
            <button type="button" onClick={() => handleAddNode("import")}>
              Import
            </button>
            <button type="button" onClick={() => handleAddNode("asset_group")}>
              Asset
            </button>
            <button type="button" onClick={() => handleAddNode("scan")}>
              Scan
            </button>
            <button type="button" onClick={() => handleAddNode("script")}>
              Script
            </button>
            <button type="button" onClick={() => handleAddNode("report")}>
              Report
            </button>
          </div>
          <div className="toolbar-actions">
            <button
              type="button"
              className="toolbar-remove"
              onClick={handleRemoveNode}
              disabled={!selectedNodeId}
            >
              Remove node
            </button>
            <button
              type="button"
              className="toolbar-remove"
              onClick={handleRemoveEdge}
              disabled={!selectedEdgeId}
            >
              Remove edge
            </button>
            <button type="button" onClick={handleUndo} disabled={!canUndo}>
              Undo
            </button>
            <button type="button" onClick={handleRedo} disabled={!canRedo}>
              Redo
            </button>
            <button type="button" onClick={handleLoadPreset}>
              Load test scan
            </button>
            <button
              type="button"
              className="toolbar-settings"
              onClick={() => setIsSettingsOpen(true)}
            >
              Settings
            </button>
            <button type="button" onClick={handleLayout}>
              Auto layout
            </button>
            <button
              type="button"
              className="toolbar-run"
              onClick={handleRun}
              disabled={isRunning}
            >
              {isRunning ? "Running..." : "Run scenario"}
            </button>
          </div>
        </div>
      </div>

      <div className="sidebar">
        <h2>Scenario</h2>
        <textarea
          value={dslInput}
          onChange={handleDslChange}
          rows={12}
          placeholder="Paste Axion DSL here and click Import"
        ></textarea>
        <button type="button" onClick={handleImport}>
          Import DSL
        </button>
        <div className="cli-panel cli-inline">
          <h3>CLI</h3>
          <textarea
            value={cliInput}
            onChange={(event) => setCliInput(event.target.value)}
            rows={3}
            placeholder="axion> run examples/hello.ax"
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
                event.preventDefault();
                handleCliSubmit();
              }
            }}
          ></textarea>
          <button type="button" onClick={handleCliSubmit}>
            Send
          </button>
        </div>
        <h2>Node editor</h2>
        <div className={`form node-form${selectedNode ? " is-active" : ""}`}>
          {selectedNode ? (
            <>
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
            </>
          ) : (
            <p>Select a node to edit its properties.</p>
          )}
        </div>
        {modules.length > 0 && (
          <div className="module-panel">
            <h3>Modules</h3>
            <ul>
              {modules.map((module) => {
                const isActive =
                  module.id === hoveredModuleId || module.id === selectedNodeId;
                const classes = ["module-item"];
                if (isActive) {
                  classes.push("is-active");
                }
                if (module.isMissing) {
                  classes.push("is-missing");
                }
                return (
                  <li key={module.id} className={classes.join(" ")}>
                    <button
                      type="button"
                      className="unstyled-button module-button"
                      onClick={() => focusModule(module.id)}
                      onMouseEnter={() => setHoveredModuleId(module.id)}
                      onMouseLeave={() => setHoveredModuleId(null)}
                    >
                      <span className="module-label">{module.label}</span>
                      <code className="module-path">
                        {module.path || "(path not set)"}
                      </code>
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        )}
        {preferences.showHotkeys && (
          <div className="hotkey-hints">
            <h3>Shortcuts</h3>
            <ul>
              <li>
                <code>Double click</code> — edit node in place
              </li>
              <li>
                <code>Del</code> — delete selected node or edge
              </li>
              <li>
                <code>Ctrl/Cmd + Enter</code> — send CLI command
              </li>
              <li>
                <code>Right click + drag</code> — pan canvas
              </li>
              <li>
                <code>Scroll</code> — zoom, <code>Shift + Scroll</code> — horizontal pan
              </li>
            </ul>
          </div>
        )}
        {importErrors.length > 0 && (
          <div className="errors">
            <h3>Import warnings</h3>
            <ul>
              {importErrors.map((error, index) => (
                <li key={index}>{error}</li>
              ))}
            </ul>
          </div>
        )}
        <h2>DSL Preview</h2>
        <textarea value={dslPreview} readOnly rows={18}></textarea>
        <button type="button" onClick={handleCopy}>
          Copy DSL
        </button>
        {feedback && <p className="feedback">{feedback}</p>}
        <p className="hint">
          Changes in the editor are reflected in the preview immediately. Use
          the copy button to paste the DSL back into your scenario files.
        </p>

        <div className="log-panel">
          <div className="log-header">
            <h3>Logs</h3>
            <div className="log-controls">
              <select
                value={logFilter}
                onChange={(event) =>
                  setLogFilter(event.target.value as "all" | LogLevel)
                }
              >
                <option value="all">All</option>
                <option value="info">Info</option>
                <option value="warn">Warn</option>
                <option value="error">Error</option>
              </select>
              <button
                type="button"
                onClick={openLogCenter}
                disabled={logs.length === 0}
              >
                Log centre
              </button>
              <button
                type="button"
                onClick={handleDownloadLogs}
                disabled={logs.length === 0}
              >
                Download
              </button>
              <button
                type="button"
                onClick={handleClearLogs}
                disabled={logs.length === 0}
              >
                Clear
              </button>
            </div>
          </div>
          {filteredLogs.length === 0 ? (
            <p className="log-placeholder">
              No entries yet. Click Run to simulate execution.
            </p>
          ) : (
            <ul>
              {filteredLogs.map((entry, index) => (
                <li
                  key={`${entry.timestamp}-${index}`}
                  className={`log-entry log-${entry.level}`}
                >
                  <button
                    type="button"
                    className="unstyled-button log-entry-button"
                    onClick={() => openLogDetail(entry)}
                  >
                    <span className="log-time">
                      {new Date(entry.timestamp).toLocaleTimeString()}
                    </span>
                    <span className="log-level">
                      {entry.level.toUpperCase()}
                    </span>
                    <span className="log-text">{entry.message}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      {isLogCenterOpen && (
        <div className="log-center-overlay" onClick={closeLogCenter}>
          <div className="log-center-card" onClick={(event) => event.stopPropagation()}>
            <header className="log-center-header">
              <div>
                <h3>Log centre</h3>
                <p>Review execution output, copy diagnostics, and inspect artifacts.</p>
              </div>
              <div className="log-center-actions">
                <select
                  value={logFilter}
                  onChange={(event) =>
                    setLogFilter(event.target.value as "all" | LogLevel)
                  }
                >
                  <option value="all">All</option>
                  <option value="info">Info</option>
                  <option value="warn">Warn</option>
                  <option value="error">Error</option>
                </select>
                <button
                  type="button"
                  onClick={handleDownloadLogs}
                  disabled={logs.length === 0}
                >
                  Download
                </button>
                <button
                  type="button"
                  onClick={handleClearLogs}
                  disabled={logs.length === 0}
                >
                  Clear
                </button>
                <button type="button" onClick={closeLogCenter}>
                  Close
                </button>
              </div>
            </header>
            <div className="log-center-body">
              <aside className="log-center-list">
                {filteredLogs.length === 0 ? (
                  <p className="log-placeholder">No entries match current filter.</p>
                ) : (
                  <ul>
                    {filteredLogs.map((entry, index) => {
                      const isSelected =
                        logDetail &&
                        logDetail.timestamp === entry.timestamp &&
                        logDetail.message === entry.message;
                      const rowClasses = ["log-entry", "log-center-item", `log-${entry.level}`];
                      if (isSelected) {
                        rowClasses.push("is-selected");
                      }
                      return (
                        <li key={`${entry.timestamp}-${index}`}>
                          <button
                            type="button"
                            className={`unstyled-button log-entry-button ${rowClasses.join(" ")}`}
                            onClick={() => setLogDetail(entry)}
                          >
                            <span className="log-time">
                              {new Date(entry.timestamp).toLocaleTimeString()}
                            </span>
                            <span className="log-level">
                              {entry.level.toUpperCase()}
                            </span>
                            <span className="log-text">{entry.message}</span>
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </aside>
              <section className="log-center-detail">
                {logDetail ? (
                  <>
                    <div className={`log-detail-badge log-${logDetail.level}`}>
                      {logDetail.level.toUpperCase()}
                    </div>
                    <p className="log-detail-meta">
                      {new Date(logDetail.timestamp).toLocaleString()}
                    </p>
                    <LogDetailBody entry={logDetail} />
                  </>
                ) : (
                  <p className="log-placeholder">Select a log entry to inspect the payload.</p>
                )}
              </section>
            </div>
          </div>
        </div>
      )}
      {isSettingsOpen && (
        <SettingsPage
          profile={profile}
          preferences={preferences}
          documentationLinks={documentationLinks}
          onClose={() => setIsSettingsOpen(false)}
          onProfileChange={updateProfile}
          onPreferenceChange={updatePreference}
          onLoadPreset={handleLoadPreset}
        />
      )}
    </div>
  );
}

type LogDetailBodyProps = {
  entry: LogEntry;
};

const LogDetailBody = memo(function LogDetailBody({ entry }: LogDetailBodyProps) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setCopied(false);
  }, [entry.timestamp, entry.message]);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timeout = setTimeout(() => setCopied(false), 1600);
    return () => clearTimeout(timeout);
  }, [copied]);

  const prettified = useMemo(() => {
    const trimmed = entry.message.trim();
    if (!trimmed) {
      return null;
    }
    const looksLikeJson =
      (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
      (trimmed.startsWith("[") && trimmed.endsWith("]"));
    if (!looksLikeJson) {
      return null;
    }
    try {
      const parsed = JSON.parse(trimmed);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return null;
    }
  }, [entry.message]);

  const handleCopy = useCallback(async () => {
    try {
      if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(entry.message);
      } else if (typeof window !== "undefined" && typeof document !== "undefined") {
        const textarea = document.createElement("textarea");
        textarea.value = entry.message;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }, [entry.message]);

  return (
    <div className="log-detail-content">
      <div className="log-detail-toolbar">
        <button type="button" onClick={handleCopy}>
          {copied ? "Copied" : "Copy message"}
        </button>
      </div>
      {prettified ? (
        <>
          <h4 className="log-detail-title">Structured payload</h4>
          <pre className="log-detail-pre">{prettified}</pre>
          <details className="log-detail-raw">
            <summary>View raw message</summary>
            <pre>{entry.message}</pre>
          </details>
        </>
      ) : (
        <pre className="log-detail-pre">{entry.message}</pre>
      )}
    </div>
  );
});

type SettingsProfile = {
  displayName: string;
  email: string;
};

type SettingsPreferences = typeof DEFAULT_PREFERENCES;

type DocumentationLink = {
  title: string;
  description: string;
  href: string;
};

type SettingsPageProps = {
  profile: SettingsProfile;
  preferences: SettingsPreferences;
  documentationLinks: DocumentationLink[];
  onClose: () => void;
  onProfileChange: (key: keyof SettingsProfile, value: string) => void;
  onPreferenceChange: (
    key: keyof SettingsPreferences,
    value: boolean | string,
  ) => void;
  onLoadPreset: () => void;
};

const SettingsPage = memo(function SettingsPage({
  profile,
  preferences,
  documentationLinks,
  onClose,
  onProfileChange,
  onPreferenceChange,
  onLoadPreset,
}: SettingsPageProps) {
  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-content" onClick={(event) => event.stopPropagation()}>
        <header className="settings-header">
          <div>
            <h2>Workspace settings</h2>
            <p>Adjust UI behaviour, CLI defaults, and browse documentation.</p>
          </div>
          <button type="button" className="settings-close" onClick={onClose}>
            Close
          </button>
        </header>
        <section className="settings-section">
          <h3>Profile</h3>
          <div className="settings-grid">
            <label>
              <span>Display name</span>
              <input
                placeholder="Display name"
                value={profile.displayName}
                onChange={(event) => onProfileChange("displayName", event.target.value)}
              />
            </label>
            <label>
              <span>Email</span>
              <input
                type="email"
                placeholder="operator@example.com"
                value={profile.email}
                onChange={(event) => onProfileChange("email", event.target.value)}
              />
            </label>
          </div>
          <p className="settings-hint">
            Profile data is stored locally and can be reused for future reporting workflows.
          </p>
        </section>
        <section className="settings-section">
          <h3>Preferences</h3>
          <div className="settings-preferences">
            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={preferences.autoLayoutOnImport}
                onChange={(event) =>
                  onPreferenceChange("autoLayoutOnImport", event.target.checked)
                }
              />
              <div>
                <span>Auto layout after import</span>
                <p>Automatically arrange nodes whenever a scenario or preset loads.</p>
              </div>
            </label>
            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={preferences.autoRunOnPreset}
                onChange={(event) =>
                  onPreferenceChange("autoRunOnPreset", event.target.checked)
                }
              />
              <div>
                <span>Run automatically after presets</span>
                <p>Trigger execution as soon as a preset is inserted.</p>
              </div>
            </label>
            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={preferences.rememberLastScenario}
                onChange={(event) =>
                  onPreferenceChange("rememberLastScenario", event.target.checked)
                }
              />
              <div>
                <span>Remember last scenario</span>
                <p>Keep the latest DSL snapshot in local storage between sessions.</p>
              </div>
            </label>
            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={preferences.showHotkeys}
                onChange={(event) =>
                  onPreferenceChange("showHotkeys", event.target.checked)
                }
              />
              <div>
                <span>Show hotkey hints</span>
                <p>Display keyboard shortcuts in the editor sidebar.</p>
              </div>
            </label>
            <label>
              <span>CLI command</span>
              <input
                placeholder="cargo run -p axion-cli --"
                value={preferences.cliCommand}
                onChange={(event) =>
                  onPreferenceChange("cliCommand", event.target.value)
                }
              />
            </label>
            <label>
              <span>Dev server port</span>
              <input
                placeholder="9001"
                value={preferences.defaultPort}
                onChange={(event) =>
                  onPreferenceChange("defaultPort", event.target.value)
                }
              />
            </label>
          </div>
        </section>
        <section className="settings-section">
          <h3>Documentation</h3>
          <ul className="settings-docs">
            {documentationLinks.map((link) => (
              <li key={link.title}>
                <div>
                  <span className="docs-title">{link.title}</span>
                  <span className="docs-desc">{link.description}</span>
                </div>
                <code className="docs-path">{link.href}</code>
              </li>
            ))}
          </ul>
          <p className="settings-hint">
            Open the listed Markdown files in your editor or host the <code>docs/</code> directory for live docs.
          </p>
        </section>
        <section className="settings-section">
          <h3>Quick actions</h3>
          <div className="settings-actions">
            <button type="button" onClick={onLoadPreset}>
              Insert test scan preset
            </button>
          </div>
        </section>
      </div>
    </div>
  );
});



