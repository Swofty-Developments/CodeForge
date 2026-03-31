export interface Project {
  id: string;
  name: string;
  path: string;
  color: string | null;
  collapsed: boolean;
  threads: Thread[];
}

export interface Thread {
  id: string;
  project_id: string;
  title: string;
  color: string | null;
}

export interface MessageMeta {
  model?: string;
  inputTokens?: number;
  outputTokens?: number;
  durationMs?: number;
  costUsd?: number;
}

export interface ChatMessage {
  id: string;
  thread_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  blocks?: ContentBlock[];
  meta?: MessageMeta;
}

export interface AgentEventPayload {
  session_id: string;
  thread_id: string;
  event_type: string;
  text?: string;
  turn_id?: string;
  reason?: string;
  message?: string;
  request_id?: string;
  description?: string;
  input_tokens?: number;
  output_tokens?: number;
  cost_usd?: number;
  model?: string;
  tool_id?: string;
  tool_name?: string;
  input_json?: string;
  tool_output?: string;
  is_error?: boolean;
  cache_read_tokens?: number;
  cache_write_tokens?: number;
}

export interface ThreadTokenUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  totalTokens: number;
  model?: string;
}

export type ToolStatus = "generating" | "running" | "completed" | "error";

export interface ContentBlock {
  type: "text" | "tool_use" | "thinking";
  content: string;
  tool_id?: string;
  tool_name?: string;
  tool_input?: string;
  tool_output?: string;
  tool_status?: ToolStatus;
  tool_error?: boolean;
}

export interface Attachment {
  id: string;
  type: "file" | "extraction" | "image";
  name: string;
  content: string;
  language?: string;
}

export type SessionStatus = "idle" | "starting" | "ready" | "generating" | "interrupting" | "error";

export const THREAD_COLORS = [
  { hex: "#e65961", label: "Red" },
  { hex: "#e6b84d", label: "Amber" },
  { hex: "#59c78c", label: "Green" },
  { hex: "#66b8e0", label: "Sky" },
  { hex: "#6680f2", label: "Blue" },
  { hex: "#c084fc", label: "Purple" },
  { hex: "#f472b6", label: "Pink" },
  { hex: "#fb923c", label: "Orange" },
];
