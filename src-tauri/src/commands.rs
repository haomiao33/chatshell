// src/commands.rs
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::io::{Read};
use std::os::unix::io::AsRawFd;
use std::io::Error;

// 全局终端管理器
pub static TERMINAL_MANAGER: Lazy<Arc<Mutex<TerminalManager>>> = 
    Lazy::new(|| Arc::new(Mutex::new(TerminalManager::new())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub shell: String,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub columns: u16,
    pub rows: u16,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        let mut env: HashMap<String, String> = std::env::vars().collect();
        
        // 设置一些默认环境变量
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        
        Self {
            shell: get_default_shell(),
            env,
            working_dir: std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
            columns: 80,
            rows: 24,
        }
    }
}

// 终端会话
pub struct TerminalSession {
    pub id: String,
    pub pty: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    pub config: TerminalConfig,
    pub plugins: Vec<Box<dyn TerminalPlugin + Send + Sync>>,
    pub app_handle: AppHandle,
}

// 插件系统接口
pub trait TerminalPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_command_start(&mut self, _command: &str, _session_id: &str) {}
    fn on_output(&mut self, output: &str, _session_id: &str) -> String {
        output.to_string() // 默认不处理
    }
    fn on_command_end(&mut self, _exit_code: Option<i32>, _session_id: &str) {}
    fn on_session_start(&mut self, _session_id: &str) {}
    fn on_session_end(&mut self, _session_id: &str) {}
}

// 内置插件：命令历史
pub struct HistoryPlugin {
    history: Vec<String>,
}

impl HistoryPlugin {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

impl TerminalPlugin for HistoryPlugin {
    fn name(&self) -> &str {
        "history"
    }

    fn on_command_start(&mut self, command: &str, _session_id: &str) {
        if !command.trim().is_empty() {
            self.history.push(command.to_string());
        }
    }
}

// 内置插件：颜色高亮
pub struct ColorPlugin;

impl TerminalPlugin for ColorPlugin {
    fn name(&self) -> &str {
        "color"
    }

