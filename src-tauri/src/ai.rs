use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::commands::TERMINAL_MANAGER;
use std::borrow::Cow;
use std::time::Duration;
use tokio::time::timeout;
use tokio::process::Command as TokioCommand;
use rmcp::{ServerHandler, model::ServerInfo, schemars};
use std::fs;
use std::path::Path;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tauri::{Emitter, EventTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub message: Option<ChatMessage>,
    pub delta: Option<ChatMessage>,
    pub finish_reason: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
    pub param: Option<String>,
    pub code: String,
}

// 命令执行类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionType {
    Terminal,    // 在当前终端会话中执行
    Process,     // 启动新进程执行（非持久）
}

// MCP 工具请求结构
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteCommandRequest {
    #[schemars(description = "要执行的命令")]
    pub command: String,
    #[schemars(description = "执行类型：terminal（终端会话）或 process（新进程）")]
    pub execution_type: Option<String>,
    #[schemars(description = "命令超时时间（秒），仅对 process 类型有效")]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadFileRequest {
    #[schemars(description = "文件路径")]
    pub path: String,
    #[schemars(description = "编码格式，默认 utf-8")]
    pub encoding: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteFileRequest {
    #[schemars(description = "文件路径")]
    pub path: String,
    #[schemars(description = "文件内容")]
    pub content: String,
    #[schemars(description = "编码格式，默认 utf-8")]
    pub encoding: Option<String>,
    #[schemars(description = "是否追加内容，默认覆盖")]
    pub append: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindFilesRequest {
    #[schemars(description = "搜索目录")]
    pub directory: String,
    #[schemars(description = "文件名模式（支持通配符）")]
    pub pattern: Option<String>,
    #[schemars(description = "文件扩展名")]
    pub extension: Option<String>,
    #[schemars(description = "是否递归搜索子目录")]
    pub recursive: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaMessage {
    pub content: Option<String>,
    // role 字段是可选的（首次 delta 中可能有）
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub delta: Option<DeltaMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
}


// 增强的 MCP Server
#[derive(Debug, Clone)]
pub struct EnhancedTerminalMCPServer;

impl EnhancedTerminalMCPServer {
    pub fn new() -> Self {
        Self
    }
}

// #[tool(tool_box)]
impl EnhancedTerminalMCPServer {
    // #[tool(description = "在终端中执行命令。支持两种模式：terminal（持久终端会话）和 process（新进程执行）")]
    async fn execute_command(&self,  req: ExecuteCommandRequest) -> Result<String, String> {
        let execution_type = match req.execution_type.as_deref() {
            Some("terminal") => ExecutionType::Terminal,
            Some("process") => ExecutionType::Process,
            _ => ExecutionType::Process, // 默认使用 process 模式
        };
        
        match execution_type {
            ExecutionType::Terminal => {
                self.execute_in_terminal(&req.command).await
            }
            ExecutionType::Process => {
                let timeout_duration = Duration::from_secs(req.timeout_seconds.unwrap_or(30));
                self.execute_in_process(&req.command, timeout_duration).await
            }
        }
    }
    
    // #[tool(description = "读取文件内容")]
    async fn read_file(&self, req: ReadFileRequest) -> Result<String, String> {
        match fs::read_to_string(&req.path) {
            Ok(content) => Ok(format!("文件内容 ({}):\n{}", req.path, content)),
            Err(e) => Err(format!("读取文件失败: {}", e)),
        }
    }
    
    // #[tool(description = "写入文件内容")]
    async fn write_file(&self, req: WriteFileRequest) -> Result<String, String> {
        let result = if req.append.unwrap_or(false) {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&req.path)
                .map_err(|e| format!("打开文件失败: {}", e))?;
            file.write_all(req.content.as_bytes())
                .map_err(|e| format!("写入文件失败: {}", e))
        } else {
            fs::write(&req.path, &req.content)
                .map_err(|e| format!("写入文件失败: {}", e))
        };
        
        match result {
            Ok(_) => Ok(format!("文件写入成功: {}", req.path)),
            Err(e) => Err(e),
        }
    }
    
    // #[tool(description = "查找文件")]
    async fn find_files(&self,  req: FindFilesRequest) -> Result<String, String> {
        let dir = Path::new(&req.directory);
        if !dir.exists() {
            return Err(format!("目录不存在: {}", req.directory));
        }
        
        let mut files = Vec::new();
        let recursive = req.recursive.unwrap_or(false);
        
        if recursive {
            self.find_files_recursive(dir, &req.pattern, &req.extension, &mut files)?;
        } else {
            self.find_files_in_dir(dir, &req.pattern, &req.extension, &mut files)?;
        }
        
        if files.is_empty() {
            Ok("未找到匹配的文件".to_string())
        } else {
            Ok(format!("找到 {} 个文件:\n{}", files.len(), files.join("\n")))
        }
    }
}

impl EnhancedTerminalMCPServer {
    async fn execute_in_terminal(&self, command: &str) -> Result<String, String> {
        let mut manager = TERMINAL_MANAGER.lock().unwrap();
        
        if let Some(session_id) = manager.get_active_session().cloned() {
            // 通知插件命令开始
            if let Some(session) = manager.get_session_mut(&session_id) {
                for plugin in &mut session.plugins {
                    plugin.on_command_start(command, &session_id);
                }
            }
            
            let command_with_newline = format!("{}\n", command);
            manager.write_to_session(&session_id, &command_with_newline)?;
            
            Ok(format!("命令 '{}' 已发送到终端会话", command))
        } else {
            Err("没有活动的终端会话".to_string())
        }
    }
    
    async fn execute_in_process(&self, command: &str, timeout_duration: Duration) -> Result<String, String> {
        let os = std::env::consts::OS;
        let (shell, shell_arg) = match os {
            "windows" => ("cmd", "/C"),
            _ => ("sh", "-c"),
        };
        
        let mut cmd = TokioCommand::new(shell);
        cmd.arg(shell_arg).arg(command);
        cmd.kill_on_drop(true);
        
        let output_future = cmd.output();
        
        match timeout(timeout_duration, output_future).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                let mut result = String::new();
                if !stdout.is_empty() {
                    result.push_str(&format!("输出:\n{}\n", stdout));
                }
                if !stderr.is_empty() {
                    result.push_str(&format!("错误:\n{}\n", stderr));
                }
                result.push_str(&format!("退出代码: {}", output.status.code().unwrap_or(-1)));
                
                Ok(result)
            }
            Ok(Err(e)) => Err(format!("命令执行失败: {}", e)),
            Err(_) => Err(format!("命令执行超时 ({}秒)", timeout_duration.as_secs())),
        }
    }
    
    fn find_files_recursive(&self, dir: &Path, pattern: &Option<String>, extension: &Option<String>, files: &mut Vec<String>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            
            if path.is_dir() {
                self.find_files_recursive(&path, pattern, extension, files)?;
            } else if path.is_file() {
                if self.matches_criteria(&path, pattern, extension) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
        
        Ok(())
    }
    
    fn find_files_in_dir(&self, dir: &Path, pattern: &Option<String>, extension: &Option<String>, files: &mut Vec<String>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && self.matches_criteria(&path, pattern, extension) {
                files.push(path.to_string_lossy().to_string());
            }
        }
        
        Ok(())
    }
    
    fn matches_criteria(&self, path: &Path, pattern: &Option<String>, extension: &Option<String>) -> bool {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // 检查扩展名
        if let Some(ext) = extension {
            if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                if !file_ext.eq_ignore_ascii_case(ext) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // 检查文件名模式（简单的通配符支持）
        if let Some(pat) = pattern {
            if pat.contains('*') {
                let parts: Vec<&str> = pat.split('*').collect();
                if parts.len() == 2 {
                    let prefix = parts[0];
                    let suffix = parts[1];
                    return filename.starts_with(prefix) && filename.ends_with(suffix);
                }
            } else {
                return filename.contains(pat);
            }
        }
        
        true
    }
}

// #[tool(tool_box)]
impl EnhancedTerminalMCPServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "enhanced-terminal-mcp".to_string(),
                version: "1.0.0".to_string(),
        },
        instructions: Some("增强的终端 MCP 服务器，支持命令执行、文件操作等功能".to_string()),
    }
}
}

