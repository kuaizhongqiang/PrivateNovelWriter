# 写作专家 Agent B 工具定义

Agent B 对外暴露的工具集。共 4 个工具，覆盖所有实体类型的完整 CRUD。

## 工具总览

| 工具 | 功能 |
|---|---|
| `read` | 读取任意实体数据 |
| `write` | 写入/更新任意实体数据 |
| `delete` | 删除任意实体 |
| `patch` | 局部替换正文/大纲中的文本 |

## 实体类型表

所有工具通过 `entity` 参数指定操作目标：

| entity 值 | 对应数据 | 说明 |
|---|---|---|
| `novel` | 小说基本信息 + 配置 | 每部小说一条 |
| `setting` | 世界观设定 | 每部小说一条 |
| `character` | 角色 | 多条 |
| `outline_phase` | 卷大纲 | 多条 |
| `outline_chapter` | 章大纲 | 多条 |
| `text_chapter` | 正文章节 | 多条 |
| `sample` | 文风样例 | 多条 |
| `plugin` | 系统/金手指设定 | 每部小说一条（可选） |

---

## 工具定义

### read

读取任意实体数据。

```json
{
  "name": "read",
  "description": "读取任意实体数据。不传 id 返回列表，传 id 返回详情。text_chapter 的 content 字段包含正文全文",
  "input_schema": {
    "type": "object",
    "properties": {
      "novel_id": { "type": "string" },
      "entity": {
        "type": "string",
        "enum": ["novel", "setting", "character", "outline_phase", "outline_chapter", "text_chapter", "sample", "plugin"]
      },
      "id": { "type": "string", "description": "实体 ID。不传则返回列表（novel/setting/plugin 忽略了 id 仍返回单条）" }
    },
    "required": ["novel_id", "entity"]
  }
}
```

**返回示例 `read novel_id=A entity=outline_phase`：**

```json
{
  "phases": [
    {
      "id": "ph-001",
      "name": "第一卷：初入都市",
      "description": "主角来到陌生城市，开始新生活",
      "sort": 1,
      "chapters": [
        { "id": "oc-001", "chapter_name": "第一章 抵达", "content": "主角坐火车到达", "hook": "站台上一个神秘身影", "text_chapter_id": "tc-001" },
        { "id": "oc-002", "chapter_name": "第二章 租房", "content": "找房子的过程", "hook": "隔壁传来奇怪声音", "text_chapter_id": null }
      ]
    }
  ]
}
```

> outline_phase 返回时携带其下所有 outline_chapter 列表，outline_chapter 携带 text_chapter_id 表示该章是否已生成正文。

---

### write

写入（新建或更新）任意实体。有 id 则更新，无 id 则新建。

```json
{
  "name": "write",
  "description": "写入任意实体。id 留空则新建，有值则更新。只传要改的字段即可",
  "input_schema": {
    "type": "object",
    "properties": {
      "novel_id": { "type": "string", "description": "小说 ID（新建 novel 时不需要）" },
      "entity": {
        "type": "string",
        "enum": ["novel", "setting", "character", "outline_phase", "outline_chapter", "text_chapter", "sample", "plugin"]
      },
      "id": { "type": "string", "description": "实体 ID。留空=新建，有值=更新" },
      "data": {
        "type": "object",
        "description": "实体的字段。根据 entity 类型不同支持不同字段"
      }
    },
    "required": ["entity", "data"]
  }
}
```

**各 entity 的 data 字段：**

| entity | data 字段 |
|---|---|
| `novel` | `name`, `total_char`, `chapter_char`, `sensitivity` (0=正常 1=轻微 2=纯肉) |
| `setting` | `title`, `inspiration`, `description`, `type` (0=都市 1=玄幻 2=历史 3=奇幻 4=武侠 5=科幻), `tags` |
| `character` | `name`, `char_type` (0=男主 1=女主 2=其他), `age`, `relationship` |
| `outline_phase` | `name`, `description`, `sort` |
| `outline_chapter` | `phase_id`, `chapter_name`, `content` (情节概要), `hook` (章尾钩子), `sort` |
| `text_chapter` | `phase_id`, `sort`, `name`, `content` (正文纯文本, 会被写入 .txt 文件) |
| `sample` | `title` (标签), `content` (样例段落) |
| `plugin` | `name`, `plugin_type` (0=系统 1=天赋 2=道具 3=技能), `description`, `benefit`, `cost` |

