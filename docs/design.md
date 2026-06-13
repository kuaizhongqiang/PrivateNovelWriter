# PrivateNovelWriter 设计文档

## 技术栈

| 层 | 技术 |
|---|---|
| 核心 | Rust |
| CLI 端口 | `clap` v4 |
| 桌面端口 | Tauri v2 + Svelte 5 |
| 存储 | SQLite + .txt |
| 序列化 | serde + serde_json |

## 核心架构: 三层工具

系统分为三层，每层有自己面向的工具集：

```
                     ┌─────────────────────────────┐
                     │         Agent A              │
                     │    (用户的 Claude / 主 AI)    │
                     │  看到的是任务级工具:           │
                     │  • write_chapter(id, btw)    │
                     │  • revise_chapter(id, fb)    │
                     │  • plan_outline(brief)       │
                     │  • get_status                │
                     │  • manage_entity(...)        │
                     └──────────┬──────────────────┘
                                │ CLI 命令
                     ┌──────────▼──────────────────┐
                     │         CLI 端口              │
                     │  pnw chapter write --id 5   │
                     │  pnw outline plan --brief... │
                     │  pnw character add ...       │
                     │  pnw status                  │
                     └──────────┬──────────────────┘
                                │
              ┌─────────────────┼────────────────────┐
              │  ┌──────────────▼──────────────┐     │
              │  │    写作专家 Agent B          │     │
              │  │  (编排创作命令)               │     │
              │  │                              │     │
              │  │  Agent A 发来:               │     │
              │  │  write_chapter(id=5, ...)    │     │
              │  │  → Agent B 拆解:             │     │
              │  │    1. read outline_chapter   │     │
              │  │    2. read characters        │     │
              │  │    3. read samples           │     │
              │  │    4. call LLM → 生成正文    │     │
              │  │    5. write text_chapter     │     │
              │  │    6. 返回摘要给 Agent A     │     │
              │  └──────────────┬──────────────┘     │
              │                                   │
              │  ┌──────────────▼──────────────┐     │
              │  │  数据命令 (直接执行)          │     │
              │  │  read / write / delete /    │     │
              │  │  patch                      │     │
              │  │  (这层 CLI 和 Agent B 共用)  │     │
              │  └──────────────┬──────────────┘     │
              │                                   │
              │  ┌──────────────▼──────────────┐     │
              │  │  Data Model                  │     │
              │  │  SQLite + .txt               │     │
              │  └─────────────────────────────┘     │
              │                                      │
              │          Kernel                       │
              └──────────────────────────────────────┘
```

### 各层工具一览

| 层 | 工具 | 使用者 |
|---|---|---|
| **任务级** | `write_chapter` `revise_chapter` `plan_outline` `get_status` `manage_entity` `evaluate` | Agent A |
| **编排级** | `read` `write` `delete` `patch` `call_llm` | Agent B（内部） |
| **数据级** | SQLite CRUD + .txt I/O | 数据命令执行器 |

**关键规则：**
- Agent A **不直接调** read/write/delete/patch，也不读正文全文
- Agent A 发任务级命令，收到的是摘要（写了多少字、做了什么）
- Agent B 是编排者，把任务拆成数据命令序列
- CLI 命令 ≈ 任务级工具的一对一映射

## CLI 设计

CLI 是 Agent A 的操作界面，命令按写作任务组织，而非按数据操作组织。

```
pnw
├── chapter
│   ├── write <id> [--btw]    写正文 (→ Agent B)
│   ├── revise <id> [--feedback] 修改正文 (→ Agent B)
│   └── read <id>             读正文 (摘要, 非全文)
├── outline
│   ├── plan [--brief]        规划大纲 (→ Agent B)
│   ├── show                  显示大纲树
│   └── chapter
│       ├── add               添加章大纲
│       └── update <id>       修改章大纲
├── character
│   ├── add                   添加角色
│   ├── list                  列出角色
│   ├── update <id>           修改角色
│   └── delete <id>           删除角色
├── setting
│   ├── show                  查看设定
│   └── update                更新设定
├── novel
│   ├── new <name>            新建小说
│   ├── open <path>           打开项目
│   └── config                配置
├── status                    全局进度 (摘要)
├── evaluate <id>             写作评估 (→ Agent B)
└── agent <question>          向 Agent B 提问
```

> 命令输出统一为 JSON，方便 Agent A 解析。
> 简单操作（add/update/delete）直接执行，不经过 Agent B。
> 需要创作能力的操作（write/revise/plan/evaluate）由 Agent B 编排执行。

### CLI 命令执行流程

**直接命令（无 AI）：**
```
> pnw character add --name "张三" --type 0
    → Command::Data(WriteCharacter { name: "张三", char_type: 0 })
    → kernel 执行 SQLite INSERT
    → 返回 JSON: { "status": "ok", "data": { "id": "char-001", "name": "张三" } }
```

