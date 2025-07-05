use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::commands::TERMINAL_MANAGER;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tokio::process::Command as TokioCommand;
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};
use std::fs;
use std::path::Path;

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
    pub message: ChatMessage,
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

// 系统信息检测
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub platform: String,
    pub shell: String,
    pub current_dir: String,
    pub user: String,
    pub hostname: String,
}

impl SystemInfo {
    pub fn detect() -> Self {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let platform = format!("{}-{}", os, arch);
        
        // 检测默认shell
        let shell = match os.as_str() {
            "windows" => "cmd".to_string(),
            "macos" | "linux" => {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
            }
            _ => "sh".to_string(),
        };
        
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .to_string_lossy()
            .to_string();
        
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "localhost".to_string());
        
        Self {
            os,
            arch,
            platform,
            shell,
            current_dir,
            user,
            hostname,
        }
    }
    
    pub fn to_prompt_context(&self) -> String {
        format!(
            r#"当前系统信息：
- 操作系统：{} ({})
- 架构：{}
- 默认Shell：{}
- 当前目录：{}
- 用户：{}
- 主机名：{}

命令执行规范：
- Windows系统使用 cmd.exe 或 PowerShell 命令
- macOS/Linux系统使用 bash/zsh/sh 命令
- 文件路径使用对应系统的路径分隔符
- 权限相关命令请提醒用户可能需要管理员权限"#,
            self.os, self.platform, self.arch, self.shell, 
            self.current_dir, self.user, self.hostname
        )
    }
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

// 增强的 MCP Server
#[derive(Debug, Clone)]
pub struct EnhancedTerminalMCPServer {
    pub system_info: SystemInfo,
}

impl EnhancedTerminalMCPServer {
    pub fn new() -> Self {
        Self {
            system_info: SystemInfo::detect(),
        }
    }
}

#[tool(tool_box)]
impl EnhancedTerminalMCPServer {
    #[tool(description = "在终端中执行命令。支持两种模式：terminal（持久终端会话）和 process（新进程执行）")]
    async fn execute_command(&self, #[tool(aggr)] req: ExecuteCommandRequest) -> Result<String, String> {
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
    
    #[tool(description = "读取文件内容")]
    async fn read_file(&self, #[tool(aggr)] req: ReadFileRequest) -> Result<String, String> {
        match fs::read_to_string(&req.path) {
            Ok(content) => Ok(format!("文件内容 ({}):\n{}", req.path, content)),
            Err(e) => Err(format!("读取文件失败: {}", e)),
        }
    }
    
    #[tool(description = "写入文件内容")]
    async fn write_file(&self, #[tool(aggr)] req: WriteFileRequest) -> Result<String, String> {
        let result = if req.append.unwrap_or(false) {
            fs::write(&req.path, &req.content)
        } else {
            fs::write(&req.path, &req.content)
        };
        
        match result {
            Ok(_) => Ok(format!("文件写入成功: {}", req.path)),
            Err(e) => Err(format!("写入文件失败: {}", e)),
        }
    }
    
    #[tool(description = "查找文件")]
    async fn find_files(&self, #[tool(aggr)] req: FindFilesRequest) -> Result<String, String> {
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
    
    #[tool(description = "获取当前系统信息")]
    async fn get_system_info(&self) -> String {
        format!("系统信息:\n{}", self.system_info.to_prompt_context())
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
        let (shell, shell_arg) = match self.system_info.os.as_str() {
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

#[tool(tool_box)]
impl ServerHandler for EnhancedTerminalMCPServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "enhanced-terminal-mcp".to_string(),
            version: "1.0.0".to_string(),
            instructions: Some("增强的终端 MCP 服务器，支持命令执行、文件操作和系统信息查询".to_string()),
            ..Default::default()
        }
    }
}

// AI Agent
pub struct AIAgent {
    config: AIConfig,
    client: reqwest::Client,
    system_info: SystemInfo,
    mcp_server: EnhancedTerminalMCPServer,
}

impl AIAgent {
    pub fn new(config: AIConfig) -> Self {
        let system_info = SystemInfo::detect();
        let mcp_server = EnhancedTerminalMCPServer::new();
        
        Self {
            config,
            client: reqwest::Client::new(),
            system_info,
            mcp_server,
        }
    }

    pub async fn chat(&self, user_message: &str) -> Result<String, String> {
        if self.config.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        // 构建增强的系统提示词
        let system_prompt = format!(
            r#"你是一个智能终端助手，可以帮助用户执行各种终端命令和文件操作。

{}

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

5. **get_system_info** - 获取系统信息

命令执行指南：
- 对于需要实时交互的命令（如 vim、top、htop 等），使用 execution_type="terminal"
- 对于一次性命令（如 ls、cat、grep 等），使用 execution_type="process"
- 长时间运行的命令建议设置适当的 timeout_seconds
- 文件操作前请确认路径存在和权限充足
- 回答时使用 Markdown 格式，让内容更易读

使用示例：
```json
{{"command": "ls -la", "execution_type": "process", "timeout_seconds": 10}}
```

请根据用户需求选择合适的工具执行任务。"#,
            self.system_info.to_prompt_context()
        );

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

        let url = format!("{}/chat/completions", self.config.base_url);
        println!("[RUST] Making request to: {}", url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
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

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(choice) = chat_response.choices.first() {
            let content = &choice.message.content;
            
            // 检查是否需要执行MCP命令
            if self.contains_mcp_commands(content) {
                return self.handle_mcp_commands(content).await;
            }
            
            Ok(content.clone())
        } else {
            Err("No response from AI".to_string())
        }
    }

    fn contains_mcp_commands(&self, content: &str) -> bool {
        content.contains("execute_command") || 
        content.contains("read_file") || 
        content.contains("write_file") || 
        content.contains("find_files") || 
        content.contains("get_system_info")
    }

    async fn handle_mcp_commands(&self, content: &str) -> Result<String, String> {
        // 这里可以实现更复杂的MCP命令解析和执行逻辑
        // 目前先简化处理
        
        if content.contains("execute_command") {
            if let Some(command) = self.extract_command_from_content(content) {
                return self.mcp_server.execute_in_process(&command, Duration::from_secs(30)).await;
            }
        }
        
        if content.contains("get_system_info") {
            return Ok(self.mcp_server.get_system_info().await);
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
pub async fn get_ai_config() -> Result<Option<AIConfig>, String> {
    let agent_guard = AI_AGENT.lock().await;
    
    if let Some(agent) = &*agent_guard {
        Ok(Some(agent.config.clone()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo::detect())
}