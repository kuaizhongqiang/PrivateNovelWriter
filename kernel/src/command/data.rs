/// 数据命令：直接对数据层操作，无需 AI 介入
#[derive(Debug, Clone)]
pub enum DataCommand {
    // ── Novel ──
    CreateNovel {
        id: String,
        name: String,
        total_char: i32,
        chapter_char: i32,
        sensitivity: i32,
    },
    GetNovel {
        id: String,
    },
    ListNovels,
    UpdateNovel {
        id: String,
        name: Option<String>,
        total_char: Option<i32>,
        chapter_char: Option<i32>,
        sensitivity: Option<i32>,
    },

    // ── Setting ──
    WriteSetting {
        novel_id: String,
        title: String,
        inspiration: String,
        description: String,
        novel_type: i32,
        tags: Vec<String>,
    },
    GetSetting {
        novel_id: String,
    },

    // ── Character ──
    CreateCharacter {
        id: String,
        novel_id: String,
        name: String,
        char_type: i32,
        age: i32,
        relationship: String,
    },
    GetCharacter {
        id: String,
    },
    ListCharacters {
        novel_id: String,
    },
    UpdateCharacter {
        id: String,
        novel_id: String,
        name: String,
        char_type: i32,
        age: i32,
        relationship: String,
    },
    DeleteCharacter {
        id: String,
    },

    // ── Plugin ──
    WritePlugin {
        novel_id: String,
        name: String,
        plugin_type: i32,
        description: String,
        benefit: String,
        cost: String,
    },
    GetPlugin {
        novel_id: String,
    },
    DeletePlugin {
        novel_id: String,
    },

    // ── Outline Phase ──
    CreateOutlinePhase {
        id: String,
        novel_id: String,
        sort: i32,
        name: String,
        description: String,
    },
    ListOutlinePhases {
        novel_id: String,
    },
    DeleteOutlinePhase {
        phase_id: String,
    },
    UpdateOutlinePhase {
        id: String,
        novel_id: String,
        sort: i32,
        name: String,
        description: String,
    },

    // ── Outline Chapter ──
    CreateOutlineChapter {
        id: String,
        phase_id: String,
        sort: i32,
        chapter_name: String,
        content: String,
        hook: String,
    },
    ListOutlineChapters {
        phase_id: String,
    },
    GetOutlineChapter {
        id: String,
    },
    UpdateOutlineChapter {
        id: String,
        phase_id: String,
        sort: i32,
        chapter_name: String,
        content: String,
        hook: String,
        text_chapter_id: Option<String>,
    },
    DeleteOutlineChapter {
        id: String,
    },

    // ── Outline Tree (卷+章一次返回) ──
    GetOutlineTree {
        novel_id: String,
        phase_id: Option<String>,
    },

    // ── Text Phase ──
    CreateTextPhase {
        id: String,
        novel_id: String,
        sort: i32,
        name: String,
    },
    ListTextPhases {
        novel_id: String,
    },
    DeleteTextPhase {
        phase_id: String,
    },

    // ── Text Chapter ──
    CreateTextChapter {
        id: String,
        phase_id: String,
        sort: i32,
        name: String,
        file_path: String,
    },
    GetTextChapter {
        id: String,
    },
    ListTextChapters {
        phase_id: String,
    },
    UpdateTextChapter {
        id: String,
        name: String,
        word_count: i32,
    },
    DeleteTextChapter {
        id: String,
        /// 章节文件的路径（相对项目根）
        file_path: String,
    },

    // ── DetailSample ──
    CreateSample {
        id: String,
        novel_id: String,
        title: String,
        content: String,
    },
    ListSamples {
        novel_id: String,
    },
    DeleteSample {
        id: String,
    },

    // ── Patch ──
    PatchOutlineChapter {
        chapter_id: String,
        field: String,  // "content" | "hook"
        old_text: String,
        new_text: String,
    },
    PatchTextChapter {
        chapter_id: String,
        old_text: String,
        new_text: String,
    },
}
