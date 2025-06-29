<template>
  <main class="container">
    <!-- AI聊天图标 - 当聊天面板隐藏时显示 -->
    <div v-if="!showChatPanel" class="ai-chat-icon" @click="showChatPanel = true">
      <div class="icon-container">
        <span class="ai-icon">🤖</span>
        <span class="tooltip">AI 助手</span>
      </div>
    </div>

    <div class="main-content">
      <!-- 终端容器 -->
      <div class="terminal-section" :style="{ width: `calc(100% - ${chatPanelWidth}px)` }">
        <div class="terminal-container" ref="terminalRef"></div>
      </div>
      
      <!-- 聊天面板 -->
      <div v-if="showChatPanel" class="chat-section" :style="{ width: `${chatPanelWidth}px` }">
        <div class="chat-panel-wrapper">
          <!-- 设置面板 - 在标题栏上方 -->
          <div v-if="showConfig" class="config-panel">
            <div class="config-header">
              <span>AI 配置</span>
              <button @click="showConfig = false" class="close-btn">×</button>
            </div>
            <div class="config-content">
              <div class="config-item">
                <label>API Key:</label>
                <input 
                  v-model="aiConfig.api_key" 
                  type="password" 
                  placeholder="输入你的DeepSeek API Key"
                />
              </div>
              <div class="config-item">
                <label>模型:</label>
                <select v-model="aiConfig.model">
                  <option value="deepseek-chat">DeepSeek-V3-0324</option>
                  <option value="deepseek-reasoner">DeepSeek-R1-0528</option>
                </select>
              </div>
              <div class="config-item">
                <label>最大Token:</label>
                <input v-model.number="aiConfig.max_tokens" type="number" min="100" max="4000" />
              </div>
              <div class="config-item">
                <label>温度:</label>
                <input v-model.number="aiConfig.temperature" type="number" min="0" max="2" step="0.1" />
              </div>
              <button @click="saveAIConfig" class="save-btn">保存配置</button>
            </div>
          </div>

          <!-- 标题栏 -->
          <div class="chat-panel-header">
            <span>AI 助手</span>
            <div class="chat-controls">
              <button @click="showConfig = !showConfig" class="config-btn" title="配置">
                ⚙️
              </button>
              <button class="minimize-btn" @click="showChatPanel = false" title="最小化">
                <span>−</span>
              </button>
            </div>
          </div>

          <!-- 拖拽手柄，pointer-events根据isResizing动态切换 -->
          <div 
            class="resize-handle" 
            :style="{ pointerEvents: isResizing ? 'auto' : 'none' }"
            @mousedown="startResize"
          ></div>
          <ChatPanel ref="chatPanelRef" />
        </div>
      </div>
    </div>
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
import ChatPanel from './components/ChatPanel.vue';

const terminalRef = ref<HTMLDivElement | null>(null);
const showChatPanel = ref(true);
const chatPanelWidth = ref(350);
let term: Terminal;
let fitAddon: FitAddon;
let unlisten: () => void;
let sessionId: string | null = null;
let isResizing = false;
let showConfig = ref(false);
let aiConfig = ref({
  api_key: '',
  model: 'deepseek-chat',
  max_tokens: 1000,
  temperature: 0.7
});

const getInfo = async () => {
  const info = await invoke("get_terminal_info");
  console.log('info:',info);
  await message(JSON.stringify(info), { title: 'Tauri', kind: 'error' });
}

