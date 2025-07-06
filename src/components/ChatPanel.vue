<template>
  <div class="chat-panel">
    <!-- 聊天消息区域 -->
    <div class="chat-messages" ref="messagesContainer">
      <div 
        v-for="(message, index) in messages" 
        :key="index"
        class="message"
        :class="{ 'user-message': message.role === 'user', 'ai-message': message.role === 'assistant' }"
      >
        <div class="message-header">
          <span class="role-icon">{{ message.role === 'user' ? '👤' : '🤖' }}</span>
          <span class="role-name">{{ message.role === 'user' ? 'You' : 'AI' }}</span>
          <span class="timestamp">{{ formatTime(message.timestamp) }}</span>
        </div>
        <div class="message-content">
          <div v-if="message.role === 'user'" class="user-content">
            {{ message.content }}
          </div>
          <div v-else class="ai-content">
            <!-- 支持 Markdown 渲染 -->
            <div v-html="renderMarkdown(message.content)" class="markdown-content"></div>
          </div>
        </div>
      </div>
      
      <!-- 正在输入指示器 -->
      <div v-if="isTyping" class="message ai-message typing-indicator">
        <div class="message-header">
          <span class="role-icon">🤖</span>
          <span class="role-name">AI</span>
          <span class="timestamp">正在输入...</span>
        </div>
        <div class="message-content">
          <div class="typing-dots">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>
      </div>
    </div>

    <!-- 输入区域 -->
    <div class="chat-input-area">
      <div class="input-container">
       <div class="chat-input-wrapper">
  <textarea
    v-model="inputMessage"
    @keydown="handleKeydown"
    @input="autoResize"
    ref="inputRef"
    class="chat-input"
    placeholder="输入消息... (Ctrl+Enter 发送)"
    rows="4"
    :disabled="isTyping"
  ></textarea>
</div>
        <div class="input-actions">
          <div class="left-actions">
            <button 
            @click="clearChat" 
            class="action-btn clear-btn"
            :disabled="messages.length === 0"
            title="清空聊天"
          >
           <Trash size="18" />
          </button>
          </div>
          <div class="right-actions">
            <button 
            @click="sendMessage" 
            class="action-btn send-btn"
            :disabled="!inputMessage.trim() || isTyping"
            title="发送消息"
          >
             <component :is="isTyping ? LoaderCircle : Send" size="18" :class="{ 'spin': isTyping }" />
          </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { marked } from 'marked';
import { markedHighlight } from 'marked-highlight';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import { Trash, Send, LoaderCircle } from 'lucide-vue-next';
import { listen } from '@tauri-apps/api/event';

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

// 响应式数据
const messages = ref<Message[]>([]);
const inputMessage = ref('');
const isTyping = ref(false);
const messagesContainer = ref<HTMLElement>();
const inputRef = ref<HTMLTextAreaElement>();
const currentAIMessage = ref('');


// 配置 marked 支持代码高亮
marked.use(markedHighlight({
  langPrefix: 'hljs language-',
  highlight(code, lang) {
    const language = hljs.getLanguage(lang) ? lang : 'plaintext';
    return hljs.highlight(code, { language }).value;
  }
}));

// 配置 marked 选项
marked.setOptions({
  breaks: true,
  gfm: true,
});

// 渲染 Markdown
const renderMarkdown = (content: string): string => {
  console.log(content)
  return marked.parse(content);
};

// 格式化时间
const formatTime = (timestamp: number): string => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('zh-CN', { 
    hour: '2-digit', 
    minute: '2-digit' 
  });
};

// 自动调整输入框高度
const autoResize = () => {
  if (inputRef.value) {
    inputRef.value.style.height = 'auto';
    inputRef.value.style.height = `${Math.min(inputRef.value.scrollHeight, 120)}px`;
  }
};

// 滚动到底部
const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }
  });
};

// 键盘事件处理
const handleKeydown = (event: KeyboardEvent) => {
    // event.preventDefault();
  // 处于输入法拼写状态，不发送
//   if (event.ctrlKey && event.key === 'Enter') {
//     event.preventDefault();
//     sendMessage();
//   } else if (event.key === 'Enter' && !event.shiftKey) {
//     event.preventDefault();
//     sendMessage();
//   }
};

// // 发送消息
// const sendMessage = async () => {
//   if (!inputMessage.value.trim() || isTyping.value) return;

//   const userMessage: Message = {
//     role: 'user',
//     content: inputMessage.value.trim(),
//     timestamp: Date.now()
//   };

