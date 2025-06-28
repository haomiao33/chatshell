// src/main.rs
mod commands;
use tauri_plugin_dialog::DialogExt;

use commands::{
    create_shell, 
    run_command_pty, 
    resize_terminal, 
    close_terminal, 
    send_input,
    get_terminal_info,
    list_plugins
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
            list_plugins
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}