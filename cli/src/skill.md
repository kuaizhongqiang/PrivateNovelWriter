# PNW 写作领域知识手册

> 目标读者：Agent A（AI 助手）
> 本文档定义 PrivateNovelWriter 的数据模型和工作流，Agent A 应据此编排写作任务。

---

## 数据模型

```
小说 (Novel)
 ├── 大纲卷 (OutlinePhase)        ← 创作规划
 │    └── 大纲章 (OutlineChapter)  ← 每章的情节概要 + 钩子
 ├── 正文卷 (TextPhase)           ← 实际产出
 │    └── 正文章 (TextChapter)     ← 正文 .txt 文件
 ├── 角色 (Character)             ← 人物表
 ├── 设定 (Setting)               ← 世界观
 ├── 文风样例 (DetailSample)      ← 风格参考
 └── 金手指 (Plugin)              ← 系统/超能力设定
```

### 关键关联

- **大纲章 (`outline_chapter`)** 通过 `text_chapter_id` 关联到已写的正文章
- `text_chapter_id` 为空 = 已规划但未写正文
- 正文内容存在 `.txt` 文件中，不在数据库里

### 角色类型

| 值 | 类型 | 说明 |
| :--- | :--- | :--- |
| 0 | 男主 | 主角（通常唯一） |
| 1 | 女主 | 女主角 |
| 2 | 其他 | 配角、反派等 |

### 小说类型

| 值 | 类型 |
| :--- | :--- |
| 0 | 都市 |
| 1 | 玄幻 |
| 2 | 历史 |
| 3 | 奇幻 |
| 4 | 武侠 |
| 5 | 科幻 |

---

## 推荐工作流

### 标准流程

```text
世界观设定 → 大纲规划 → 逐章写作 → 评估 → 修改
```

### 步骤详解

**1. 设定世界观（可选，但推荐）**

在写大纲前先建立设定，LLM 生成的内容会更一致。

调用 `POST /api/setting` 写入书名、灵感、简介、标签。

**2. 规划大纲**

- 先创建大纲卷 (`POST /api/command` `list_outline_phases`)
- 再往卷下添加大纲章，每章包含：章名、情节概要（content）、章尾钩子（hook）
- 钩子很重要——它决定下一章的起始点

**3. 写正文**

- 先创建正文卷和正文章（通过 `POST /api/command` 或直接调 CRUD API）
- 大纲章创建时 `text_chapter_id` 为空 → 写完正文后自动关联
- 调用 `POST /api/agent/write`，传入 chapter_id 和 brief
- Agent B 会自动读大纲 + 角色 + 样例，调 LLM 生成正文

**4. 评估**

`POST /api/agent/evaluate/{chapter_id}` 对已写章节做质量评估。

**5. 修改**

`POST /api/agent/revise` 传入反馈意见，Agent B 调 LLM 修改。

---

## REST API 速查

### 项目信息

| 方法 | 路径 | 说明 |
| :--- | :--- | :--- |
| GET | `/api/health` | 探活（轻量，无 DB） |
| GET | `/api/status` | Server 信息（版本、session） |
| GET | `/api/tools` | 工具发现（端点列表 + 命令名） |
| GET | `/api/project` | 项目基本信息 |
| GET | `/api/stats` | 完整统计（字数、章节数、完成度） |
| GET | `/api/outline` | 大纲树结构 |
| GET | `/api/chapters` | 所有正文章节列表 |

### 读写数据

| 方法 | 路径 | 说明 |
| :--- | :--- | :--- |
| GET | `/api/chapter/{id}` | 读正文章节（含正文内容） |
| PUT | `/api/chapter/{id}` | 保存正文章节（覆盖全文） |
| GET | `/api/characters` | 列出角色 |
| GET | `/api/setting` | 读世界观设定 |
| POST | `/api/setting` | 更新设定 |
| GET | `/api/samples` | 文风样例列表 |
| POST | `/api/command` | 通用命令接口（支持读写，详见下方） |

### 通用命令 (`POST /api/command`)

请求体 `{ "command": "...", "args": { ... }, "client_request_id": "..." }` 支持的命令：

> `client_request_id` 可选参数。提供后，相同 ID 的重复请求不会重复执行，而是返回首次执行的结果。用于网络超时后安全重试。

| command | args | 说明 |
| :--- | :--- | :--- |
| `get_outline` | — | 大纲树 |
| `get_novel` | — | 小说信息 |
| `list_characters` | — | 角色列表 |
| `get_setting` | — | 设定详情 |
| `list_samples` | — | 文风样例列表 |
| `get_plugin` | — | 金手指设定 |
| `list_outline_phases` | — | 大纲卷列表 |
| `list_outline_chapters` | `{ phase_id }` | 卷下的大纲章列表 |
| `list_text_phases` | — | 正文卷列表 |
| `list_text_chapters` | `{ phase_id }` | 正文章节列表 |
| `create_outline_phase` | `{ name }` | 创建大纲卷 |
| `create_outline_chapter` | `{ phase_id, name, content?, hook? }` | 创建大纲章（自动计算 sort） |
| `create_text_phase` | `{ name }` | 创建正文卷 |
| `create_text_chapter` | `{ phase_id, name, from_outline }` | 创建正文章节（自动建 .txt 路径） |
| `create_character` | `{ name, char_type?, age?, relationship? }` | 创建角色 |
| `write_setting` | `{ title?, inspiration?, description?, novel_type?, tags? }` | 更新设定 |
| `delete_character` | `{ id }` | 删除角色 |
| `delete_outline_phase` | `{ phase_id }` | 删除大纲卷 |
| `delete_outline_chapter` | `{ id }` | 删除大纲章 |
| `get_unwritten_chapters` | — | 列出所有已规划但未写正文的大纲章 |
| `get_phase_progress` | — | 每卷的规划/完成章节数 |

### 写作

| 方法 | 路径 | 说明 |
| :--- | :--- | :--- |
| POST | `/api/agent/write` | 写正文（Agent B 编排） |
| POST | `/api/agent/revise` | 修改正文 |
| POST | `/api/agent/evaluate/{id}` | 评估章节 |

### 导出

| 方法 | 路径 | 说明 |
| :--- | :--- | :--- |
| POST | `/api/export/txt` | 导出合并全文 |

---

## 反模式

- ❌ **没有大纲直接写正文** — LLM 缺乏上下文，章节之间容易脱节
- ❌ **一次性写太多章** — 建议逐章或三五章一批，每批后做评估
- ❌ **依赖 LLM 记忆前文** — 每章是独立请求，LLM 不会自动记住之前写了什么
- ❌ **在正文 prompt 里塞角色表全文** — Server 会自动处理角色注入，不需要你手动拼

---

## 最佳实践

- ✅ 先写 `setting` 建立世界观，再规划大纲
- ✅ 每章大纲写清楚 `content` 和 `hook`，LLM 输出质量会显著提升
- ✅ 写几章后调用 `evaluate`，根据反馈 `revise`
- ✅ 用 `stats` 追踪进度，根据完成度决定写作节奏
- ✅ 文风样例放 2-3 个代表性段落即可，不需要大量堆砌

---

## 关于 Server

- 启动：`pnw server --host 0.0.0.0 --port 3000 --project /path/to/project`
- 环境变量 `PNW_PROJECT` 可替代 `--project`
- 所有 API 返回 `{ status, data, error }` 统一格式
- Agent B 的 LLM 调用通过 `.env` 配置：`LLM_PROVIDER`、`LLM_API_KEY`、`LLM_MODEL`
