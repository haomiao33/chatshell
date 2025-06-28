// src/plugins/mod.rs - 插件模块示例

use crate::commands::TerminalPlugin;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;

// 命令计时插件
pub struct TimingPlugin {
    command_start_times: HashMap<String, u64>,
}

impl TimingPlugin {
    pub fn new() -> Self {
        Self {
            command_start_times: HashMap::new(),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl TerminalPlugin for TimingPlugin {
    fn name(&self) -> &str {
        "timing"
    }

    fn on_command_start(&mut self, command: &str, session_id: &str) {
        let timestamp = Self::current_timestamp();
        self.command_start_times.insert(session_id.to_string(), timestamp);
        println!("🐶 [{}] Command started: {}", session_id, command);
    }

    fn on_command_end(&mut self, exit_code: Option<i32>, session_id: &str) {
        if let Some(start_time) = self.command_start_times.remove(session_id) {
            let duration = Self::current_timestamp() - start_time;
            let status = match exit_code {
                Some(0) => "✅ Success",
                Some(code) => &format!("❌ Failed ({})", code),
                None => "⚠️ Unknown",
            };
            println!("🐶 [{}] Command finished: {} ({}ms)", session_id, status, duration);
        }
    }
}

// Git 状态插件
pub struct GitPlugin {
    git_regex: Regex,
}

impl GitPlugin {
    pub fn new() -> Self {
        Self {
            git_regex: Regex::new(r"git\s+").unwrap(),
        }
    }

    fn is_git_repo() -> bool {
        std::path::Path::new(".git").exists()
    }

    fn get_git_branch() -> Option<String> {
        std::process::Command::new("git")
            .args(&["branch", "--show-current"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                } else {
                    None
                }
            })
    }
}

impl TerminalPlugin for GitPlugin {
    fn name(&self) -> &str {
        "git"
    }

    fn on_command_start(&mut self, command: &str, session_id: &str) {
        if self.git_regex.is_match(command) && Self::is_git_repo() {
            if let Some(branch) = Self::get_git_branch() {
                println!("🐶 [{}] Git command in branch: {}", session_id, branch);
            }
        }
    }

    fn on_output(&mut self, output: &str, _session_id: &str) -> String {
        // 可以在这里添加 Git 输出的颜色高亮
        output.to_string()
    }
}

// 自动补全插件
pub struct AutoCompletePlugin {
    suggestions: Vec<String>,
}

impl AutoCompletePlugin {
    pub fn new() -> Self {
        Self {
            suggestions: vec![
                "ls".to_string(),
                "cd".to_string(),
                "pwd".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "git status".to_string(),
                "git add".to_string(),
                "git commit".to_string(),
                "git push".to_string(),
                "git pull".to_string(),
                "npm install".to_string(),
                "npm run".to_string(),
                "cargo build".to_string(),
                "cargo run".to_string(),
                "cargo test".to_string(),
            ],
        }
    }

    pub fn get_suggestions(&self, input: &str) -> Vec<String> {
        self.suggestions
            .iter()
            .filter(|suggestion| suggestion.starts_with(input))
            .cloned()
            .collect()
    }
}

impl TerminalPlugin for AutoCompletePlugin {
    fn name(&self) -> &str {
        "autocomplete"
    }
}

// 别名插件
pub struct AliasPlugin {
    aliases: HashMap<String, String>,
}

impl AliasPlugin {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        
        // 添加一些常用别名
        aliases.insert("ll".to_string(), "ls -la".to_string());
        aliases.insert("la".to_string(), "ls -A".to_string());
        aliases.insert("l".to_string(), "ls -CF".to_string());
        aliases.insert("..".to_string(), "cd ..".to_string());
        aliases.insert("...".to_string(), "cd ../..".to_string());
        aliases.insert("gs".to_string(), "git status".to_string());
        aliases.insert("ga".to_string(), "git add".to_string());
        aliases.insert("gc".to_string(), "git commit".to_string());
        aliases.insert("gp".to_string(), "git push".to_string());
        aliases.insert("gl".to_string(), "git pull".to_string());
        
        Self { aliases }
    }

    pub fn add_alias(&mut self, alias: &str, command: &str) {
        self.aliases.insert(alias.to_string(), command.to_string());
    }

    pub fn expand_alias(&self, command: &str) -> String {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if let Some(first_word) = parts.first() {
            if let Some(expanded) = self.aliases.get(*first_word) {
                if parts.len() > 1 {
                    format!("{} {}", expanded, parts[1..].join(" "))
                } else {
                    expanded.clone()
                }
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        }
    }
}

impl TerminalPlugin for AliasPlugin {
    fn name(&self) -> &str {
        "alias"
    }

    fn on_command_start(&mut self, command: &str, session_id: &str) {
        let expanded = self.expand_alias(command);
        if expanded != command {
            println!("🐶 [{}] Alias expanded: {} -> {}", session_id, command, expanded);
        }
    }
}

// 主题插件
pub struct ThemePlugin {
    current_theme: String,
    themes: HashMap<String, ThemeConfig>,
}

#[derive(Clone)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
}

impl ThemePlugin {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        
        // 默认主题
        themes.insert("default".to_string(), ThemeConfig {
            background: "#1e1e1e".to_string(),
            foreground: "#d4d4d4".to_string(),
            cursor: "#ffffff".to_string(),
            selection: "#264f78".to_string(),
        });
        
