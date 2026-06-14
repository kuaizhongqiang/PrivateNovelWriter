# Agent A 操作手册 (Playbook)

> 目标读者：蜜蜜（Agent A / AI 助手）
> 本文档是 PNW 的「操作手册」而非「知识手册」——告诉你了蜜一步步做什么。

---

## Step 0: 检查环境

每次收到主人写作请求时，先做探活检查。

```
GET /api/health
```

| 响应 | 含义 | 下一步 |
|------|------|--------|
| `{"status":"ok"}` | Server 活着 | 继续 Step 1 |
| 连接失败 | Server 没启动 | 执行 `pnw server --host 127.0.0.1 --port 3000 --project <path>` |
| HTTP 5xx | Server 异常 | 告诉主人 Server 异常，请求重启 |

**探活注意事项：**
- `/api/health` 不查 DB、不消耗资源，可以高频调用
- 如果不知道 `--project` 路径，问主人项目文件夹在哪
- OpenClaw 重启后 Server 不会自动重启，需要重新启动

### 启动命令

```bash
# 后台运行（Linux/Mac）
nohup pnw server --host 127.0.0.1 --port 3000 --project /path/to/project > pnw.log 2>&1 &

# 前台运行（Windows）
pnw server --host 127.0.0.1 --port 3000 --project C:\path\to\project
```

---

## Step 1: 理解用户需求

### 写作问诊

收到主人请求后，先问清楚几个关键问题：

- **写什么？** — 书名、类型（都市/玄幻/历史/奇幻/武侠/科幻）
- **多长？** — 短篇（1-3万）、中篇（10-30万）、长篇（50万+）
- **什么风格？** — 传统小说还是网络小说？有没有参考作品？
- **角色明确吗？** — 已有角色设定还是从零开始？

### 流程选择

| 场景 | 推荐路径 |
|------|---------|
| 一句话灵感 → 几章 | 设定 → 大纲 → 逐章写 → 评估 → 修改 |
| 已有完整大纲 | 跳过设定/大纲阶段，直接写正文 |
| 续写已有项目 | 查 stats 了解进度，从最新未写章开始 |
| 局部修改 | 读目标章节 → 调 revise → 确认 |

---

## Step 2: 设定世界观

开始写正文之前，先确认设定是否完整。

```
GET /api/setting
```

### 如果设定为空

帮主人建立基础设定：

```json
POST /api/setting
{
  "title": "书名",
  "inspiration": "灵感来源",
  "description": "一句话简介",
  "novel_type": 0,   // 0=都市 1=玄幻 2=历史 3=奇幻 4=武侠 5=科幻
  "tags": ["标签1", "标签2"]
}
```

**建议问主人一句：** 「要先建一下世界观设定吗？这样我写出来的内容会更一致。」

### 如果设定已存在

读出来给主人看看，确认是否需要调整。

---

## Step 3: 建立角色

```
GET /api/characters
```

### 如果角色为空

根据小说类型推荐基础角色：

```json
POST /api/characters
{
  "name": "角色名",
  "char_type": 0,   // 0=男主 1=女主 2=其他
  "age": 25,
  "relationship": "主角"
}
```

**角色类型速查：**

| char_type | 含义 |
|-----------|------|
| 0 | 男主（主角） |
| 1 | 女主（女主角） |
| 2 | 其他（配角/反派） |

**建议一次建 3-5 个核心角色就够了**，后续可以随时追加。

---

## Step 4: 规划大纲

**这是最重要的步骤！没有大纲不要写正文！**

### 4.1 创建大纲卷

```json
POST /api/command
{
  "command": "create_outline_phase",
  "args": { "name": "第一卷：开端" }
}
```

一般来说 10 章一卷，一部长篇 5-10 卷。

### 4.2 创建大纲章

```json
POST /api/command
{
  "command": "create_outline_chapter",
  "args": {
    "phase_id": "<上一步返回的 id>",
    "name": "第一章 相遇"
  }
}
```

**每章大纲建议写清楚：**
- `content` — 本章情节概要（50-100 字）
- `hook` — 章尾钩子（10-20 字），决定下一章起点

### 4.3 展示给主人确认

```
GET /api/outline
```

把大纲树呈现给主人，确认后再进入写作阶段。**不要跳过确认步骤。**

---

## Step 5: 确认项目状态

