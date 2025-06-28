<template>
  <main class="container">
    <div class="header">
        <span class="header-item" @click="getInfo">获取信息</span>
    </div>
    <div class="terminal-container" ref="terminalRef" style="width: 100%; height: 100%; background: black;"></div>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { message } from '@tauri-apps/plugin-dialog';

const terminalRef = ref<HTMLDivElement | null>(null);
let term: Terminal;
let fitAddon: FitAddon;
let unlisten: () => void;
let sessionId: string | null = null;

const getInfo = async () => {
  const info = await invoke("get_terminal_info");
  console.log('info:',info);
  await message(JSON.stringify(info), { title: 'Tauri', kind: 'error' });
}

onMounted(async () => {
  if (!terminalRef.value) {
    console.error("Terminal container not found");
    return;
  }

  // 初始化终端
  term = new Terminal({
    cursorBlink: true,
    fontFamily: `'JetBrainsMono Nerd Font', 'Noto Sans Mono CJK SC', monospace`,
    fontSize: 14,
    lineHeight: 1.2,
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      cursor: '#ffffff',
      black: '#000000',
      red: '#cd3131',
      green: '#0dbc79',
      yellow: '#e5e510',
      blue: '#2472c8',
      magenta: '#bc3fbc',
      cyan: '#11a8cd',
      white: '#e5e5e5',
      brightBlack: '#666666',
      brightRed: '#f14c4c',
      brightGreen: '#23d18b',
      brightYellow: '#f5f543',
      brightBlue: '#3b8eea',
      brightMagenta: '#d670d6',
      brightCyan: '#29b8db',
      brightWhite: '#e5e5e5'
    },
    allowTransparency: true,
    scrollback: 10000,
    tabStopWidth: 4,
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  term.open(terminalRef.value);
  fitAddon.fit();
  term.focus();

  // 创建终端会话
  try {
    sessionId = await invoke<string>("create_shell");
    console.log("Terminal session created:", sessionId);
  } catch (error) {
    console.error("Failed to create terminal session:", error);
    term.write("\r\n❌ Failed to create terminal session\r\n");
    return;
  }

  // 监听后端流式输出事件
  unlisten = await listen<string>("terminal-output", event => {
    term.write(event.payload);
  });

  // 处理所有输入（包括键盘输入和粘贴）
  term.onData(async (data) => {
    try {
      await invoke("send_input", { input: data });
    } catch (error) {
      console.error("Failed to send input:", error);
    }
  });

  // 窗口大小变化时自动调整终端大小
  const handleResize = async () => {
    fitAddon.fit();
    const dimensions = fitAddon.proposeDimensions();
    if (dimensions && sessionId) {
      try {
        await invoke("resize_terminal", { 
          cols: dimensions.cols, 
          rows: dimensions.rows 
        });
      } catch (error) {
        console.error("Failed to resize terminal:", error);
      }
    }
  };

  window.addEventListener("resize", handleResize);

  // 初始化大小
  setTimeout(handleResize, 100);
});

onBeforeUnmount(async () => {
  if (unlisten) unlisten();
  if (term) term.dispose();
  
  // 清理终端会话
  if (sessionId) {
    try {
      await invoke("close_terminal");
    } catch (error) {
      console.error("Failed to close terminal session:", error);
    }
  }
});
</script>

<style scoped>
.container {
  margin: 0; 
  padding: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}
.terminal-container{
    padding: 4px;
}
.header{    
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
}
.header-item:hover{
    color: #007bff;
}       
.header-item{
    cursor: pointer;
}
</style>