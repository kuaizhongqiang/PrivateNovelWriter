# Prompt 架构设计

> 目标：最大化 DeepSeek Prefix Cache 命中率，同时利用 Reasoning + Tool Calling 能力。
> 状态：设计文档 — 进入实现前需细化。

---

## 核心约束

DeepSeek 的 prefix cache 命中条件：

- 从 token 0 开始逐字节匹配
- 命中后成本降至约 1/50
- cache 以 64 tokens 为单位
- 首次请求预热，后续同前缀请求命中

这意味着 prompt 的不变部分必须在最前端且结构绝对稳定。

---

## Prompt 结构

```text
[System Prompt]          ← 固定，从不变化
[世界观设定]              ← 固定，项目创建后不变
[角色表]                  ← 固定（或按需检索后保持稳定）
[文风样例]                ← 固定（或按场景匹配后保持稳定）
───────── 分割线 ─────────
[当前章大纲 + hook]       ← 变化，每次不同
[写作要求 / brief]        ← 变化
```

### 不变部分 (Cached Prefix)

这部分一旦预热，后续请求命中率极高：

- **System Prompt** — Agent B 的角色定义、写作原则、输出格式
- **世界观设定** — 项目的 setting 内容（title, inspiration, description, tags）
- **角色概要表** — 所有角色的 "姓名 + 角色类型" 精简列表
- **文风样例首段** — 最多 3 个样例，每个取前 200 字

### 变化部分 (Cache Miss)

这部分每请求不同，必须放在末尾：

- **本章大纲** — outline_chapter.content + hook
- **用户 brief** — 额外的写作指示
- **前情摘要** — 上一章的自动摘要（如果启用）

---

## Provider 感知策略

Server 根据 `LLM_PROVIDER` 的值切换行为：

### DeepSeek (Remote API)

- 严格保持 prompt 前缀结构不变
- 同一项目的连续写作请求共享同一个 prefix cache
- 优先复用 session，减少重建前缀的次数

### LM Studio (Local LLM)

- 不受 prefix cache 约束
- session 可随意重建，上下文可以即时压缩
- prompt 结构可以更灵活

---

## Session 生命周期

```
用户打开项目 → 创建 Server session
                    ↓
            首次写请求 → 预热 prefix cache
                    ↓
            后续写请求 → 命中 prefix cache (仅传输变化部分)
                    ↓
            项目关闭 → 销毁 session, cache 自然过期
```

### 长 session 策略

- 同一项目的写作操作在同一个 session 内完成
- 跨 session 时重建前缀（新 session → 新 cache）
- Server 启动时不做预热，首次请求自然预热

---

## Reasoning + Tool Calling

DeepSeek 支持在推理过程中调用工具：

```text
请求 1: 写第五章
  → LLM 思考: 需要角色信息
  → 调 read_character 工具
  → 继续思考: 看大纲
  → 调 read_outline 工具
  → 输出正文

请求 2: 评估第五章
  → LLM 思考: 需要读全文
  → 调 read_chapter 工具
  → 输出评估结果
```

### 注意

- 涉及工具调用后，`reasoning_content` 必须完整传回后续请求
- 但这与 prefix cache 存在张力——reasoning 内容每轮不同会 break cache
- **权衡策略**：工具调用限定在需要即时数据查询的场景；写作正文本身使用稳定的 prompt 前缀

---

## 待细化

1. System Prompt 的精确文本
2. 角色表在 prompt 中的格式（JSON vs 文本）
3. 样例选择策略（按场景匹配的具体规则）
4. Session 过期时间
5. Token 消耗和 cache 命中率的采集方式
