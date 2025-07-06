import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { 
  StreamResponse, 
  McpTool, 
  McpToolCall, 
  McpToolResult, 
  ChunkPayload, 
  EndPayload 
} from '../types/ai';

export function useAiClient() {
  const tools = ref<McpTool[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  
  const streamListeners = new Map<number, {
    onChunk: (chunk: string) => void;
    onEnd: (status: number) => void;
    onError: (error: string) => void;
  }>();

  let unlistenStream: UnlistenFn | null = null;

  // 初始化流式响应监听器
  const initializeStreamListener = async () => {
    unlistenStream = await listen<ChunkPayload | EndPayload>('stream-response', (event) => {
      const payload = event.payload;
      console.log('Received event:', event);
      
      // 修复：正确判断是 chunk 还是 end payload
      if ('chunk' in payload) {
        console.log('Received chunk:', payload.chunk);
        // 处理数据块
        const listener = streamListeners.get(payload.request_id);
        if (listener) {
          const chunk = new TextDecoder().decode(new Uint8Array(payload.chunk));
          listener.onChunk(chunk);
        }
      } else if ('status' in payload) {
        // 修复：应该检查 'status' 而不是 'DONE'
        console.log('Received end with status:', payload.status);
        // 处理结束事件
        const listener = streamListeners.get(payload.request_id);
        if (listener) {
          listener.onEnd(payload.status);
          streamListeners.delete(payload.request_id);
        }
      }
    });
  };

  // 获取所有可用工具
  const getAvailableTools = async () => {
    try {
      loading.value = true;
      error.value = null;
      tools.value = await invoke<McpTool[]>('get_ai_tools');
    } catch (err) {
      error.value = err as string;
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // 执行工具
  const executeTool = async (toolCall: McpToolCall): Promise<McpToolResult> => {
    try {
      loading.value = true;
      error.value = null;
      return await invoke<McpToolResult>('execute_ai_tool', { toolCall });
    } catch (err) {
      error.value = err as string;
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // 流式 HTTP 请求
  const streamFetch = async (
    method: string,
    url: string,
    headers: Record<string, string> = {},
    body: Uint8Array = new Uint8Array(),
    callbacks: {
      onChunk: (chunk: string) => void;
      onEnd: (status: number) => void;
      onError: (error: string) => void;
    }
  ): Promise<StreamResponse> => {
    try {
      loading.value = true;
      error.value = null;
      
      const response = await invoke<StreamResponse>('stream_fetch', {
        method,
        url,
        headers,
        body: Array.from(body),
      });

      // 注册回调
      streamListeners.set(response.request_id, callbacks);
      
      console.log('Stream fetch initiated, request_id:', response.request_id);

      return response;
    } catch (err) {
      error.value = err as string;
      callbacks.onError(err as string);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // 便捷方法：执行命令
  const executeCommand = async (command: string, workingDirectory?: string): Promise<McpToolResult> => {
    return executeTool({
      name: 'execute_command',
      arguments: {
        command,
        ...(workingDirectory && { working_directory: workingDirectory }),
      },
    });
  };

  // 便捷方法：读取文件
  const readFile = async (filePath: string): Promise<McpToolResult> => {
    return executeTool({
      name: 'read_file',
      arguments: { file_path: filePath },
    });
  };

  // 便捷方法：写入文件
  const writeFile = async (
    filePath: string,
    content: string,
    createDirectories: boolean = false
  ): Promise<McpToolResult> => {
    return executeTool({
      name: 'write_file',
      arguments: {
        file_path: filePath,
        content,
        create_directories: createDirectories,
      },
    });
  };

  // 便捷方法：删除文件
  const deleteFile = async (filePath: string): Promise<McpToolResult> => {
    return executeTool({
      name: 'delete_file',
      arguments: { file_path: filePath },
    });
  };

  // 便捷方法：列出目录
  const listDirectory = async (directoryPath: string, recursive: boolean = false): Promise<McpToolResult> => {
    return executeTool({
      name: 'list_directory',
      arguments: {
        directory_path: directoryPath,
        recursive,
      },
    });
  };

  // 便捷方法：获取当前目录
  const getCurrentDirectory = async (): Promise<McpToolResult> => {
    return executeTool({
      name: 'get_current_directory',
      arguments: {},
    });
  };

  // 创建 AI 代理
  const createAgent = async (): Promise<string> => {
    try {
      loading.value = true;
      error.value = null;
      return await invoke<string>('create_ai_agent');
    } catch (err) {
      error.value = err as string;
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // 生命周期钩子
  onMounted(() => {
    initializeStreamListener();
    getAvailableTools();
  });

  onUnmounted(() => {
    if (unlistenStream) {
      unlistenStream();
    }
    streamListeners.clear();
  });

  return {
    tools: tools,
    loading: loading,
    error: error,
    getAvailableTools,
    executeTool,
    streamFetch,
    executeCommand,
    readFile,
    writeFile,
    deleteFile,
    listDirectory,
    getCurrentDirectory,
    createAgent,
  };
}