```json
GET /api/stats
```

检查：
- `planned_chapters` — 规划了多少章
- `written_chapters` — 已写了多少章
- `total_written` — 总字数
- `completion_pct` — 完成百分比

### 5.1 创建正文卷

```bash
pnw text phase create <novel_id> "第一卷"
```

### 5.2 创建正文章节

```bash
pnw text chapter create <text_phase_id> --from-outline <outline_chapter_id> --name "第一章 相遇"
```

注意 `--from-outline` 参数会自动关联大纲章，这样大纲树上就能看到哪些章已写。

---

## Step 6: 写正文（调用 Agent B）

```json
POST /api/agent/write
{
  "chapter_id": "<text_chapter_id>",
  "brief": "主角在酒吧与反派首次交锋，注意氛围描写"
}
```

**brief 写得越具体，Agent B 输出质量越高。**

### 写一章的注意事项

- Agent B 自动读大纲 + 角色 + 样例，不需要你手动拼 prompt
- 返回 `summary` 字段告诉你写了多少字、内容梗概
- 每章约 2000 字（取决于项目配置的 `chapter_char`）

### 批量写

建议写 3-5 章就评估一次，不要一口气写 50 章不回头。

---

## Step 7: 评估与修改

### 评估已写章节

```json
POST /api/agent/evaluate/{chapter_id}
```

返回质量评估结果。

### 修改章节

根据评估结果，调 revise 改进：

```json
POST /api/agent/revise
{
  "chapter_id": "<text_chapter_id>",
  "feedback": "对话部分需要更自然，张三的台词不够符合他的性格"
}
```

**反馈越具体，修改效果越好。**

---

## Step 8: 导出

```json
GET /api/export/txt?limit=100
```

导出全文。`limit` 参数可限制返回章节数（防止大项目 OOM）。

---

## 错误处理指南

### 错误码（规划中，当前版本只有文字描述）

当前所有错误返回 `{ "status": "error", "error": "..." }`。

**常见错误与应对：**

| 错误特征 | 原因 | 应对 |
|----------|------|------|
| `Not found: Chapter x` | chapter_id 不存在 | 重新 list 确认 ID |
| `FOREIGN KEY constraint` | 关联的实体不存在 | 检查 novel_id / phase_id 是否正确 |
| `No novels found` | 项目刚创建但没设 active | 调 `novel show` 确认 |
| `No LLM provider configured` | .env 没配或找不到 | 检查环境变量 `LLM_PROVIDER`/`LLM_API_KEY` |
| 连接被拒绝 | Server 没启动 | 走 Step 0 探活流程 |
| HTTP 4xx | 请求参数有误 | 检查请求体 JSON 格式 |
| HTTP 5xx | Server 内部错误 | 重试；如果频繁出现告诉主人 |

### 重试策略

| 场景 | 策略 |
|------|------|
| 网络超时 | 等待 3 秒后重试 1 次 |
| Server 返回 5xx | 等待 5 秒后重试 1 次 |
| LLM 调用失败 | 不自动重试（可能扣费了），告诉主人 |
| 参数错误 (4xx) | 不重试，修复参数 |

---

## 反模式清单

- ❌ **没建大纲就写正文** — Agent B 不知道写什么，质量极差
- ❌ **一口气写 50 章不评估** — 写到后面发现前面方向错了，返工成本极高
- ❌ **在 brief 里塞角色表** — Server 自动处理角色注入，不需要你手动拼
- ❌ **一次创建几十个角色** — 角色太多后 context 装不下，够用就行
- ❌ **依赖 Agent B 记住前文** — 每章是独立请求，LLM 不会自动记住之前写了什么

---

## 快速参考卡

### 检查 → 规划 → 写作 → 评估 → 导出的完整流程

```
1. GET  /api/health                    ← 探活
2. GET  /api/project                   ← 确认有项目
3. GET  /api/setting                   ← 读设定
4. GET  /api/characters                ← 读角色
5. POST /api/command create_outline_*  ← 建大纲
6. GET  /api/outline                   ← 给主人确认
7. CLI: text phase/chapter create      ← 建正文结构
8. POST /api/agent/write               ← 写正文（循环）
9. POST /api/agent/evaluate            ← 评估
10. POST /api/agent/revise             ← 修改
11. GET  /api/export/txt               ← 导出
```