const startResize = (e: MouseEvent) => {
  e.preventDefault();
  isResizing = true;
  
  const startX = e.clientX;
  const startWidth = chatPanelWidth.value;
  
  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing) return;
    
    const deltaX = startX - e.clientX;
    const newWidth = Math.max(250, Math.min(600, startWidth + deltaX));
    chatPanelWidth.value = newWidth;
  };
  
  const handleMouseUp = () => {
    isResizing = false;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
  };
  
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
};

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
  unlisten = await listen<string>("terminal-output", (event: any) => {
    term.write(event.payload);
  });

  // 处理所有输入（包括键盘输入和粘贴）
  term.onData(async (data: any) => {
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

const saveAIConfig = async () => {
  try {
    await invoke('configure_ai', { config: aiConfig.value });
    showConfig.value = false;
    console.log('AI配置已保存');
  } catch (error) {
    console.error('保存AI配置失败:', error);
  }
};
</script>

<style scoped>
.container {
  margin: 0; 
  padding: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  position: relative;
}

.main-content {
  display: flex;
  height: 100vh;
  width: 100%;
}

.terminal-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.terminal-container {
  flex: 1;
  padding: 4px;
  background: black;
  height: 100%;
  user-select: text;
}

/* 美化终端滚动条 */
.terminal-container ::-webkit-scrollbar {
  width: 8px;
}

.terminal-container ::-webkit-scrollbar-track {
  background: #1a1a1a;
  border-radius: 4px;
}

.terminal-container ::-webkit-scrollbar-thumb {
  background: #444;
  border-radius: 4px;
  transition: background 0.3s ease;
}

.terminal-container ::-webkit-scrollbar-thumb:hover {
  background: #666;
}

.terminal-container ::-webkit-scrollbar-corner {
  background: #1a1a1a;
}

.chat-section {
  position: relative;
  background: #1a1a1a;
  border-left: 1px solid #333;
}

.chat-panel-wrapper {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.chat-panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  background: #2a2a2a;
  border-bottom: 1px solid #333;
  color: #fff;
  font-size: 14px;
  font-weight: 500;
}

.chat-controls {
  display: flex;
  gap: 5px;
}

.minimize-btn {
  background: none;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 16px;
  line-height: 1;
}

.minimize-btn:hover {
  background: #444;
  color: #fff;
}

.resize-handle {
  position: absolute;
  left: -3px;
  top: 0;
  bottom: 0;
  width: 6px;
  cursor: col-resize;
  background: transparent;
  z-index: 10;
  user-select: none;
  pointer-events: auto;
}

.resize-handle:hover {
  background: rgba(0, 123, 255, 0.3);
}

.resize-handle::before {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 2px;
  height: 20px;
  background: #007bff;
  border-radius: 1px;
}

/* AI聊天图标样式 */
.ai-chat-icon {
  position: fixed;
  top: 40px;
  right: 20px;
  z-index: 1000;
}

.icon-container {
  position: relative;
  width: 50px;
  height: 50px;
  background: #007bff;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(0, 123, 255, 0.3);
  transition: all 0.3s ease;
}

.icon-container:hover {
  transform: scale(1.1);
  box-shadow: 0 6px 16px rgba(0, 123, 255, 0.4);
}

.ai-icon {
  font-size: 24px;
  color: white;
}

.tooltip {
  position: absolute;
  top: -30px;
  left: 50%;
  transform: translateX(-50%);
  background: #333;
  color: white;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.3s ease;
}

.icon-container:hover .tooltip {
  opacity: 1;
}

.tooltip::after {
  content: '';
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 4px solid transparent;
  border-top-color: #333;
}

/* 防止拖拽时选中文本 */
.container {
  user-select: none;
}

.chat-section {
  user-select: text;
}

.config-panel {
  background: #2a2a2a;
  border-bottom: 1px solid #333;
  z-index: 100;
}

.config-header {
  background: #333;
  padding: 10px 15px;
  border-bottom: 1px solid #444;
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: #fff;
  font-size: 14px;
  font-weight: 500;
}

.close-btn {
  background: none;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 16px;
  line-height: 1;
}

.close-btn:hover {
  background: #444;
  color: #fff;
}

.config-content {
  padding: 15px;
}

.config-item {
  margin-bottom: 10px;
}

.config-item label {
  display: block;
  color: #ccc;
  margin-bottom: 5px;
  font-size: 12px;
}

.config-item input,
.config-item select {
  width: 100%;
  padding: 8px;
  border: 1px solid #555;
  border-radius: 4px;
  background: #1a1a1a;
  color: #fff;
  font-size: 12px;
}

.save-btn {
  width: 100%;
  background: #007bff;
  border: none;
  color: white;
  cursor: pointer;
  padding: 8px;
  border-radius: 4px;
  margin-top: 10px;
}

.save-btn:hover {
  background: #0056b3;
}

.config-btn {
  background: none;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 16px;
  line-height: 1;
  transition: all 0.2s ease;
}

.config-btn:hover {
  background: #444;
  color: #fff;
}
</style> 