//   messages.value.push(userMessage);
//   const messageToSend = inputMessage.value.trim();
//   inputMessage.value = '';
//   autoResize();
//   scrollToBottom();

//   isTyping.value = true;

//   try {
//     // 调用后端 API 发送消息
//     // const response = await invoke<string>('chat_with_ai', {
//     //   message: messageToSend,
//     //   history: messages.value.slice(-10) // 只发送最近 10 条消息作为上下文
//     // });
//     await invoke('chat_with_ai_stream', {
//       message: messageToSend
//     });

//     // const aiMessage: Message = {
//     //   role: 'assistant',
//     //   content: response,
//     //   timestamp: Date.now()
//     // };

//     // messages.value.push(aiMessage);
//     scrollToBottom();

//   } catch (error) {
//     console.error('发送消息失败:', error);
//     const errorMessage: Message = {
//       role: 'assistant',
//       content: `❌ 发送消息失败: ${error}`,
//       timestamp: Date.now()
//     };
//     messages.value.push(errorMessage);
//     scrollToBottom();
//   } finally {
//     isTyping.value = false;
//   }
// };


// 发送消息
const sendMessage = async () => {
  const content = inputMessage.value.trim();
  if (!content || isTyping.value) return;

  const userMessage: Message = {
    role: 'user',
    content,
    timestamp: Date.now()
  };

  messages.value.push(userMessage);
  inputMessage.value = '';
  currentAIMessage.value = '';
  autoResize();
  scrollToBottom();

  isTyping.value = true;

  try {
    await invoke('chat_with_ai_stream', {
      message: content
    });
  } catch (error) {
    console.error('发送消息失败:', error);
    messages.value.push({
      role: 'assistant',
      content: `❌ 发送失败：${error}`,
      timestamp: Date.now()
    });
    isTyping.value = false;
  }
};

// 监听流式响应片段
listen<string>('ai-stream-chunk', (event) => {
  console.log('--- ai-stream-chunk', event);
  if (!isTyping.value) {
    currentAIMessage.value = '';
    messages.value.push({
      role: 'assistant',
      content: '',
      timestamp: Date.now()
    });
  }

  currentAIMessage.value += event.payload;

  const lastMessage = messages.value[messages.value.length - 1];
  if (lastMessage?.role === 'assistant') {
    lastMessage.content = currentAIMessage.value;
  }

  scrollToBottom();
});

// ✅ 监听流式结束
listen('ai-stream-end', () => {
  console.log('[ai-stream-end] AI 响应结束');
  isTyping.value = false;
});



// 清空聊天
const clearChat = () => {
  messages.value = [];
};

// 监听消息变化，自动滚动到底部
watch(messages, () => {
  scrollToBottom();
}, { deep: true });



// 组件挂载后聚焦输入框
onMounted(() => {
  if (inputRef.value) {
    inputRef.value.focus();
  }
  
  // 添加欢迎消息
  messages.value.push({
    role: 'assistant',
    content: '👋 你好！我是 AI 助手，有什么可以帮助你的吗？\n\n💡 **使用提示：**\n- 支持 Markdown 语法\n- 支持代码高亮\n- 使用 Ctrl+Enter 或 Enter 发送消息\n- 可以通过右上角配置按钮设置 AI 模型',
    timestamp: Date.now()
  });
});

watch(inputMessage, () => {
  nextTick(() => {
    inputRef.value?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  });
});
</script>

<style scoped>
.chat-panel {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #1a1a1a;
  color: #e0e0e0;
  overflow: hidden; /* 防止内部溢出 */
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  scroll-behavior: smooth;
  min-height: 0; /* 💥关键，防止撑出父容器 */
}


.message {
  margin-bottom: 20px;
  animation: fadeIn 0.3s ease-in;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}

.message-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 12px;
  color: #888;
}

.role-icon {
  font-size: 14px;
}

.role-name {
  font-weight: 600;
  color: #ccc;
}

.timestamp {
  margin-left: auto;
  font-size: 11px;
  color: #666;
}

.message-content {
  padding: 12px 16px;
  border-radius: 12px;
  max-width: 100%;
  word-wrap: break-word;
}

.user-message .message-content {
  background: #0d7377;
  color: white;
  margin-left: 20px;
  border-bottom-right-radius: 4px;
}

.ai-message .message-content {
  background: #2a2a2a;
  border: 1px solid #333;
  margin-right: 20px;
  border-bottom-left-radius: 4px;
}

.user-content {
  white-space: pre-wrap;
  line-height: 1.5;
}