> **注意:** write text_chapter 时传 `content` 会覆盖该章节的 .txt 全文。如果只是改一小段，用 patch 工具。

> **注意:** write outline_chapter 时传 `phase_id` 用于把章挂到指定卷下。新建时必须传。

---

### delete

删除任意实体。

```json
{
  "name": "delete",
  "description": "删除实体。删除 outline_phase 会级联删除其下所有 outline_chapter。delete text_chapter 会同时删除 .txt 文件",
  "input_schema": {
    "type": "object",
    "properties": {
      "novel_id": { "type": "string" },
      "entity": {
        "type": "string",
        "enum": ["character", "outline_phase", "outline_chapter", "text_chapter", "sample", "plugin"]
      },
      "id": { "type": "string" }
    },
    "required": ["novel_id", "entity", "id"]
  }
}
```

> novel 和 setting 不支持删除（一部小说只有一个，删除即删项目）。

---

### patch

局部替换正文或大纲中的文本，不需要发送全文。

```json
{
  "name": "patch",
  "description": "局部替换文本内容。在原文中定位 old_text 并替换为 new_text。old_text 必须唯一匹配，否则操作失败",
  "input_schema": {
    "type": "object",
    "properties": {
      "novel_id": { "type": "string" },
      "entity": {
        "type": "string",
        "enum": ["outline_chapter", "text_chapter"]
      },
      "id": { "type": "string" },
      "field": {
        "type": "string",
        "description": "要修改的字段",
        "enum": ["content", "hook"]
      },
      "old_text": { "type": "string", "description": "原文中要被替换的文字" },
      "new_text": { "type": "string" }
    },
    "required": ["novel_id", "entity", "id", "field", "old_text", "new_text"]
  }
}
```

> `patch` 适用于 outline_chapter.content / outline_chapter.hook / text_chapter.content。
> text_chapter 的 field 只支持 `content`。

---

## 使用示例

### 场景 1：读大纲 + 写正文

```
Agent A: "写第五章正文"

Agent B 编排:
  step 1: read  novel_id=A  entity=outline_chapter  id=oc-005
          → 拿到大纲: 主角在酒吧遇到反派
  step 2: read  novel_id=A  entity=text_chapter  id=tc-004
          → 读上一章结尾了解当前进度
  step 3: read  novel_id=A  entity=character
          → 读角色表确认人物名称和关系
  step 4: LLM 生成正文
  step 5: write novel_id=A  entity=text_chapter  id=tc-005  data={content: "..."}
  step 6: 返回执行摘要
```

### 场景 2：局部修改

```
Agent A: "把第五章打斗改得更激烈"

Agent B:
  step 1: read  novel_id=A  entity=text_chapter  id=tc-005
          → 拿到全文
  step 2: LLM 决定改 "他一拳打过去" → "他裹挟着雷霆之势一拳轰出"
  step 3: patch novel_id=A  entity=text_chapter  id=tc-005  field=content
          old_text="他一拳打过去"  new_text="他裹挟着雷霆之势一拳轰出"
  step 4: 返回摘要
```

### 场景 3：建大纲

```
Agent A: "规划第一卷的大纲, 十章"

Agent B:
  step 1: read  novel_id=A  entity=setting
          → 读世界观
  step 2: read  novel_id=A  entity=character
          → 读角色
  step 3: LLM 规划十章大纲
  step 4: write novel_id=A  entity=outline_phase  id=ph-001  data={name: "第一卷", ...}
  step 5: write novel_id=A  entity=outline_chapter  data={phase_id: ph-001, chapter_name: "第一章", content: "...", hook: "...", sort: 1}
          write novel_id=A  entity=outline_chapter  data={phase_id: ph-001, chapter_name: "第二章", content: "...", hook: "...", sort: 2}
          ... (x10)
  step 6: 返回摘要
```

### 场景 4：查统计

```
Agent A: "进度怎么样了?"

Agent B:
  step 1: read  novel_id=A  entity=novel
          → 得到 total_char=1000000, name="我的小说"
  step 2: read  novel_id=A  entity=outline_phase
          → 列出所有卷和章，看哪些有 text_chapter_id
  step 3: 统计已写字数、完成度，返回摘要
```
