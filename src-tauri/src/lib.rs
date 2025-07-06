// src/main.rs
mod commands;
mod ai;

use commands::{
    create_shell, 
    run_command_pty, 
    resize_terminal, 
    close_terminal, 
    send_input,
    get_terminal_info,
    list_plugins,
};

use ai::{
    stream_fetch,
    get_ai_tools,
    execute_ai_tool,
    create_ai_agent
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            create_shell,
            run_command_pty,
            resize_terminal,
            close_terminal,
            send_input,
            get_terminal_info,
            list_plugins,
            // AI相关命令
            stream_fetch,
            get_ai_tools,
            execute_ai_tool,
            create_ai_agent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}