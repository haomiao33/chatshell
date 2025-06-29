<template>
  <div class="chat-panel">
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
import { ref, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

const messages = ref<ChatMessage[]>([]);
const inputMessage = ref('');
const isLoading = ref(false);
const messagesRef = ref<HTMLDivElement>();

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
  
  console.log('[FRONTEND] Sending message:', userMessage);
  
  messages.value.push({
    role: 'user',
    content: userMessage,
    timestamp: new Date(),
  });

  await scrollToBottom();
  isLoading.value = true;

  try {
    console.log('[FRONTEND] Calling chat_with_ai command...');
    const response = await invoke<string>('chat_with_ai', { message: userMessage });
    console.log('[FRONTEND] Received response:', response);
    
    messages.value.push({
      role: 'assistant',
      content: response,
      timestamp: new Date(),
    });
  } catch (error) {
    console.error('[FRONTEND] AI chat error:', error);
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

// 暴露方法给父组件
const clearMessages = () => {
  messages.value = [];
};

defineExpose({
  clearMessages
});
</script>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1a1a1a;
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