    fn on_output(&mut self, output: &str, _session_id: &str) -> String {
        // 简单的颜色处理，可以扩展
        output.to_string()
    }
}

// 终端管理器
pub struct TerminalManager {
    pub sessions: HashMap<String, TerminalSession>,
    pub active_session: Option<String>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_session: None,
        }
    }

    pub fn create_session(&mut self, config: TerminalConfig, app_handle: AppHandle) -> Result<String, String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        // 创建 PTY
        let pty_system = portable_pty::native_pty_system();
        let pty_pair = pty_system
            .openpty(portable_pty::PtySize {
                rows: config.rows,
                cols: config.columns,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to create PTY: {}", e))?;

        // 启动 shell
        let mut cmd = portable_pty::CommandBuilder::new(&config.shell);
        
        // 设置环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        
        // 设置工作目录
        if let Some(ref dir) = config.working_dir {
            cmd.cwd(dir);
        }

        let _child = pty_pair.slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        // 创建插件实例
        let mut plugins: Vec<Box<dyn TerminalPlugin + Send + Sync>> = Vec::new();
        plugins.push(Box::new(HistoryPlugin::new()));
        plugins.push(Box::new(ColorPlugin));
        
        // 可以在这里添加更多插件
        // plugins.push(Box::new(crate::plugins::TimingPlugin::new()));
        // plugins.push(Box::new(crate::plugins::GitPlugin::new()));
        // plugins.push(Box::new(crate::plugins::AliasPlugin::new()));

        let mut session = TerminalSession {
            id: session_id.clone(),
            pty: Arc::new(Mutex::new(pty_pair.master)),
            config,
            plugins,
            app_handle: app_handle.clone(),
        };

        // 启动输出监听
        self.start_output_listener(&session_id, &session)?;
        
        // 通知插件会话开始
        for plugin in &mut session.plugins {
            plugin.on_session_start(&session_id);
        }

        self.sessions.insert(session_id.clone(), session);
        self.active_session = Some(session_id.clone());

        Ok(session_id)
    }

    fn start_output_listener(&self, session_id: &str, session: &TerminalSession) -> Result<(), String> {
        let session_id = session_id.to_string();
        let app_handle = session.app_handle.clone();
        let pty = session.pty.clone();
        
        tokio::spawn(async move {
            let mut reader = {
                let pty_guard = pty.lock().unwrap();
                pty_guard.try_clone_reader().unwrap()
            };

            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        
                        // 应用插件处理
                        let processed_output = {
                            let mut manager = TERMINAL_MANAGER.lock().unwrap();
                            if let Some(session) = manager.sessions.get_mut(&session_id) {
                                let mut result = output.to_string();
                                for plugin in &mut session.plugins {
                                    result = plugin.on_output(&result, &session_id);
                                }
                                result
                            } else {
                                output.to_string()
                            }
                        };

                        // 发送到前端
                        if let Err(e) = app_handle.emit("terminal-output", &processed_output) {
                            eprintln!("Failed to emit terminal output: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from PTY: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn write_to_session(&mut self, session_id: &str, data: &str) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            let pty = session.pty.clone();
            let pty_guard = pty.lock().unwrap();
            let fd = pty_guard.as_raw_fd().expect("pty fd missing");
            let bytes = data.as_bytes();
            let ret = unsafe { libc::write(fd, bytes.as_ptr() as *const _, bytes.len()) };
            if ret < 0 {
                return Err(format!("Failed to write to PTY: {}", Error::last_os_error()));
            }
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn resize_session(&mut self, session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            let pty = session.pty.clone();
            let pty_guard = pty.lock().unwrap();
            pty_guard.resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            }).map_err(|e| format!("Failed to resize PTY: {}", e))?;
            
            session.config.rows = rows;
            session.config.columns = cols;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(mut session) = self.sessions.remove(session_id) {
            // 通知插件会话结束
            for plugin in &mut session.plugins {
                plugin.on_session_end(session_id);
            }
            
            if self.active_session.as_ref() == Some(&session_id.to_string()) {
                self.active_session = None;
            }
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn get_active_session(&self) -> Option<&String> {
        self.active_session.as_ref()
    }

    pub fn set_active_session(&mut self, session_id: String) -> Result<(), String> {
        if self.sessions.contains_key(&session_id) {
            self.active_session = Some(session_id);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }
}

// Tauri 命令
#[tauri::command]
pub async fn create_shell(app_handle: AppHandle) -> Result<String, String> {
    let config = TerminalConfig::default();
    let mut manager = TERMINAL_MANAGER.lock().unwrap();
    manager.create_session(config, app_handle)
}

#[tauri::command]
pub async fn run_command_pty(command: String) -> Result<(), String> {
    let mut manager = TERMINAL_MANAGER.lock().unwrap();
    
    if let Some(session_id) = manager.get_active_session().cloned() {
        // 通知插件命令开始
        if let Some(session) = manager.sessions.get_mut(&session_id) {
            for plugin in &mut session.plugins {
                plugin.on_command_start(&command, &session_id);
            }
        }
        
        let command_with_newline = format!("{}\n", command);
        manager.write_to_session(&session_id, &command_with_newline)
    } else {
        Err("No active terminal session".to_string())
    }
}

#[tauri::command]
pub async fn resize_terminal(cols: u16, rows: u16) -> Result<(), String> {
    let mut manager = TERMINAL_MANAGER.lock().unwrap();
    
    if let Some(session_id) = manager.get_active_session().cloned() {
        manager.resize_session(&session_id, cols, rows)
    } else {
        Err("No active terminal session".to_string())
    }
}

#[tauri::command]
pub async fn close_terminal() -> Result<(), String> {
    let mut manager = TERMINAL_MANAGER.lock().unwrap();
    
    if let Some(session_id) = manager.get_active_session().cloned() {
        manager.close_session(&session_id)
    } else {
        Err("No active terminal session".to_string())
    }
}

#[tauri::command]
pub async fn send_input(input: String) -> Result<(), String> {
    let mut manager = TERMINAL_MANAGER.lock().unwrap();
    
    if let Some(session_id) = manager.get_active_session().cloned() {
        manager.write_to_session(&session_id, &input)
    } else {
        Err("No active terminal session".to_string())
    }
}

fn get_default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}

// 新增的 Tauri 命令
#[tauri::command]
pub async fn get_terminal_info() -> Result<serde_json::Value, String> {
    let manager = TERMINAL_MANAGER.lock().unwrap();
    let info = serde_json::json!({
        "active_session": manager.get_active_session(),
        "total_sessions": manager.sessions.len(),
        "default_shell": get_default_shell(),
    });
    Ok(info)
}

#[tauri::command]
pub async fn list_plugins() -> Result<Vec<String>, String> {
    let manager = TERMINAL_MANAGER.lock().unwrap();
    if let Some(session_id) = manager.get_active_session() {
        if let Some(session) = manager.sessions.get(session_id) {
            let plugin_names: Vec<String> = session.plugins.iter()
                .map(|p| p.name().to_string())
                .collect();
            Ok(plugin_names)
        } else {
            Err("No active session".to_string())
        }
    } else {
        Err("No active session".to_string())
    }
}