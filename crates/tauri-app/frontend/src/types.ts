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

export interface ChatMessage {
  id: string;
  thread_id: string;
  role: "user" | "assistant" | "system";
  content: string;
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
}

export type SessionStatus = "idle" | "starting" | "ready" | "generating" | "error";

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
