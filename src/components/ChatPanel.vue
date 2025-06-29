<template>
  <div class="chat-panel">
    <div class="chat-header">
      <h3>AI 助手</h3>
      <button @click="showConfig = !showConfig" class="config-btn">
        ⚙️
      </button>
    </div>

    <!-- AI配置面板 -->
    <div v-if="showConfig" class="config-panel">
      <div class="config-item">
        <label>API Key:</label>
        <input 
          v-model="config.api_key" 
          type="password" 
          placeholder="输入你的OpenAI API Key"
        />
      </div>
      <div class="config-item">
        <label>模型:</label>
        <select v-model="config.model">
          <option value="deepseek-chat">DeepSeek-V3-0324</option>
          <option value="deepseek-reasoner">DeepSeek-R1-0528</option>
        </select>
      </div>
      <div class="config-item">
        <label>最大Token:</label>
        <input v-model.number="config.max_tokens" type="number" min="100" max="4000" />
      </div>
      <div class="config-item">
        <label>温度:</label>
        <input v-model.number="config.temperature" type="number" min="0" max="2" step="0.1" />
      </div>
      <button @click="saveConfig" class="save-btn">保存配置</button>
    </div>

    <!-- 聊天消息列表 -->
    <div class="chat-messages" ref="messagesRef">
      <div 
        v-for="(message, index) in messages" 
        :key="index" 
        :class="['message', message.role]"
      >
        <div class="message-header">
          <span class="role">{{ message.role === 'user' ? '👤 你' : '🤖 AI' }}</span>
          <span class="time">{{ formatTime(message.timestamp) }}</span>
        </div>
        <div class="message-content">{{ message.content }}</div>
      </div>
    </div>

    <!-- 输入框 -->
    <div class="chat-input">
      <textarea
        v-model="inputMessage"
        @keydown.ctrl.enter="sendMessage"
        placeholder="输入你的问题... (Ctrl+Enter 发送)"
        rows="3"
      ></textarea>
      <button @click="sendMessage" :disabled="isLoading || !inputMessage.trim()">
        {{ isLoading ? '发送中...' : '发送' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface AIConfig {
  api_key: string;
  model: string;
  base_url: string;
  max_tokens: number;
  temperature: number;
}

const messages = ref<ChatMessage[]>([]);
const inputMessage = ref('');
const isLoading = ref(false);
const showConfig = ref(false);
const messagesRef = ref<HTMLDivElement>();

const config = ref<AIConfig>({
  api_key: '',
  model: 'deepseek-chat',
  base_url: 'https://api.deepseek.com',
  max_tokens: 1000,
  temperature: 0.7,
});

const formatTime = (date: Date) => {
  return date.toLocaleTimeString();
};

const scrollToBottom = async () => {
  await nextTick();
  if (messagesRef.value) {
    messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
  }
};

const sendMessage = async () => {
  if (!inputMessage.value.trim() || isLoading.value) return;

  const userMessage = inputMessage.value.trim();
  inputMessage.value = '';
  
  messages.value.push({
    role: 'user',
    content: userMessage,
    timestamp: new Date(),
  });

  await scrollToBottom();
  isLoading.value = true;

  try {
    const response = await invoke<string>('chat_with_ai', { message: userMessage });
    
    messages.value.push({
      role: 'assistant',
      content: response,
      timestamp: new Date(),
    });
  } catch (error) {
    console.error('AI chat error:', error);
    messages.value.push({
      role: 'assistant',
      content: `❌ 错误: ${error}`,
      timestamp: new Date(),
    });
  } finally {
    isLoading.value = false;
    await scrollToBottom();
  }
};

const saveConfig = async () => {
  try {
    await invoke('configure_ai', { config: config.value });
    showConfig.value = false;
    messages.value.push({
      role: 'assistant',
      content: '✅ AI配置已保存！',
      timestamp: new Date(),
    });
  } catch (error) {
    console.error('Save config error:', error);
    messages.value.push({
      role: 'assistant',
      content: `❌ 配置保存失败: ${error}`,
      timestamp: new Date(),
    });
  }
};

const loadConfig = async () => {
  try {
    const savedConfig = await invoke<AIConfig | null>('get_ai_config');
    if (savedConfig) {
      config.value = savedConfig;
    }
  } catch (error) {
    console.error('Load config error:', error);
  }
};

onMounted(() => {
  loadConfig();
});
</script>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1a1a1a;
  border-left: 1px solid #333;
  width: 350px;
}

.chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px;
  border-bottom: 1px solid #333;
  background: #2a2a2a;
}

.chat-header h3 {
  margin: 0;
  color: #fff;
  font-size: 16px;
}

.config-btn {
  background: none;
  border: none;
  color: #fff;
  cursor: pointer;
  font-size: 18px;
  padding: 5px;
  border-radius: 4px;
}

.config-btn:hover {
  background: #444;
}

.config-panel {
  padding: 15px;
  border-bottom: 1px solid #333;
  background: #2a2a2a;
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
  padding: 8px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  margin-top: 10px;
}

.save-btn:hover {
  background: #0056b3;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 15px;
}

.message {
  margin-bottom: 15px;
  padding: 10px;
  border-radius: 8px;
  max-width: 100%;
}

.message.user {
  background: #2b5797;
  margin-left: 20px;
}

.message.assistant {
  background: #333;
  margin-right: 20px;
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 5px;
  font-size: 11px;
  color: #ccc;
}

.message-content {
  color: #fff;
  line-height: 1.4;
  font-size: 13px;
  white-space: pre-wrap;
}

.chat-input {
  padding: 15px;
  border-top: 1px solid #333;
  background: #2a2a2a;
}

.chat-input textarea {
  width: 100%;
  padding: 10px;
  border: 1px solid #555;
  border-radius: 4px;
  background: #1a1a1a;
  color: #fff;
  font-family: inherit;
  font-size: 13px;
  resize: vertical;
  min-height: 60px;
}

.chat-input textarea:focus {
  outline: none;
  border-color: #007bff;
}

.chat-input button {
  width: 100%;
  padding: 10px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  margin-top: 10px;
  font-size: 14px;
}

.chat-input button:hover:not(:disabled) {
  background: #0056b3;
}

.chat-input button:disabled {
  background: #555;
  cursor: not-allowed;
}
</style> 