export interface StreamResponse {
  request_id: number;
  status: number;
  status_text: string;
  headers: Record<string, string>;
}

export interface McpTool {
  name: string;
  description: string;
  input_schema: any;
}

export interface McpToolCall {
  name: string;
  arguments: Record<string, any>;
}

export interface McpToolResult {
  content: Array<{ type: string; text: string }>;
  is_error: boolean;
}

export interface ChunkPayload {
  request_id: number;
  chunk: Uint8Array;
}

export interface EndPayload {
  request_id: number;
  status: number;
}
