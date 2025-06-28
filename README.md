# 🧠 ChatShell - AI 原生终端工具（Tauri + Vue + Xterm.js + Rust）

> **ChatShell** 是新一代高性能终端工具，专为构建大模型（LLM）驱动的 Shell 环境而设计。前端基于 Tauri + Vue3 + xterm.js，后端核心采用 Rust 构建，当前已支持 macOS 与 Windows，未来将扩展至 Linux 及更多平台。

---

## ✨ 项目简介

ChatShell 聚焦于构建**面向 AI 的原生终端工具链**，目标是打造：
- 📟 类 Unix 的 Shell 使用体验
- 🧠 具备 LLM 接入能力的命令层
- 🦀 高性能 Rust 后端指令执行控制
- 💻 可扩展 UI 和插件式 Shell 模拟器

---

## ✅ 已实现功能

- ✅ 跨平台支持（macOS、Windows）
- ✅ 终端核心：基于 xterm.js 的交互式终端
- ✅ 执行系统命令（默认使用 zsh / cmd）
- ✅ Tauri 构建快速、原生体验
- ✅ 后端全部使用 Rust 编写，具备高性能异步处理能力
- ✅ 中文字体、右键行为、窗口菜单可控

---

## 🚧 规划中功能

- [ ] AI 指令转译（支持 GPT、Claude 等大模型）
- [ ] 自定义指令 DSL
- [ ] LLM 命令建议、补全
- [ ] 多窗口 / 多标签页
- [ ] 内置 WASI Shell 解释器（非系统依赖）
- [ ] Linux 平台支持

---

## 🧪 快速开始

### 安装依赖

```bash
# 前端依赖
npm install

# 后端 Rust 编译
cargo build