        // 深色主题
        themes.insert("dark".to_string(), ThemeConfig {
            background: "#000000".to_string(),
            foreground: "#ffffff".to_string(),
            cursor: "#ffffff".to_string(),
            selection: "#404040".to_string(),
        });
        
        // 浅色主题
        themes.insert("light".to_string(), ThemeConfig {
            background: "#ffffff".to_string(),
            foreground: "#000000".to_string(),
            cursor: "#000000".to_string(),
            selection: "#b3d7ff".to_string(),
        });
        
        // Monokai 主题
        themes.insert("monokai".to_string(), ThemeConfig {
            background: "#272822".to_string(),
            foreground: "#f8f8f2".to_string(),
            cursor: "#f8f8f0".to_string(),
            selection: "#49483e".to_string(),
        });
        
        // Dracula 主题
        themes.insert("dracula".to_string(), ThemeConfig {
            background: "#282a36".to_string(),
            foreground: "#f8f8f2".to_string(),
            cursor: "#f8f8f0".to_string(),
            selection: "#44475a".to_string(),
        });

        Self {
            current_theme: "default".to_string(),
            themes,
        }
    }

    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        if self.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name))
        }
    }

    pub fn get_current_theme(&self) -> Option<&ThemeConfig> {
        self.themes.get(&self.current_theme)
    }

    pub fn list_themes(&self) -> Vec<&String> {
        self.themes.keys().collect()
    }
}

impl TerminalPlugin for ThemePlugin {
    fn name(&self) -> &str {
        "theme"
    }

    fn on_command_start(&mut self, command: &str, session_id: &str) {
        if command.starts_with("doge theme ") {
            let theme_name = command.trim_start_matches("doge theme ").trim();
            match self.set_theme(theme_name) {
                Ok(()) => println!("🐶 [{}] Theme changed to: {}", session_id, theme_name),
                Err(e) => println!("🐶 [{}] Theme error: {}", session_id, e),
            }
        } else if command == "doge theme list" {
            let themes = self.list_themes();
            println!("🐶 [{}] Available themes: {:?}", session_id, themes);
        }
    }
}

// 系统监控插件
pub struct MonitorPlugin {
    show_system_info: bool,
}

impl MonitorPlugin {
    pub fn new() -> Self {
        Self {
            show_system_info: false,
        }
    }

    fn get_system_info() -> String {
        let mut info = Vec::new();
        
        // CPU 使用率（简化版）
        if let Ok(loadavg) = std::fs::read_to_string("/proc/loadavg") {
            if let Some(load) = loadavg.split_whitespace().next() {
                info.push(format!("Load: {}", load));
            }
        }
        
        // 内存使用情况（简化版）
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let lines: Vec<&str> = meminfo.lines().collect();
            if lines.len() >= 2 {
                // 这里可以解析更详细的内存信息
                info.push("Mem: OK".to_string());
            }
        }
        
        if info.is_empty() {
            "System info not available".to_string()
        } else {
            info.join(" | ")
        }
    }
}

impl TerminalPlugin for MonitorPlugin {
    fn name(&self) -> &str {
        "monitor"
    }

    fn on_command_start(&mut self, command: &str, session_id: &str) {
        if command == "doge monitor on" {
            self.show_system_info = true;
            println!("🐶 [{}] System monitoring enabled", session_id);
        } else if command == "doge monitor off" {
            self.show_system_info = false;
            println!("🐶 [{}] System monitoring disabled", session_id);
        } else if command == "doge monitor status" {
            let info = Self::get_system_info();
            println!("🐶 [{}] System status: {}", session_id, info);
        }
    }

    fn on_session_start(&mut self, session_id: &str) {
        if self.show_system_info {
            let info = Self::get_system_info();
            println!("🐶 [{}] Welcome! System: {}", session_id, info);
        }
    }
}

// 插件管理器
pub struct PluginManager {
    plugins: Vec<Box<dyn TerminalPlugin + Send + Sync>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn register_default_plugins(&mut self) {
        self.plugins.push(Box::new(TimingPlugin::new()));
        self.plugins.push(Box::new(GitPlugin::new()));
        self.plugins.push(Box::new(AutoCompletePlugin::new()));
        self.plugins.push(Box::new(AliasPlugin::new()));
        self.plugins.push(Box::new(ThemePlugin::new()));
        self.plugins.push(Box::new(MonitorPlugin::new()));
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn TerminalPlugin + Send + Sync>) {
        self.plugins.push(plugin);
    }

    pub fn get_plugins(&self) -> &Vec<Box<dyn TerminalPlugin + Send + Sync>> {
        &self.plugins
    }

    pub fn get_plugins_mut(&mut self) -> &mut Vec<Box<dyn TerminalPlugin + Send + Sync>> {
        &mut self.plugins
    }

    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }
}

// 扩展的 Tauri 命令用于插件管理
#[tauri::command]
pub async fn list_plugins() -> Result<Vec<String>, String> {
    let manager = crate::commands::TERMINAL_MANAGER.lock().unwrap();
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

#[tauri::command]
pub async fn execute_plugin_command(command: String) -> Result<String, String> {
    // 这里可以实现插件特定的命令执行逻辑
    if command.starts_with("doge ") {
        Ok(format!("🐶 Plugin command executed: {}", command))
    } else {
        Err("Not a plugin command".to_string())
    }
}