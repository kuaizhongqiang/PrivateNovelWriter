# PrivateNovelWriter

私人小说写作助手 —— 专为中长篇小说批量生产设计。

## 概述

PrivateNovelWriter 是一个本地优先的小说写作工具，核心设计围绕 **AI Agent 协作** 展开：

| 组件 | 说明 |
|---|---|
| **Agent A** | 用户的 AI 助手（Claude 等），通过 CLI 与系统交互 |
| **Agent B** | 内置的写作专家 AI，负责创作编排 |
| **CLI** | Agent A 的操作接口，任务级命令 |
| **Desktop** | Windows 桌面应用（Tauri + Svelte） |

## 架构

三层工具设计：

```
Agent A (任务级)  →  write_chapter, revise, plan_outline, get_status, manage_entity
    ↓ CLI
Agent B (编排级)  →  将创作任务拆解为数据操作序列
    ↓
数据层 (执行级)   →  SQLite + .txt 纯文本存储
```

详见 [docs/design.md](docs/design.md) 和 [docs/tools.md](docs/tools.md)。

## 技术栈

- **核心**: Rust
- **CLI**: clap v4
- **桌面**: Tauri v2 + Svelte 5
- **存储**: SQLite + 纯文本 .txt 文件

## 开发状态

| Phase | 内容 | 状态 |
|---|---|---|
| P1 | 数据模型 + SQLite + .txt 存储 | 未开始 |
| P2 | CLI 完整命令集 | 未开始 |
| P3 | Agent B 写作专家集成 | 未开始 |
| P4 | Desktop 桌面端 | 未开始 |
| P5 | 高级特性（批量、导出） | 未开始 |

## 许可证

Apache 2.0