// AI Agent with Stream Support
pub struct AIAgent {
    config: AIConfig,
    client: reqwest::Client,
    mcp_server: EnhancedTerminalMCPServer,
}

impl AIAgent {
    pub fn new(config: AIConfig) -> Self {
        let mcp_server = EnhancedTerminalMCPServer::new();
        
        Self {
            config,
            client: reqwest::Client::new(),
            mcp_server,
        }
    }

    pub async fn chat(&self, user_message: &str) -> Result<String, String> {
        if self.config.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        let system_prompt = self.build_system_prompt();
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ];

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: false,
        };

        let response = self.send_request(&request).await?;
        
        if let Some(choice) = response.choices.first() {
            if let Some(message) = &choice.message {
                let content = &message.content;
                
                // 检查是否需要执行MCP命令
                if self.contains_mcp_commands(content) {
                    return self.handle_mcp_commands(content).await;
                }
                
                Ok(content.clone())
            } else {
                Err("No message content in response".to_string())
            }
        } else {
            Err("No response from AI".to_string())
        }
    }

    pub async fn chat_stream(&self, user_message: &str) -> Result<mpsc::Receiver<String>, String> {
        if self.config.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        let system_prompt = self.build_system_prompt();
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ];

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: true,
        };

        let (tx, rx) = mpsc::channel(100);
        
        let client = self.client.clone();
        let config = self.config.clone();
        
        if let Err(e) = Self::handle_stream_response(client, config, request, tx).await {
            eprintln!("Stream error: {}", e);
        }

        Ok(rx)
    }
    async fn handle_stream_response(
        client: reqwest::Client,
        config: AIConfig,
        request: ChatRequest,
        tx: mpsc::Sender<String>,
    ) -> Result<(), String> {
        let url = format!("{}/chat/completions", config.base_url);
        
        println!("[handle_stream_response] 请求 URL: {}", url);
    
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
    
        println!("[handle_stream_response] 状态码: {}", response.status());
    
        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_else(|_: reqwest::Error| "<无法读取正文>".to_string());
            println!("[handle_stream_response] 响应失败内容: {}", text);
             return Err(format!("流读取失败"));
        }
    
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
    
        println!("[handle_stream_response] 开始处理 stream");
    
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    println!("[handle_stream_response] 流读取失败: {}", e);
                    return Err(format!("流读取失败: {}", e));
                }
            };
    
            let chunk_str = String::from_utf8_lossy(&chunk);
            println!("[handle_stream_response] 接收到原始 chunk: {}", chunk_str);
    
            buffer.push_str(&chunk_str);
    
            // 拆行处理
            for line in chunk_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
    
                // 打印每一行
                println!("[handle_stream_response] 行数据: {}", line);
    
                // 处理 SSE 格式中的 data: 前缀
                let json_part = if line.starts_with("data:") {
                    &line[5..].trim()
                } else {
                    line
                };
    
                if json_part == "[DONE]" {
                    println!("[handle_stream_response] 接收到 [DONE]，结束流");
                    break;
                }
    
                match serde_json::from_str::<StreamChunk>(json_part) {
                    Ok(chunk_data) => {
                        if let Some(choice) = chunk_data.choices.first() {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    println!("[handle_stream_response] Stream 内容片段: {}", content);
                                    if tx.send(content.clone()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        
                    }
                    Err(e) => {
                        println!("[handle_stream_response] JSON 解析失败: {}", e);
                        println!("[handle_stream_response] 原始 JSON: {}", json_part);
                    }
                }
            }
        }
    
        println!("[handle_stream_response] 流处理完成");
        Ok(())
    }
    

    async fn send_request(&self, request: &ChatRequest) -> Result<ChatResponse, String> {
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| format!("Failed to get response text: {}", e))?;

        if !status.is_success() {
            match serde_json::from_str::<ErrorResponse>(&response_text) {
                Ok(error_response) => {
                    return Err(error_response.error.message);
                }
                Err(_) => {
                    return Err(format!("API Error ({}): {}", status, response_text));
                }
            }
        }

        serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    fn build_system_prompt(&self) -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .to_string_lossy()
            .to_string();

        format!(
            r#"你是一个智能终端助手，可以帮助用户执行各种终端命令和文件操作。

当前系统信息：
- 操作系统：{} ({})
- 架构：{}
- 当前目录：{}

你拥有以下 MCP 工具功能：

1. **execute_command** - 执行命令
   - command: 要执行的命令（必需）
   - execution_type: 执行类型
     - "terminal": 在持久终端会话中执行，适合交互式命令
     - "process": 启动新进程执行（默认），适合一次性命令
   - timeout_seconds: 超时时间（秒），仅对 process 类型有效，默认30秒

2. **read_file** - 读取文件内容
   - path: 文件路径（必需）
   - encoding: 编码格式，默认 utf-8

3. **write_file** - 写入文件内容
   - path: 文件路径（必需）
   - content: 文件内容（必需）
   - encoding: 编码格式，默认 utf-8
   - append: 是否追加内容，默认覆盖

4. **find_files** - 查找文件
   - directory: 搜索目录（必需）
   - pattern: 文件名模式（支持通配符 *）
   - extension: 文件扩展名
   - recursive: 是否递归搜索子目录

命令执行指南：
- 对于需要实时交互的命令（如 vim、top、htop 等），使用 execution_type="terminal"
- 对于一次性命令（如 ls、cat、grep 等），使用 execution_type="process"
- 长时间运行的命令建议设置适当的 timeout_seconds
- 文件操作前请确认路径存在和权限充足
- 回答时使用 Markdown 格式，让内容更易读

请根据用户需求选择合适的工具执行任务。"#,
            os, format!("{}-{}", os, arch), arch, current_dir
        )
    }

    fn contains_mcp_commands(&self, content: &str) -> bool {
        content.contains("execute_command") || 
        content.contains("read_file") || 
        content.contains("write_file") || 
        content.contains("find_files")
    }

    async fn handle_mcp_commands(&self, content: &str) -> Result<String, String> {
        if content.contains("execute_command") {
            if let Some(command) = self.extract_command_from_content(content) {
                return self.mcp_server.execute_in_process(&command, Duration::from_secs(30)).await;
            }
        }
        
        // 如果没有找到具体的MCP命令，返回原始内容
        Ok(content.to_string())
    }

    fn extract_command_from_content(&self, content: &str) -> Option<String> {
        // 简化的命令提取逻辑
        if let Some(start) = content.find("execute_command") {
            if let Some(command_start) = content[start..].find("\"command\":") {
                let search_start = start + command_start + 11; // 跳过 "command":"
                if let Some(command_end) = content[search_start..].find('"') {
                    return Some(content[search_start..search_start + command_end].to_string());
                }
            }
        }
        None
    }
}

