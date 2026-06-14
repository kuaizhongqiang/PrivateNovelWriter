# Beta 路线图

> 版本: `0.2.0-beta`
> 基础: `0.1.0-alpha`（已完成 P1–P4 全部实现）
> 本文档随设计推进逐步细化。

---

## 架构决策：Server Mode 作为前置基础

**已定。** 插件化和上下文优化共同依赖一个常驻 Server，而不是 CLI 进程模型。

- 插件（OpenClaw / Claude）调 Server 的 REST API，而非 spawn CLI 进程
- 上下文优化依赖 Server 维持连接/Session，KV cache 才能命中
- 容器化和 Gateway UI 自然承接：Docker 跑 Server，Gateway 连 Server

### 为什么不是 CLI

每次 `child_process.spawn` 都是一次冷启动：

- LLM provider 侧看不到历史连接，prefix cache 无从发挥
- 无法维持 session，写→评估→改写这个链条每次都重建上下文
- Local LLM 场景尤其伤，每次都是 model reload

---

## 三大方向

### 一、插件化：PNW 作为 AI 助手的插件

将 PNW 的写作能力封装为可注册的工具集，交付给上层 AI 助手使用。

**目标平台：**

- OpenClaw — 注册为 tools 插件，通过 REST API 调用 Server
- Claude / MCP — 通过 Model Context Protocol 暴露写作能力

**核心思路：**

PNW 不内部搞插件系统，而是把自己做成"插件"供别人集成。插件层是薄客户端，所有业务逻辑在 Server 层。

### Plugin + Skill 配合模型

编排责任交给 Agent A，但 PNW 提供两部分：

| 组件 | 作用 | 使用方式 |
| :--- | :--- | :--- |
| **Plugin (工具集)** | PNW Server 暴露的 REST API，封装为 OpenClaw/MCP 可调用的工具 | Agent 在执行时调用 |
| **Skill (领域知识)** | PNW 的写作领域模型和推荐工作流，告诉 Agent A 怎样编排是最优的 | Agent 首涉写作时读取一次 |

**不把 Skill 塞进每轮请求**，而是作为静态知识保存，与上下文优化方向一致。

Skill 包含：

- PNW 数据模型结构（小说→卷→章→正文、角色、设定等）
- 推荐的写作工作流（设定 → 大纲 → 正文 → 评估 → 修改）
- 反模式提示（什么做法会降低结果质量）

**典型信息流：**

```text
用户（一句话灵感 → 前十章）→ Agent A
                                  ↓ 读 Skill（写作领域知识）
                                  ↓ 自行编排：plan → create → write ×10 → evaluate
                                  ↓ 逐个调用 PNW 工具（经 Server）
                                → 用户（摘要结果）
```

### 当前数据接口的缺口

现有 PNW 的数据操作面向内部 Agent B 设计，给 Agent A 编排使用时有以下不足：

- **读能力颗粒度不够** — 当前读写组合写死在代码里，Agent A 需要按需自由组合（如只读大纲+角色，不读样例）
- **缺少关联查询** — 无法直接问"哪些大纲章还没写正文""某卷完成度"这类跨实体问题
- **缺少校验与清理** — Agent A 写入的数据可能不干净，缺少 validate 类工具做后处理
- **缺少批量操作** — 创建一卷+十章的原子化操作需要调多次 API，应有 batch 端点

这些缺口在 Plugin 工具设计时补齐，Server 层提供更细粒度的接口即可。

### 二、上下文优化：最大化 DeepSeek Prefix Cache 命中率

**这是核心方向。** 目标是让 DeepSeek API 的硬盘缓存命中率达到最高——50 倍的成本差异，直接决定 PNW 在规模化使用时的经济性。

Local LLM 不受此约束（上下文有限、无按量计费），可随建随弃。

#### Remote API 的两大核心设计依据

Server 在调用 Remote API（DeepSeek）时，以下两个机制决定所有 prompt 构造和 session 管理策略：

**Prefix Cache（硬盘缓存）**

- 从 token 0 开始逐字节匹配，前缀一致即命中，命中后成本降至约 1/50
- 第一请求预热，后续同前缀请求命中
- 意味着：system prompt + 设定 + 角色表等不变部分必须在最前端且结构绝对稳定
- 任何动态内容（用户 brief、章名等）必须放在 prompt 末尾，不破坏前缀

**Reasoning + Tool Calling 交叠**

- LLM 可在推理过程中多次调用工具（读数据/查上下文/写文件），思考→调工具→继续思考→最终输出
- 但涉及工具调用后，`reasoning_content` 必须完整传回后续请求，否则 400 错误
- 意味着：Agent B 不需要固定编排流水线，LLM 自行决定何时读什么数据
- 与 prefix cache 存在张力——reasoning 内容每轮不同会 break cache，设计上需权衡

这两个机制共同决定了 Remote API 场景下 Server 的 prompt 架构和 session 生命周期策略。

核心不是减少 token 量，而是让 prompt 前缀稳定重复，驱动 LLM 的 KV cache 命中。

**Provider 差异：**

| Provider | 特性 | 策略 |
| :--- | :--- | :--- |
| **Local LLM** (LM Studio) | 上下文有限，cache 不跨请求持久化 | 可以更随意 new session，对话压缩后重建即可 |
| **Remote API** (DeepSeek) | 服务端 prefix cache，但重新输入完整 prompt 消耗大 | 需要更精细控制 session 生命周期，复用连接减少重复传输 |

**设计原则：**

- 将每次请求拆分为**不变部分**（system prompt、世界观设定、角色表、文风样例）和**变化部分**（章大纲、brief）
- 不变部分保持在 prompt 前端且结构固定，最大化 prefix cache 命中
- 避免无意义的 prompt 重排导致 cache miss
- 关注 provider 实际的 caching 行为，针对性优化

### 三、容器化 + 轻量 Gateway

让 PNW 能一键部署、浏览器访问。

**组成部分：**

- **Dockerfile + docker-compose** — 多阶段构建，`docker compose up` 启动（含 Server + Gateway）
- **Gateway UI** — 一个简洁的 Web 面板，类似 OpenClaw Control UI：
  - 项目状态概览
  - 章节列表与内容
  - 触发写作/评估命令
  - 不追求完整 Desktop 体验，能看见、能操作即可

---

## 节奏

1. 本文档只放方向性描述，不展开细节
2. 每个方向有进展时，逐节细化
3. 不赶工，设计充分再动手

---

## 当前状态 （2026-06）

Paper 阶段已完成。所有方向性决策已定，边界问题已收口（详见 [beta-checklist.md](beta-checklist.md)）。

**下一步进入设计阶段** — 基于本文档的框架，逐项拆解为 GitHub Issues 推进实现。

### 产出物索引

| 文档 | 用途 |
| :--- | :--- |
| [beta-roadmap.md](beta-roadmap.md) | 技术路线与方向性决策 |
| [beta-checklist.md](beta-checklist.md) | 边界问题与销项清单 |
