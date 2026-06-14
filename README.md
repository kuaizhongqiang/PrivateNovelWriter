# PrivateNovelWriter (PNW)

> 私人小说写作助手 — 专为中长篇小说批量生产设计。
> 当前版本: **0.2.0-beta** | 架构: **Server Mode + REST API + OpenClaw 插件**

---

## 概述

PrivateNovelWriter 是一个**本地优先**的小说写作工具，核心设计围绕 AI Agent 协作展开。

### 核心架构

```
Agent A (你的 AI 助手)
    │ 通过 Plugin 工具集 或 CLI 调用
    ▼
PNW Server (HTTP 服务)
    │ 编排 Agent B + 数据层
    ▼
Agent B (写作专家 AI)    ↔    DB (SQLite) + 正文 (.txt)
```

**三层分工：**

| 层 | 角色 | 说明 |
| :--- | :--- | :--- |
| **Agent A** | 编排者 | 用户侧 AI 助手（Claude/OpenClaw），负责理解用户需求、按工作流编排工具调用 |
| **PNW Server** | 执行引擎 | HTTP 服务，暴露 REST API，管理项目数据、调用 Agent B |
| **Agent B** | 写作专家 | 内置 LLM 编排层，读大纲/角色/样例 → 调用 LLM 生成正文 → 写入文件 |

### 数据模型

```
小说                            ← Novel
 ├── 设定 (Setting)             ← 世界观、类型、标签
 ├── 角色 (Character)           ← 人物表（男主/女主/其他）
 ├── 大纲卷 → 大纲章            ← OutlinePhase → OutlineChapter
 │     └── text_chapter_id      ← 关联已写正文（空=未写）
 ├── 正文卷 → 正文章 (.txt)     ← TextPhase → TextChapter
 ├── 文风样例 (DetailSample)    ← 风格参考
 └── 金手指 (Plugin)            ← 系统/超能力设定
```

### 关键设计决策

- **Server Mode 前置** — 非常驻 CLI 进程，而是 HTTP 服务。插件化和 LLM cache 优化都依赖它
- **编排归 Agent A** — PNW 提供工具集 + Skill 知识文档，不固定写作流程
- **Prefix Cache 优化** — DeepSeek 50 倍成本差，prompt 不变部分在前、变化部分在后

详见 [docs/beta-roadmap.md](docs/beta-roadmap.md)。

---

## 快速开始

### 前置条件

- Rust 工具链（`cargo build`）
- 或直接下载 Release 二进制

### 编译

```bash
cargo build --release -p pnw
```

### 配置 LLM

复制 `.env.example` 为 `.env`，配置 API Key：

```bash
LLM_PROVIDER=deepseek
LLM_API_KEY=sk-your-key-here
LLM_MODEL=deepseek-v4-flash
```

支持切换 LM Studio 本地模型（修改 `LLM_PROVIDER=lmstudio` 即可）。

### 创建项目

```bash
# 创建一个新小说项目
pnw novel new 我的小说

# 进入项目目录
cd 我的小说
```

### 启动 Server（核心入口）

```bash
pnw server --host 127.0.0.1 --port 3000
```

Server 启动后：

- **REST API**: `http://127.0.0.1:3000/api/...`
- **Gateway UI**: 浏览器打开 `http://127.0.0.1:3000` 查看项目仪表盘

---

## 集成 OpenClaw

### 方式一：Tool Plugin

将 PNW 注册为 OpenClaw 的 tool plugin。Agent A 通过以下工具与 PNW 交互：

| 工具 | 方法 | 路径 |
| :--- | :--- | :--- |
| 获取项目信息 | GET | `/api/project` |
| 获取统计 | GET | `/api/stats` |
| 获取大纲树 | GET | `/api/outline` |
| 列出章节 | GET | `/api/chapters` |
| 读章节 | GET | `/api/chapter/{id}` |
| 保存章节 | PUT | `/api/chapter/{id}` |
| 列出角色 | GET | `/api/characters` |
| 创建角色 | POST | `/api/characters` |
| 获取设定 | GET | `/api/setting` |
| 更新设定 | POST | `/api/setting` |
| 列出样例 | GET | `/api/samples` |
| 通用命令 | POST | `/api/command` |
| 写正文（AI） | POST | `/api/agent/write` |
| 修改正文（AI） | POST | `/api/agent/revise` |
| 评估正文（AI） | POST | `/api/agent/evaluate/{id}` |
| 导出全文 | POST | `/api/export/txt` |

### 方式二：CLI 直接调

```bash
pnw <command> [args]
```

所有命令支持 JSON 输出，每章在 `status` 返回摘要而非全文。

### Agent A 入门指引

1. 先读 `cli/src/skill.md` 了解 PNW 的数据模型和写作工作流
2. 启动 PNW Server，获取项目信息
3. 按推荐工作流编排：设定 → 大纲 → 正文 → 评估 → 修改
4. 每步调用 REST API，LLM 生成的任务走 `/api/agent/*` 端点
5. 参考下文「示例工作流」