// 全局AI Agent实例
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static AI_AGENT: Lazy<Arc<Mutex<Option<AIAgent>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

// Tauri 命令
#[tauri::command]
pub async fn configure_ai(config: AIConfig) -> Result<(), String> {
    let agent = AIAgent::new(config);
    let mut global_agent = AI_AGENT.lock().await;
    *global_agent = Some(agent);
    Ok(())
}

#[tauri::command]
pub async fn chat_with_ai(message: String) -> Result<String, String> {
    let agent_guard = AI_AGENT.lock().await;
    
    if let Some(agent) = &*agent_guard {
        agent.chat(&message).await
    } else {
        Err("AI Agent not configured. Please configure API key first.".to_string())
    }
}

#[tauri::command]
pub async fn chat_with_ai_stream(app: tauri::AppHandle, message: String) -> Result<(), String> {
    let agent_guard = AI_AGENT.lock().await;

    if let Some(agent) = &*agent_guard {
        let mut receiver = agent.chat_stream(&message).await?;
        drop(agent_guard); // 释放锁
        
        // 异步任务中发送消息片段
        tauri::async_runtime::spawn(async move {
            while let Some(chunk) = receiver.recv().await {
                let _ =  app.emit("ai-stream-chunk",  chunk.clone());
            }
            let _ = app.emit("ai-stream-end", "");
        });

        Ok(())
    } else {
        Err("AI Agent not configured. Please configure API key first.".to_string())
    }
}

#[tauri::command]
pub async fn get_ai_config() -> Result<Option<AIConfig>, String> {
    let agent_guard = AI_AGENT.lock().await;
    
    if let Some(agent) = &*agent_guard {
        Ok(Some(agent.config.clone()))
    } else {
        Ok(None)
    }
}