.ai-content {
  line-height: 1.6;
}



/* Markdown 样式 */
 .markdown-content {
  color: #e0e0e0;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  color: #fff;
  margin: 16px 0 8px 0;
  font-weight: 600;
}

.markdown-content :deep(p) {
  margin: 8px 0;
  line-height: 1.6;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 8px 0;
  padding-left: 20px;
}

.markdown-content :deep(li) {
  margin: 4px 0;
}

.markdown-content :deep(blockquote) {
  border-left: 4px solid #0d7377;
  padding-left: 16px;
  margin: 16px 0;
  color: #ccc;
  font-style: italic;
  background: rgba(13, 115, 119, 0.1);
  border-radius: 4px;
  padding: 12px 16px;
}

.markdown-content :deep(code) {
  background: #333;
  color: #e0e0e0;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'JetBrainsMono Nerd Font', 'Fira Code', monospace;
  font-size: 0.9em;
}

.markdown-content :deep(pre) {
  background: #1e1e1e;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
  margin: 16px 0;
  position: relative;
}

.markdown-content :deep(pre code) {
  background: none;
  padding: 0;
  border-radius: 0;
  font-size: 0.9em;
  line-height: 1.4;
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 16px 0;
  background: #2a2a2a;
  border-radius: 8px;
  overflow: hidden;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: 12px;
  text-align: left;
  border-bottom: 1px solid #333;
}

.markdown-content :deep(th) {
  background: #333;
  font-weight: 600;
  color: #fff;
}

.markdown-content :deep(a) {
  color: #4fc3f7;
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 8px;
  margin: 8px 0;
} 


.chat-input-area {
  flex-shrink: 0;
  border-top: 1px solid #333;
  padding: 16px;
  background: #1a1a1a;
}
.input-container {
  display: flex;
  gap: 8px;
  align-items: flex-end;

  display: flex;
  flex-direction: column;
}
.chat-input-wrapper {
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid #444;
  background: #2a2a2a;
   width: 100%;
}

.chat-input {
  border: none;
  border-radius: 0;
  background: transparent;
  color: #e0e0e0;
  resize: none;
  font-size: 14px;
  padding: 12px 14px;
  width: 100%;
  max-height: 120px;
  overflow-y: auto;
  box-sizing: border-box;
  font-family: inherit;
  line-height: 1.6;
 
}

.chat-input:focus {
  outline: none;
  border-color: #0d7377;
}

.chat-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.chat-input::-webkit-scrollbar {
  width: 6px;
}

.chat-input::-webkit-scrollbar-thumb {
  background: #444;
  border-radius: 3px;
}


/* actions */
.input-actions {
  display: flex;
  width: 100%;
  gap: 4px;
  align-items: center;
  margin-bottom: 10px;
  justify-content: space-between;
}

.action-btn {
  background: #2a2a2a;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 8px 10px;
  color: #e0e0e0;
  cursor: pointer;
  transition: all 0.2s ease;
  font-size: 14px;
}

.action-btn:hover:not(:disabled) {
  background: #333;
  border-color: #555;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.send-btn:hover:not(:disabled) {
  background: #0d7377;
  border-color: #0d7377;
}

.clear-btn:hover:not(:disabled) {
  background: #d32f2f;
  border-color: #d32f2f;
}

/* 正在输入指示器 */
.typing-indicator .message-content {
  padding: 16px;
}

.typing-dots {
  display: flex;
  gap: 4px;
  align-items: center;
}

.typing-dots span {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #666;
  animation: typing 1.4s infinite ease-in-out;
}

.typing-dots span:nth-child(1) {
  animation-delay: 0s;
}

.typing-dots span:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dots span:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typing {
  0%, 80%, 100% {
    transform: scale(0.8);
    opacity: 0.5;
  }
  40% {
    transform: scale(1);
    opacity: 1;
  }
}

/* 滚动条样式 */
.chat-messages::-webkit-scrollbar {
  width: 6px;
}

.chat-messages::-webkit-scrollbar-track {
  background: #1a1a1a;
}

.chat-messages::-webkit-scrollbar-thumb {
  background: #444;
  border-radius: 3px;
}

.chat-messages::-webkit-scrollbar-thumb:hover {
  background: #555;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .chat-messages {
    padding: 12px;
  }
  
  .message-content {
    padding: 10px 12px;
  }
  
  .user-message .message-content,
  .ai-message .message-content {
    margin-left: 0;
    margin-right: 0;
  }
  
  .chat-input-area {
    padding: 12px;
  }
}
</style>