**创作命令（Agent B 编排）：**
```
> pnw chapter write --id 5 --btw "主角开始调查反派"
    → Command::Agent(WriteChapter { chapter_id: "5", brief: "主角开始调查反派" })
    → Agent B 编排:
        1. read outline_chapter id=oc-005    (读大纲)
        2. read character                     (读角色表)
        3. read sample                        (读文风样例)
        4. call_llm  → 生成正文
        5. write text_chapter id=tc-005       (写正文)
    → 返回 JSON: { "status": "ok", "data": { "chapter_id": "5", "word_count": 3120, "summary": "..." } }
```

## Agent B 写作专家设计

### Context 管理策略

Agent B 写正文时只读必要数据，不读前面的章节全文：

```
写正文时读取:
  √ 卷描述 (phase description)     → 定基调
  √ 当前章大纲 (content + hook)    → 知道写什么、怎么收尾
  √ 角色表                        → 名字、关系
  √ 文风样例                      → 保持统一风格
  × 不读前面章节的正文全文          → hook 已承担承接功能
```

这样每章的 context 消耗是固定的，写到 200 章也不会膨胀。

### LLM 抽象层

支持切换 API 和本地模型，通过 `LLM_PROVIDER` 环境变量控制：

| 提供商 | 类型 | 配置 |
|---|---|---|
| `deepseek` | API | `LLM_API_KEY` + `LLM_MODEL` |
| `lmstudio` | 本地 (OpenAI 兼容) | `LMSTUDIO_BASE_URL` (默认 `http://localhost:1234/v1`) |

```rust
#[async_trait]
pub trait LlmProvider {
    /// 发送对话请求，返回生成的文本
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse>;
}

pub struct DeepSeekProvider { ... }    // → OpenAI 兼容 API
pub struct LmStudioProvider { ... }   // → 本地 HTTP 服务
```

两者都是 OpenAI 兼容接口，所以 `DeepSeekProvider` 和 `LmStudioProvider` 共享同一套 HTTP 请求逻辑，只是 base_url 和 api_key 不同。

切换方式：
```bash
# 使用 DeepSeek API
LLM_PROVIDER=deepseek

# 切换到 LM Studio 本地模型
LLM_PROVIDER=lmstudio
```

### Agent B 系统提示词（草案）

```
你是一个专业的中长篇小说写作助手 (Agent B)。

你的核心职责：根据大纲、设定和角色信息，生成高质量的正文内容。

工作原则:
1. 严格遵循"先大纲后正文"流程，没有大纲不写正文
2. 写正文前必须读：卷大纲 + 章大纲 + 角色表 + 文风样例
3. 正文是纯文本，禁止任何 Markdown 或格式标记
4. 每次操作返回摘要（写了什么、字数、下一章建议）

质量标准:
- 人物行为符合角色设定
- 对话体现人物性格
- 情节推进有因果逻辑
- 章尾钩子自然承接下一章

注意:
- 不要复述大纲内容给用户
- 不要评价自己的写作
- 直接输出正文，不要附加说明
```

### Agent B 工具响应格式

```json
{
  "status": "ok",
  "data": {
    "summary": "生成了第五章正文，3120字。主角在酒吧与反派首次交锋，留下线索指向地下势力。",
    "word_count": 3120,
    "chapter_id": "tc-005",
    "chapter_name": "第五章 暗流",
    "next_hook": "酒吧老板的身份引发新的疑问"
  }
}
```

## 项目结构

```
PrivateNovelWriter/
├── Cargo.toml              # workspace root
├── kernel/                 # 核心库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── command/        # 命令系统
│       │   ├── mod.rs      # Command 枚举 + 分发
│       │   ├── data.rs     # 数据命令: read/write/delete/patch
│       │   └── agent.rs    # 创作命令: writeChapter/revise/planOutline
│       ├── handler/        # 数据命令执行器
│       │   ├── read.rs
│       │   ├── write.rs
│       │   ├── delete.rs
│       │   └── patch.rs
│       ├── models/         # 数据模型
│       │   ├── novel.rs
│       │   ├── character.rs
│       │   ├── outline.rs
│       │   └── text.rs
│       ├── db/             # SQLite
│       │   ├── schema.rs
│       │   └── crud.rs     # 通用 CRUD
│       ├── storage/        # .txt 文件读写
│       └── agent/          # Agent B
│           ├── mod.rs
│           ├── llm.rs      # LLM 接口抽象
│           ├── write_chapter.rs
│           ├── revise.rs
│           ├── plan.rs
│           └── evaluate.rs
├── cli/                    # CLI 端口
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── commands/       # 子命令实现
├── desktop/                # Desktop 端口 (Tauri + Svelte)
│   ├── Cargo.toml
│   ├── src-tauri/
│   │   └── src/
│   │       └── lib.rs
│   └── src/
│       ├── App.svelte
│       └── routes/
└── docs/
    ├── design.md
    └── tools.md
```

