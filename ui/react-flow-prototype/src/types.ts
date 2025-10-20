export type NodeKind = "asset_group" | "scan" | "report";

export interface AxionNodeData {
  label: string;
  kind: NodeKind;
  config: Record<string, string>;
}
