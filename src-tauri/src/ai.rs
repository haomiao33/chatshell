use std::time::Duration;
use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::fs;
use std::io::{Read, Write};
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{HeaderName, HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::Window;
use tauri::Emitter;

static REQUEST_COUNTER: AtomicU32 = AtomicU32::new(0);

// HTTP Stream Client
#[derive(Debug, Clone)]
pub struct HttpStreamClient {
    client: Client,
    window: Window,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamResponse {
    pub request_id: u32,
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Serialize)]
pub struct ChunkPayload {
    pub request_id: u32,
    pub chunk: String,
}

#[derive(Clone, Serialize)]
pub struct EndPayload {
    pub request_id: u32,
    pub status: u16,
}

impl HttpStreamClient {
    pub fn new(window: Window) -> Result<Self, Box<dyn Error>> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .connect_timeout(Duration::new(3, 0))
            .build()?;

        Ok(Self { client, window })
    }

    pub async fn stream_fetch(
        &self,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<StreamResponse, String> {
        let event_name = "stream-response";
        let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);

        let mut header_map = HeaderMap::new();
        for (key, value) in &headers {
            header_map.insert(
                key.parse::<HeaderName>().map_err(|e| format!("Invalid header name: {}", e))?,
                value.parse().map_err(|e| format!("Invalid header value: {}", e))?,
            );
        }

        let method = method.parse::<reqwest::Method>()
            .map_err(|e| format!("Invalid HTTP method: {}", e))?;

        let mut request = self.client.request(method.clone(), &url)
            .headers(header_map);

        if matches!(method, reqwest::Method::POST | reqwest::Method::PUT | reqwest::Method::PATCH) {
            request = request.body(body);
        }

        match request.send().await {
            Ok(response) => {
                let mut headers = HashMap::new();
                for (name, value) in response.headers() {
                    headers.insert(
                        name.as_str().to_string(),
                        String::from_utf8_lossy(value.as_bytes()).to_string(),
                    );
                }
        
                let status = response.status().as_u16();
        
                println!("[stream_fetch] 请求成功: {} {}", status, url);
                println!("[stream_fetch] 响应头: {:?}", headers);
        
                let window = self.window.clone();
        
                tauri::async_runtime::spawn(async move {
                    let mut stream = response.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                println!("[stream_fetch] 收到 chunk: {:?}", bytes);
                                match String::from_utf8(bytes.to_vec()) {
                                    Ok(chunk_str) => {
                                        println!("[stream_fetch] chunk 转为字符串: {}", chunk_str);
                                        if let Err(e) = window.emit(event_name, ChunkPayload { request_id, chunk: chunk_str }) {
                                            eprintln!("[stream_fetch] emit chunk 失败: {}", e);
                                        }else{
                                            println!("[stream_fetch] emit chunk 成功");
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("[stream_fetch] UTF-8 解码失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => eprintln!("[stream_fetch] stream error: {}", e),
                        }
                    }
        
                    if let Err(e) = window.emit(event_name, EndPayload { request_id, status }) {
                        eprintln!("[stream_fetch] emit end 失败: {}", e);
                    }
                });
        
                Ok(StreamResponse {
                    request_id,
                    status,
                    status_text: "OK".to_string(),
                    headers,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                println!("[stream_fetch] 请求失败: {}", error_msg);
        
                let window = self.window.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = window.emit(event_name, ChunkPayload { 
                        request_id, 
                        chunk: format!("[stream_fetch error] {}", error_msg),
                    }) {
                        eprintln!("[stream_fetch] emit 错误 chunk 失败: {}", e);
                    }
                    if let Err(e) = window.emit(event_name, EndPayload { request_id, status: 0 }) {
                        eprintln!("[stream_fetch] emit 错误 end 失败: {}", e);
                    }
                });
        
                Ok(StreamResponse {
                    request_id,
                    status: 599,
                    status_text: "Error".to_string(),
                    headers: HashMap::new(),
                })
            }
        }
        
           
    }
}

// MCP Tool definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: HashMap<String, Value>,
}

// File System Tools
pub struct FileSystemTools;

impl FileSystemTools {
    pub fn get_tools() -> Vec<McpTool> {
        vec![
            McpTool {
                name: "execute_command".to_string(),
                description: "Execute a shell command and return the output".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        },
                        "working_directory": {
                            "type": "string",
                            "description": "Working directory for the command (optional)"
                        }
                    },
                    "required": ["command"]
                }),
            },
            McpTool {
                name: "get_current_directory".to_string(),
                description: "Get the current working directory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "read_file".to_string(),
                description: "Read contents of a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["file_path"]
                }),
            },
            McpTool {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        },
                        "create_directories": {
                            "type": "boolean",
                            "description": "Create parent directories if they don't exist"
                        }
                    },
                    "required": ["file_path", "content"]
                }),
            },
            McpTool {
                name: "delete_file".to_string(),
                description: "Delete a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to delete"
                        }
                    },
                    "required": ["file_path"]
                }),
            },
            McpTool {
                name: "list_directory".to_string(),
                description: "List contents of a directory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "directory_path": {
                            "type": "string",
                            "description": "Path to the directory to list"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "List recursively"
                        }
                    },
                    "required": ["directory_path"]
                }),
            },
        ]
    }

    pub fn execute_tool(tool_call: &McpToolCall) -> McpToolResult {
        match tool_call.name.as_str() {
            "execute_command" => Self::execute_command(tool_call),
            "get_current_directory" => Self::get_current_directory(),
            "read_file" => Self::read_file(tool_call),
            "write_file" => Self::write_file(tool_call),
            "delete_file" => Self::delete_file(tool_call),
            "list_directory" => Self::list_directory(tool_call),
            _ => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Unknown tool: {}", tool_call.name),
                }],
                is_error: true,
            },
        }
    }

    fn execute_command(tool_call: &McpToolCall) -> McpToolResult {
        let command = tool_call.arguments.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let working_dir = tool_call.arguments.get("working_directory")
            .and_then(|v| v.as_str());

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(&["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(&["-c", command]);
            c
        };

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let success = output.status.success();

                let text = if success {
                    stdout.to_string()
                } else {
                    format!("Command failed with exit code: {:?}\nStdout: {}\nStderr: {}", 
                           output.status.code(), stdout, stderr)
                };

                McpToolResult {
                    content: vec![McpContent {
                        r#type: "text".to_string(),
                        text,
                    }],
                    is_error: !success,
                }
            }
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to execute command: {}", e),
                }],
                is_error: true,
            },
        }
    }

    fn get_current_directory() -> McpToolResult {
        match std::env::current_dir() {
            Ok(dir) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: dir.to_string_lossy().to_string(),
                }],
                is_error: false,
            },
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to get current directory: {}", e),
                }],
                is_error: true,
            },
        }
    }

    fn read_file(tool_call: &McpToolCall) -> McpToolResult {
        let file_path = tool_call.arguments.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match fs::read_to_string(file_path) {
            Ok(content) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: content,
                }],
                is_error: false,
            },
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to read file {}: {}", file_path, e),
                }],
                is_error: true,
            },
        }
    }

    fn write_file(tool_call: &McpToolCall) -> McpToolResult {
        let file_path = tool_call.arguments.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content = tool_call.arguments.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let create_directories = tool_call.arguments.get("create_directories")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if create_directories {
            if let Some(parent) = Path::new(file_path).parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return McpToolResult {
                        content: vec![McpContent {
                            r#type: "text".to_string(),
                            text: format!("Failed to create directories: {}", e),
                        }],
                        is_error: true,
                    };
                }
            }
        }

        match fs::write(file_path, content) {
            Ok(_) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Successfully wrote to file: {}", file_path),
                }],
                is_error: false,
            },
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to write file {}: {}", file_path, e),
                }],
                is_error: true,
            },
        }
    }

    fn delete_file(tool_call: &McpToolCall) -> McpToolResult {
        let file_path = tool_call.arguments.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match fs::remove_file(file_path) {
            Ok(_) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Successfully deleted file: {}", file_path),
                }],
                is_error: false,
            },
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to delete file {}: {}", file_path, e),
                }],
                is_error: true,
            },
        }
    }

    fn list_directory(tool_call: &McpToolCall) -> McpToolResult {
        let directory_path = tool_call.arguments.get("directory_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = tool_call.arguments.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match Self::list_dir_recursive(directory_path, recursive) {
            Ok(entries) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: entries.join("\n"),
                }],
                is_error: false,
            },
            Err(e) => McpToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to list directory {}: {}", directory_path, e),
                }],
                is_error: true,
            },
        }
    }

    fn list_dir_recursive(dir_path: &str, recursive: bool) -> Result<Vec<String>, Box<dyn Error>> {
        let mut entries = Vec::new();
        let dir = fs::read_dir(dir_path)?;

        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();

            if path.is_dir() {
                entries.push(format!("{}/", path_str));
                if recursive {
                    match Self::list_dir_recursive(&path_str, true) {
                        Ok(sub_entries) => entries.extend(sub_entries),
                        Err(_) => {} // Skip directories we can't read
                    }
                }
            } else {
                entries.push(path_str);
            }
        }

        Ok(entries)
    }
}

