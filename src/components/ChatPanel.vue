<template>
  <div class="min-h-screen bg-gradient-to-br from-blue-600 via-purple-600 to-indigo-800 p-4">
    <div class="max-w-6xl mx-auto">
      <!-- 配置面板 -->
      <div class="glass-effect rounded-2xl p-6 mb-6 shadow-xl">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-xl font-bold text-white flex items-center gap-2">
            🤖 AI 终端助手配置
          </h2>
          <button
            @click="toggleConfig"
            class="px-4 py-2 bg-white/10 hover:bg-white/20 text-white rounded-lg transition-all duration-200"
          >
            {{ showConfig ? '收起' : '展开' }}
          </button>
        </div>
        
        <div v-show="showConfig" class="space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="block text-white text-sm font-medium mb-2">API Key</label>
              <input
                v-model="config.api_key"
                type="password"
                placeholder="输入您的 API Key"
                class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400"
              />
            </div>
            <div>
              <label class="block text-white text-sm font-medium mb-2">模型</label>
              <select
                v-model="config.model"
                class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-400"
              >
                <option value="deepseek-chat">deepseek-chat</option>
                <option value="gpt-3.5-turbo">gpt-3.5-turbo</option>
                <option value="gpt-4">gpt-4</option>
              </select>
            </div>
            <div>
              <label class="block text-white text-sm font-medium mb-2">Base URL</label>
              <input
                v-model="config.base_url"
                type="text"
                class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400"
              />
            </div>
            <div>
              <label class="block text-white text-sm font-medium mb-2">
                Temperature: {{ config.temperature }}
              </label>
              <input
                v-model="config.temperature"
                type="range"
                min="0"
                max="1"
                step="0.1"
                class="w-full"
              />
            </div>
          </div>
          
          <div class="flex items-center gap-4">
            <label class="flex items-center gap-2 text-white">
              <input
                v-model="config.stream"
                type="checkbox"
                class="rounded"
              />
              启用流式输出
            </label>
            <label class="flex items-center gap-2 text-white">
              <input
                v-model="config.mcp_enabled"
                type="checkbox"
                class="rounded"
              />
              启用 MCP 工具
            </label>
          </div>
          
          <div class="flex gap-2 flex-wrap">
            <button
              @click="saveConfig"
              :disabled="loading"
              class="px-4 py-2 bg-green-500 hover:bg-green-600 text-white rounded-lg transition-colors disabled:opacity-50"
            >
              {{ loading ? '保存中...' : '保存配置' }}
            </button>
            <button
              @click="loadConfig"
              class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors"
            >
              加载配置
            </button>
            <button
              @click="testMcpTools"
              class="px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-white rounded-lg transition-colors"
            >
              测试 MCP 工具
            </button>
            <button
              @click="refreshTools"
              class="px-4 py-2 bg-purple-500 hover:bg-purple-600 text-white rounded-lg transition-colors"
            >
              刷新工具
            </button>
          </div>
          
          <!-- 显示可用工具 -->
          <div v-if="tools.length > 0" class="mt-4">
            <h3 class="text-white font-medium mb-2">可用的 MCP 工具:</h3>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="tool in tools"
                :key="tool.name"
                class="px-3 py-1 bg-white/10 text-white rounded-lg text-sm"
                :title="tool.description"
              >
                {{ tool.name }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- 聊天界面 -->
      <div class="glass-effect rounded-2xl p-6 shadow-xl">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-lg font-semibold text-white">💬 AI 对话</h3>
          <div class="flex gap-2">
            <button
              @click="createNewAgent"
              class="px-3 py-1 bg-green-500/20 hover:bg-green-500/30 text-green-200 rounded-lg transition-colors text-sm"
            >
              新建代理
            </button>
            <button
              @click="clearChat"
              class="px-3 py-1 bg-red-500/20 hover:bg-red-500/30 text-red-200 rounded-lg transition-colors text-sm"
            >
              清空对话
            </button>
          </div>
        </div>
        
        <!-- 消息列表 -->
        <div
          ref="messagesContainer"
          class="h-96 overflow-y-auto scrollbar-hide space-y-4 mb-4 p-4 bg-black/10 rounded-lg"
        >
          <div
            v-for="message in messages"
            :key="message.id"
            class="chat-message"
            :class="message.role === 'user' ? 'text-right' : 'text-left'"
          >
            <div
              class="inline-block max-w-xs lg:max-w-md px-4 py-2 rounded-lg"
              :class="message.role === 'user' 
                ? 'bg-blue-500 text-white' 
                : message.role === 'system'
                  ? 'bg-gray-500 text-white'
                  : 'bg-white/20 text-white'"
            >
              <div class="text-xs opacity-70 mb-1">
                {{ message.role === 'user' ? '👤 用户' : 
                   message.role === 'system' ? '⚙️ 系统' : '🤖 AI助手' }}
              </div>
              <div 
                class="message-content"
                v-html="formatMessage(message.content)"
              ></div>
              <div class="text-xs opacity-70 mt-1">
                {{ formatTime(message.timestamp) }}
              </div>
            </div>
          </div>
          
          <!-- 加载指示器 -->
          <div v-if="isStreaming" class="text-left">
            <div class="inline-block max-w-xs lg:max-w-md px-4 py-2 rounded-lg bg-white/20 text-white">
              <div class="text-xs opacity-70 mb-1">🤖 AI助手</div>
              <div class="typing-indicator">正在思考中...</div>
            </div>
          </div>
        </div>
        
        <!-- 输入区域 -->
        <div class="flex gap-2">
          <input
            v-model="userInput"
            @keyup.enter="sendMessage"
            :disabled="isStreaming"
            type="text"
            placeholder="输入您的消息..."
            class="flex-1 px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 disabled:opacity-50"
          />
          <button
            @click="sendMessage"
            :disabled="isStreaming || !userInput.trim()"
            class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {{ isStreaming ? '发送中...' : '发送' }}
          </button>
        </div>
        
        <!-- 快捷命令 -->
        <div class="mt-4">
          <div class="flex flex-wrap gap-2 mb-2">
            <button
              v-for="cmd in quickCommands"
              :key="cmd"
              @click="useQuickCommand(cmd)"
              :disabled="isStreaming"
              class="px-3 py-1 bg-white/10 hover:bg-white/20 text-white rounded-lg transition-colors text-sm disabled:opacity-50"
            >
              {{ cmd }}
            </button>
          </div>
          
          <!-- 文件操作快捷按钮 -->
          <div class="flex flex-wrap gap-2">
            <button
              v-for="fileCmd in fileCommands"
              :key="fileCmd.label"
              @click="useFileCommand(fileCmd)"
              :disabled="isStreaming"
              class="px-3 py-1 bg-indigo-500/20 hover:bg-indigo-500/30 text-indigo-200 rounded-lg transition-colors text-sm disabled:opacity-50"
            >
              {{ fileCmd.label }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event'
import { useAiClient } from '../composables/useAiClient'

// 使用 AI 客户端
const {
  tools,
  loading,
  error,
  getAvailableTools,
  executeTool,
  streamFetch,
  executeCommand,
  readFile,
  writeFile,
  deleteFile,
  listDirectory,
  getCurrentDirectory,
  createAgent
} = useAiClient()

// 响应式数据
const showConfig = ref(false)
const isStreaming = ref(false)
const userInput = ref('')
const messages = ref([])
const messagesContainer = ref(null)
const currentStreamMessage = ref(null)
const currentAgentId = ref(null)

// 配置数据
const config = ref({
  api_key: 'sk-307b526d430b4f498d66d967697987a6',
  model: 'deepseek-chat',
  base_url: 'https://api.deepseek.com',
  max_tokens: 1000,
  temperature: 0.7,
  stream: true,
  mcp_enabled: true
})

// 快捷命令
const quickCommands = ref([
  '列出当前目录文件',
  '显示当前目录',
  '查看系统信息',
  '帮我创建一个新文件',
  '执行 ls -la',
  '查看 package.json'
])

// 文件操作命令
const fileCommands = ref([
  { label: '📁 当前目录', action: 'getCurrentDirectory' },
  { label: '📄 列出文件', action: 'listDirectory' },
  { label: '📝 读取文件', action: 'readFile' },
  { label: '✏️ 写入文件', action: 'writeFile' },
  { label: '🗑️ 删除文件', action: 'deleteFile' },
  { label: '⚡ 执行命令', action: 'executeCommand' }
])

// 生成消息ID
const generateMessageId = () => {
  return Date.now().toString() + Math.random().toString(36).substr(2, 9)
}

// 格式化时间
const formatTime = (timestamp) => {
  const date = new Date(timestamp)
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

// 格式化消息内容（支持 Markdown）
const formatMessage = (content) => {
  if (!content) return ''
  
  // 简单的 Markdown 解析
  let formatted = content
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/```([\s\S]*?)```/g, '<pre><code>$1</code></pre>')
    .replace(/\n/g, '<br>')
  
  // 处理工具调用标记
  formatted = formatted.replace(/🔧 Calling tool: (.+)/g, '<div class="tool-call">🔧 调用工具: $1</div>')
  
  return formatted
}

// 滚动到底部
const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

// 切换配置面板
const toggleConfig = () => {
  showConfig.value = !showConfig.value
}

// 保存配置
const saveConfig = async () => {
  try {
    await invoke('configure_ai', { config: config.value })
    console.log('配置保存成功')
    addSystemMessage('✅ AI 配置已保存')
  } catch (error) {
    console.error('保存配置失败:', error)
    addSystemMessage('❌ 配置保存失败: ' + error)
  }
}

// 加载配置
const loadConfig = async () => {
  try {
    const savedConfig = await invoke('get_ai_config')
    if (savedConfig) {
      config.value = { ...config.value, ...savedConfig }
      addSystemMessage('✅ 配置加载成功')
    } else {
      addSystemMessage('ℹ️ 未找到已保存的配置')
    }
  } catch (error) {
    console.error('加载配置失败:', error)
    addSystemMessage('❌ 配置加载失败: ' + error)
  }
}

// 测试 MCP 工具
const testMcpTools = async () => {
  try {
    await getAvailableTools()
    if (tools.value.length > 0) {
      addSystemMessage('🔧 找到 ' + tools.value.length + ' 个可用的 MCP 工具')
    } else {
      addSystemMessage('⚠️ 未找到可用的 MCP 工具')
    }
  } catch (error) {
    console.error('测试 MCP 工具失败:', error)
    addSystemMessage('❌ 测试 MCP 工具失败: ' + error)
  }
}

// 刷新工具列表
const refreshTools = async () => {
  try {
    await getAvailableTools()
    addSystemMessage('🔄 工具列表已刷新')
  } catch (error) {
    console.error('刷新工具失败:', error)
    addSystemMessage('❌ 刷新工具失败: ' + error)
  }
}

// 创建新代理
const createNewAgent = async () => {
  try {
    currentAgentId.value = await createAgent()
    addSystemMessage('🤖 新的 AI 代理已创建: ' + currentAgentId.value)
  } catch (error) {
    console.error('创建代理失败:', error)
    addSystemMessage('❌ 创建代理失败: ' + error)
  }
}

// 添加系统消息
const addSystemMessage = (content) => {
  messages.value.push({
    id: generateMessageId(),
    role: 'system',
    content: content,
    timestamp: Date.now()
  })
  scrollToBottom()
}

// 发送消息
const sendMessage = async () => {
  if (!userInput.value.trim() || isStreaming.value) return
  
  const message = userInput.value.trim()
  userInput.value = ''
  
  // 添加用户消息
  messages.value.push({
    id: generateMessageId(),
    role: 'user',
    content: message,
    timestamp: Date.now()
  })
  
  // 准备 AI 回复消息
  currentStreamMessage.value = {
    id: generateMessageId(),
    role: 'assistant',
    content: '',
    timestamp: Date.now()
  }
  
  messages.value.push(currentStreamMessage.value)
  scrollToBottom()
  
  try {
    isStreaming.value = true
    
    if (config.value.stream) {
      // 使用流式 HTTP 请求
      const requestBody = new TextEncoder().encode(JSON.stringify({
        model: config.value.model,
        messages: [{ role: 'user', content: message }],
        temperature: config.value.temperature,
        max_tokens: config.value.max_tokens,
        stream: true
      }))
      
      await streamFetch(
        'POST',
        config.value.base_url + '/chat/completions',
        {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer ' + config.value.api_key
        },
        requestBody,
        {
          onChunk: (chunk) => {
            if (currentStreamMessage.value) {
              currentStreamMessage.value.content += chunk
              scrollToBottom()
            }
          },
          onEnd: (status) => {
            if (status === 200) {
              addSystemMessage('✅ 消息发送成功')
            } else {
              addSystemMessage('❌ 消息发送失败，状态码: ' + status)
            }
          },
          onError: (error) => {
            addSystemMessage('❌ 流式请求失败: ' + error)
          }
        }
      )
    } else {
      // 非流式输出
      const response = await invoke('chat_with_ai', { message })
      currentStreamMessage.value.content = response
    }
  } catch (error) {
    console.error('发送消息失败:', error)
    addSystemMessage('❌ 消息发送失败: ' + error)
  } finally {
    isStreaming.value = false
    currentStreamMessage.value = null
    scrollToBottom()
  }
}

// 使用快捷命令
const useQuickCommand = (command) => {
  userInput.value = command
  sendMessage()
}

// 使用文件命令
const useFileCommand = async (fileCmd) => {
  try {
    let result
    const timestamp = Date.now()
    
    switch (fileCmd.action) {
      case 'getCurrentDirectory':
        result = await getCurrentDirectory()
        break
      case 'listDirectory':
        const currentDir = await getCurrentDirectory()
        if (currentDir.is_error) {
          addSystemMessage('❌ 获取当前目录失败: ' + currentDir.content)
          return
        }
        result = await listDirectory(currentDir.content.trim())
        break
      case 'readFile':
        const filePath = prompt('请输入要读取的文件路径:')
        if (!filePath) return
        result = await readFile(filePath)
        break
      case 'writeFile':
        const writeFilePath = prompt('请输入要写入的文件路径:')
        if (!writeFilePath) return
        const content = prompt('请输入文件内容:')
        if (content === null) return
        result = await writeFile(writeFilePath, content, true)
        break
      case 'deleteFile':
        const deleteFilePath = prompt('请输入要删除的文件路径:')
        if (!deleteFilePath) return
        if (!confirm('确定要删除文件 ' + deleteFilePath + ' 吗？')) return
        result = await deleteFile(deleteFilePath)
        break
      case 'executeCommand':
        const command = prompt('请输入要执行的命令:')
        if (!command) return
        result = await executeCommand(command)
        break
      default:
        addSystemMessage('❌ 未知的文件操作: ' + fileCmd.action)
        return
    }
    
    // 显示结果
    messages.value.push({
      id: generateMessageId(),
      role: 'assistant',
      content: result.is_error 
        ? '❌ 操作失败: ' + result.content 
        : '✅ 操作成功:\n```\n' + result.content + '\n```',
      timestamp: timestamp
    })
    
    scrollToBottom()
  } catch (error) {
    console.error('文件操作失败:', error)
    addSystemMessage('❌ 文件操作失败: ' + error)
  }
}

// 清空对话
const clearChat = () => {
  messages.value = []
  addSystemMessage('🗑️ 对话已清空')
}

// 监听流式输出
const setupStreamListener = async () => {
  try {
    await listen('ai-stream-chunk', (event) => {
      if (currentStreamMessage.value) {
        console.log('流式输出:', event.payload)
        currentStreamMessage.value.content += event.payload
        scrollToBottom()
      }
    })
  } catch (error) {
    console.error('设置流式监听失败:', error)
  }
}

// 组件挂载时的初始化
onMounted(async () => {
  await setupStreamListener()
  await loadConfig()
  await getAvailableTools()
  addSystemMessage('🎉 AI 终端助手已启动！请先配置您的 API Key。')
})
</script>

<style scoped>
.glass-effect {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.chat-message {
  animation: fadeInUp 0.3s ease-out;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.typing-indicator {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

.message-content :deep(pre) {
  background: rgba(0, 0, 0, 0.2);
  padding: 12px;
  border-radius: 8px;
  overflow-x: auto;
  font-family: 'Courier New', monospace;
  font-size: 14px;
  line-height: 1.4;
  margin: 8px 0;
}

.message-content :deep(code) {
  background: rgba(0, 0, 0, 0.2);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 14px;
}

.message-content :deep(.tool-call) {
  background: rgba(255, 165, 0, 0.2);
  border-left: 4px solid #ffa500;
  padding: 8px 12px;
  margin: 8px 0;
  border-radius: 4px;
  font-family: monospace;
  font-size: 14px;
}

.message-content :deep(strong) {
  font-weight: 600;
}

.message-content :deep(em) {
  font-style: italic;
}
</style>