---

## 示例工作流（Agent A 编排）

### 场景：一句话灵感 → 前三章

```text
用户：写一个校园修仙故事，主角是转校生
```

Agent A 的工作流：

```
1. 创建项目              POST /api/project (新建)
2. 写入设定              POST /api/setting
3. 创建角色              POST /api/characters
4. 创建大纲卷            POST /api/command {"command":"create_outline_phase","args":{"name":"第一卷"}}
5. 创建大纲章 x3         POST /api/command {"command":"create_outline_chapter","args":{"...":""}}
6. 等待用户确认大纲      ← 建议暂停，让用户过目
7. 创建正文卷            CLI: pnw text phase create
8. 逐章写正文            POST /api/agent/write (返回摘要)
9. 评估已写章节          POST /api/agent/evaluate/{id}
10. 根据反馈修改          POST /api/agent/revise
11. 导出全文              POST /api/export/txt
```

---

## API 参考

所有接口返回统一信封：

```json
{
  "status": "ok" | "error",
  "data": { ... },
  "error": "错误描述（仅 error 时）"
}
```

### 通用命令 (POST /api/command)

请求体：

```json
{
  "command": "get_outline",
  "args": { "name": "新卷" }
}
```

支持的命令：

| command | args | 说明 |
| :--- | :--- | :--- |
| `get_outline` | — | 大纲树 |
| `get_novel` | — | 小说信息 |
| `list_characters` | — | 角色列表 |
| `get_setting` | — | 设定 |
| `list_samples` | — | 文风样例 |
| `get_plugin` | — | 金手指 |
| `list_outline_phases` | — | 大纲卷列表 |
| `list_outline_chapters` | `{ phase_id }` | 章列表 |
| `create_outline_phase` | `{ name }` | 创建卷 |
| `create_outline_chapter` | `{ phase_id, name }` | 创建章 |
| `create_character` | `{ name, char_type?, age?, relationship? }` | 创建角色 |
| `write_setting` | `{ title?, inspiration?, description?, novel_type?, tags? }` | 更新设定 |

### Agent 写作 (POST)

**写正文：** `/api/agent/write`

```json
{ "chapter_id": "uuid", "brief": "主角开始调查反派" }
```

**修改正文：** `/api/agent/revise`

```json
{ "chapter_id": "uuid", "feedback": "打斗场景需要更激烈" }
```

**评估：** `POST /api/agent/evaluate/{chapter_id}`

---

## 环境变量

| 变量 | 默认值 | 说明 |
| :--- | :--- | :--- |
| `LLM_PROVIDER` | `deepseek` | LLM 提供商：`deepseek` / `lmstudio` |
| `LLM_API_KEY` | — | API Key |
| `LLM_MODEL` | `deepseek-v4-flash` | 模型名 |
| `LMSTUDIO_BASE_URL` | `http://localhost:1234/v1` | 本地 LLM 地址 |
| `PNW_PROJECT` | — | 项目路径（可选） |

---

## 项目状态

| Phase | 内容 | 状态 |
| :--- | :--- | :--- |
| P1 | 数据模型 + SQLite + .txt 存储 | ✅ 完成 |
| P2 | CLI 完整命令集 | ✅ 完成 |
| P3 | Agent B 写作专家集成 | ✅ 完成 |
| P4 | Desktop 桌面端 (Tauri + Svelte) | ✅ 完成 |
| P5 | Server Mode + REST API | ✅ 完成 |
| **Beta** | OpenClaw 插件 + Prefix Cache 优化 | **当前阶段** |
| Future | 容器化 + Gateway UI 增强 | 规划中 |

---

## 文档索引

| 文档 | 用途 |
| :--- | :--- |
| [docs/beta-roadmap.md](docs/beta-roadmap.md) | 技术路线与方向性决策 |
| [docs/beta-checklist.md](docs/beta-checklist.md) | 边界问题销项清单 |
| [docs/prompt-architecture.md](docs/prompt-architecture.md) | Prompt 模板设计（prefix cache） |
| [docs/tools.md](docs/tools.md) | Agent B 工具定义 |
| [docs/design.md](docs/design.md) | 架构设计说明 |
| [cli/src/skill.md](cli/src/skill.md) | **面向 Agent A 的知识手册（必读）** |

---

## 开发

```bash
cargo build                    # 构建所有
cargo build -p pnw             # 仅构建 CLI + Server
cargo build -p pnw-desktop     # 构建桌面端
cargo test -p pnw-kernel       # 运行核心测试
cargo test -p pnw --test e2e   # 运行端到端测试
```

Desktop 需要 MSVC 工具链（当前环境使用 GNU 工具链，桌面端构建受限）。

---

## 许可证

Apache 2.0