## 存储设计

### 磁盘布局

```
my-novel/
├── project.db              # SQLite 数据库
└── text/                   # 正文 .txt 文件
    ├── phase-1/
    │   ├── ch-001.txt
    │   ├── ch-002.txt
    │   └── ...
    └── phase-2/
```

### SQLite Schema

```sql
CREATE TABLE novel (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    created      TEXT NOT NULL,
    modified     TEXT NOT NULL,
    active       INTEGER NOT NULL DEFAULT 0,
    total_char   INTEGER NOT NULL DEFAULT 0,
    chapter_char INTEGER NOT NULL DEFAULT 2000,
    sensitivity  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE setting (
    novel_id     TEXT PRIMARY KEY REFERENCES novel(id),
    title        TEXT NOT NULL DEFAULT '',
    inspiration  TEXT NOT NULL DEFAULT '',
    description  TEXT NOT NULL DEFAULT '',
    type         INTEGER NOT NULL DEFAULT 0,
    tags_json    TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE character (
    id           TEXT PRIMARY KEY,
    novel_id     TEXT NOT NULL REFERENCES novel(id),
    name         TEXT NOT NULL,
    char_type    INTEGER NOT NULL DEFAULT 0,
    age          INTEGER NOT NULL DEFAULT 0,
    relationship TEXT NOT NULL DEFAULT ''
);

CREATE TABLE plugin (
    novel_id     TEXT PRIMARY KEY REFERENCES novel(id),
    name         TEXT NOT NULL DEFAULT '',
    plugin_type  INTEGER NOT NULL DEFAULT 0,
    description  TEXT NOT NULL DEFAULT '',
    benefit      TEXT NOT NULL DEFAULT '',
    cost         TEXT NOT NULL DEFAULT ''
);

CREATE TABLE outline_phase (
    id           TEXT PRIMARY KEY,
    novel_id     TEXT NOT NULL REFERENCES novel(id),
    sort         INTEGER NOT NULL DEFAULT 0,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT ''
);

CREATE TABLE outline_chapter (
    id               TEXT PRIMARY KEY,
    phase_id         TEXT NOT NULL REFERENCES outline_phase(id),
    sort             INTEGER NOT NULL DEFAULT 0,
    chapter_name     TEXT NOT NULL,
    content          TEXT NOT NULL DEFAULT '',
    hook             TEXT NOT NULL DEFAULT '',
    text_chapter_id  TEXT REFERENCES text_chapter(id)
);

CREATE TABLE text_phase (
    id       TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL REFERENCES novel(id),
    sort     INTEGER NOT NULL DEFAULT 0,
    name     TEXT NOT NULL
);

CREATE TABLE text_chapter (
    id         TEXT PRIMARY KEY,
    phase_id   TEXT NOT NULL REFERENCES text_phase(id),
    sort       INTEGER NOT NULL DEFAULT 0,
    name       TEXT NOT NULL DEFAULT '',
    file_path  TEXT NOT NULL DEFAULT '',
    word_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE detail_sample (
    id       TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL REFERENCES novel(id),
    title    TEXT NOT NULL,
    content  TEXT NOT NULL DEFAULT ''
);
```

**设计要点：**
- outline_chapter.text_chapter_id 可空 → 先大纲后正文
- 排序用 sort INTEGER，简单够用
- file_path 存相对路径
- 小量 JSON 用 TEXT 字段（serde_json 读写）

### 正文 .txt 存储

- UTF-8 纯文本，**严格禁止 Markdown**
- 段落间用空行分隔
- `\n` 换行
- 按 `text/{phase_name}/{chapter_id}.txt` 路径组织

## 开发阶段

| Phase | 内容 |
|---|---|
| **P1** | kernel 数据模型 + SQLite schema + .txt 存储 |
| **P2** | CLI 完整命令集（直连数据层） |
| **P3** | Agent B 集成（LLM 编排创作命令） |
| **P4** | Desktop 端口（Tauri + Svelte + Agent B 对话界面） |
| **P5** | 高级特性（批量生产、导出等） |

### Phase 1 范围

1. `kernel/models/` — Rust struct 翻译 Data.cs
2. `kernel/db/` — SQLite schema 初始化 + 通用 CRUD
3. `kernel/storage/` — .txt 文件读写
4. `kernel/command/` — 命令枚举 + 分发骨架
5. `kernel/handler/` — 数据命令执行器
6. `cli/` — 基础命令接通: novel new/open, outline add/list, text create/read/write
7. `cargo test` 验证: 新建→建大纲→写正文 流程可跑通

> P1 不做:
> - Agent B (LLM)
> - 创作命令编排
> - Desktop
> - 搜索
