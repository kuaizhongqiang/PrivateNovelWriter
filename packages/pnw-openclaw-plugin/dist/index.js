import { Type } from "@sinclair/typebox";
import { defineToolPlugin } from "openclaw/plugin-sdk/tool-plugin";
const BASE_URL_DESC = "PNW Server URL, e.g. http://127.0.0.1:3000";
async function api(baseUrl, path, opts) {
    const res = await fetch(`${baseUrl}/api${path}`, {
        headers: { "Content-Type": "application/json" },
        ...opts,
    });
    const json = await res.json();
    if (json.status === "error") {
        throw new Error(`[${json.error_code || "UNKNOWN"}] ${json.error}`);
    }
    return json.data;
}
export default defineToolPlugin({
    id: "pnw-writer",
    name: "PrivateNovelWriter",
    description: "小说写作助手 — 规划大纲、生成正文、评估修改",
    configSchema: Type.Object({
        serverUrl: Type.String({ description: BASE_URL_DESC }),
    }),
    tools: (tool) => [
        // ── Project ──
        tool({
            name: "get_project",
            description: "读取当前项目的基本信息（名称、字数目标等）",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/project");
            },
        }),
        tool({
            name: "get_stats",
            description: "获取项目完整统计：总字数、章节数、完成度百分比",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/stats");
            },
        }),
        // ── Outline ──
        tool({
            name: "get_outline",
            description: "读取完整大纲树：所有卷和章，标注哪些章已写正文",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/outline");
            },
        }),
        tool({
            name: "create_outline_phase",
            description: "创建大纲卷（如'第一卷：开端'），返回新卷 ID",
            parameters: Type.Object({
                name: Type.String({ description: "卷名" }),
            }),
            async execute(params, config) {
                const data = await api(config.serverUrl, "/command", {
                    method: "POST",
                    body: JSON.stringify({ command: "create_outline_phase", args: { name: params.name } }),
                });
                return data;
            },
        }),
        tool({
            name: "create_outline_chapter",
            description: "在指定卷下创建大纲章（含情节概要和章尾钩子）",
            parameters: Type.Object({
                phase_id: Type.String({ description: "所属大纲卷 ID" }),
                name: Type.String({ description: "章名" }),
                content: Type.Optional(Type.String({ description: "情节概要" })),
                hook: Type.Optional(Type.String({ description: "章尾钩子" })),
            }),
            async execute(params, config) {
                return api(config.serverUrl, "/command", {
                    method: "POST",
                    body: JSON.stringify({ command: "create_outline_chapter", args: params }),
                });
            },
        }),
        // ── Character ──
        tool({
            name: "list_characters",
            description: "列出项目所有角色",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/characters");
            },
        }),
        tool({
            name: "create_character",
            description: "添加新角色。char_type: 0=男主 1=女主 2=其他",
            parameters: Type.Object({
                name: Type.String({ description: "角色名" }),
                char_type: Type.Optional(Type.Number({ description: "0=男主 1=女主 2=其他", default: 2 })),
                age: Type.Optional(Type.Number({ description: "年龄" })),
                relationship: Type.Optional(Type.String({ description: "关系描述" })),
            }),
            async execute(params, config) {
                return api(config.serverUrl, "/characters", {
                    method: "POST",
                    body: JSON.stringify(params),
                });
            },
        }),
        // ── Setting ──
        tool({
            name: "get_setting",
            description: "读取世界观设定（书名、灵感、简介、类型、标签）",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/setting");
            },
        }),
        tool({
            name: "update_setting",
            description: "更新世界观设定",
            parameters: Type.Object({
                title: Type.Optional(Type.String({ description: "书名" })),
                inspiration: Type.Optional(Type.String({ description: "灵感来源" })),
                description: Type.Optional(Type.String({ description: "作品简介" })),
                novel_type: Type.Optional(Type.Number({ description: "0=都市 1=玄幻 2=历史 3=奇幻 4=武侠 5=科幻" })),
                tags: Type.Optional(Type.Array(Type.String(), { description: "标签列表" })),
            }),
            async execute(params, config) {
                return api(config.serverUrl, "/setting", {
                    method: "POST",
                    body: JSON.stringify(params),
                });
            },
        }),
        // ── Chapter (read/write) ──
        tool({
            name: "list_chapters",
            description: "列出所有已创建的正文章节（含所属卷、字数）",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/chapters");
            },
        }),
        tool({
            name: "read_chapter",
            description: "读取指定章节的正文全文",
            parameters: Type.Object({
                chapter_id: Type.String({ description: "正文章节 ID" }),
            }),
            async execute(params, config) {
                return api(config.serverUrl, `/chapter/${params.chapter_id}`);
            },
        }),
        // ── Agent B (writing) ──
        tool({
            name: "write_chapter",
            description: "写正文！Agent B 自动读大纲+角色+样例，调用 LLM 生成正文并写入文件",
            parameters: Type.Object({
                chapter_id: Type.String({ description: "正文章节 ID" }),
                brief: Type.String({ description: "写作要求，越具体质量越高" }),
            }),
            async execute(params, config) {
                return api(config.serverUrl, "/agent/write", {
                    method: "POST",
                    body: JSON.stringify(params),
                });
            },
        }),
        tool({
            name: "revise_chapter",
            description: "修改已写的正文章节，Agent B 根据反馈意见调用 LLM 修改",
            parameters: Type.Object({
                chapter_id: Type.String({ description: "正文章节 ID" }),
                feedback: Type.String({ description: "修改意见，越具体效果越好" }),
            }),
            async execute(params, config) {
                return api(config.serverUrl, "/agent/revise", {
                    method: "POST",
                    body: JSON.stringify(params),
                });
            },
        }),
        tool({
            name: "evaluate_chapter",
            description: "评估已写章节的质量，返回分析结果",
            parameters: Type.Object({
                chapter_id: Type.String({ description: "正文章节 ID" }),
            }),
            async execute(params, config) {
                return api(config.serverUrl, `/agent/evaluate/${params.chapter_id}`, { method: "POST" });
            },
        }),
        // ── Export ──
        tool({
            name: "export_txt",
            description: "导出全文合并的 TXT。limit 可选，限制返回章节数防止大项目 OOM",
            parameters: Type.Object({
                limit: Type.Optional(Type.Number({ description: "最大返回章节数" })),
            }),
            async execute(params, config) {
                const qs = params.limit ? `?limit=${params.limit}` : "";
                return api(config.serverUrl, `/export/txt${qs}`);
            },
        }),
        // ── Samples ──
        tool({
            name: "list_samples",
            description: "列出文风样例列表",
            parameters: Type.Object({}),
            async execute(_params, config) {
                return api(config.serverUrl, "/samples");
            },
        }),
    ],
});
