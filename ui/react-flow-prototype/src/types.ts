export type NodeKind =
  | "import"
  | "asset_group"
  | "scan"
  | "script"
  | "report";

export interface AxionNodeData {
  label: string;
  kind: NodeKind;
  config: Record<string, string>;
}