// MCP Server
pub struct McpServer {
    tools: Vec<McpTool>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tools: FileSystemTools::get_tools(),
        }
    }

    pub fn get_tools(&self) -> &[McpTool] {
        &self.tools
    }

    pub fn execute_tool(&self, tool_call: &McpToolCall) -> McpToolResult {
        FileSystemTools::execute_tool(tool_call)
    }
}

// AI Agent
pub struct AiAgent {
    mcp_server: McpServer,
    http_client: Option<HttpStreamClient>,
}

impl AiAgent {
    pub fn new(window: Option<Window>) -> Result<Self, Box<dyn Error>> {
        let http_client = if let Some(w) = window {
            Some(HttpStreamClient::new(w)?)
        } else {
            None
        };

        Ok(Self {
            mcp_server: McpServer::new(),
            http_client,
        })
    }

    pub fn get_available_tools(&self) -> &[McpTool] {
        self.mcp_server.get_tools()
    }

    pub fn execute_tool(&self, tool_call: &McpToolCall) -> McpToolResult {
        self.mcp_server.execute_tool(tool_call)
    }

    pub async fn stream_http_request(
        &self,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<StreamResponse, String> {
        match &self.http_client {
            Some(client) => client.stream_fetch(method, url, headers, body).await,
            None => Err("HTTP client not initialized".to_string()),
        }
    }
}


// Tauri command exports
#[tauri::command]
pub async fn stream_fetch(
    window: Window,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> Result<StreamResponse, String> {
    let client = HttpStreamClient::new(window).map_err(|e| e.to_string())?;
    client.stream_fetch(method, url, headers, body).await
}

#[tauri::command]
pub fn get_ai_tools() -> Vec<McpTool> {
    FileSystemTools::get_tools()
}

#[tauri::command]
pub fn execute_ai_tool(tool_call: McpToolCall) -> McpToolResult {
    FileSystemTools::execute_tool(&tool_call)
}

#[tauri::command]
pub fn create_ai_agent() -> Result<String, String> {
    match AiAgent::new(None) {
        Ok(_) => Ok("AI Agent created successfully".to_string()),
        Err(e) => Err(e.to_string()),
    }
}