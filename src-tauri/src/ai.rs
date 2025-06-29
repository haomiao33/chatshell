use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use crate::commands::TERMINAL_MANAGER;

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
            model: "gpt-3.5-turbo".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

// MCP Server 功能 - 终端控制
pub struct TerminalMCPServer;

impl TerminalMCPServer {
    pub async fn execute_command(command: &str) -> Result<String, String> {
        let mut manager = TERMINAL_MANAGER.lock().unwrap();
        
        if let Some(session_id) = manager.get_active_session().cloned() {
            // 通知插件命令开始
            if let Some(session) = manager.sessions.get_mut(&session_id) {
                for plugin in &mut session.plugins {
                    plugin.on_command_start(command, &session_id);
                }
            }
            
            let command_with_newline = format!("{}\n", command);
            manager.write_to_session(&session_id, &command_with_newline)?;
            
            Ok(format!("Command '{}' sent to terminal", command))
        } else {
            Err("No active terminal session".to_string())
        }
    }

    pub async fn get_current_directory() -> Result<String, String> {
        let manager = TERMINAL_MANAGER.lock().unwrap();
        
        if let Some(session_id) = manager.get_active_session() {
            if let Some(session) = manager.sessions.get(session_id) {
                Ok(session.config.working_dir.clone().unwrap_or_else(|| "Unknown".to_string()))
            } else {
                Err("Session not found".to_string())
            }
        } else {
            Err("No active terminal session".to_string())
        }
    }

    pub async fn list_files() -> Result<String, String> {
        let current_dir = Self::get_current_directory().await?;
        
        match std::fs::read_dir(&current_dir) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            format!("📁 {}", name)
                        } else {
                            format!("📄 {}", name)
                        }
                    })
                    .collect();
                Ok(format!("Files in {}:\n{}", current_dir, files.join("\n")))
            }
            Err(e) => Err(format!("Failed to read directory: {}", e))
        }
    }
}

// AI Agent
pub struct AIAgent {
    config: AIConfig,
    client: reqwest::Client,
}

impl AIAgent {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn chat(&self, user_message: &str) -> Result<String, String> {
        if self.config.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        // 构建系统提示词，包含MCP功能说明
        let system_prompt = r#"你是一个智能终端助手，可以帮助用户执行终端命令。你可以：

1. 理解用户的自然语言请求
2. 将其转换为合适的终端命令
3. 执行命令并返回结果

可用的MCP功能：
- execute_command(command): 在终端中执行命令
- get_current_directory(): 获取当前工作目录
- list_files(): 列出当前目录的文件

请根据用户的需求，选择合适的命令执行。如果用户只是想查看信息，直接回答；如果需要执行命令，使用相应的MCP功能。"#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
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
        };

        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(choice) = chat_response.choices.first() {
            let content = &choice.message.content;
            
            // 检查是否需要执行MCP命令
            if content.contains("execute_command") {
                return self.handle_mcp_commands(content).await;
            }
            
            Ok(content.clone())
        } else {
            Err("No response from AI".to_string())
        }
    }

    async fn handle_mcp_commands(&self, content: &str) -> Result<String, String> {
        // 简单的MCP命令解析
        if content.contains("list_files") {
            TerminalMCPServer::list_files().await
        } else if content.contains("get_current_directory") {
            TerminalMCPServer::get_current_directory().await
        } else if content.contains("execute_command") {
            // 提取命令内容
            if let Some(command) = self.extract_command(content) {
                TerminalMCPServer::execute_command(&command).await
            } else {
                Ok("No command found to execute".to_string())
            }
        } else {
            Ok(content.to_string())
        }
    }

    fn extract_command(&self, content: &str) -> Option<String> {
        // 简单的命令提取逻辑
        if let Some(start) = content.find("execute_command(") {
            if let Some(end) = content[start..].find(')') {
                let command_content = &content[start + 16..start + end];
                // 移除引号
                let command = command_content.trim_matches('"').trim_matches('\'');
                return Some(command.to_string());
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