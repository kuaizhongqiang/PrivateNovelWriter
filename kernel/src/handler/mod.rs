use std::path::PathBuf;

use crate::command::data::DataCommand;
use crate::db::crud;
use crate::models::*;
use crate::storage;
use rusqlite::Connection;

#[derive(Debug, Clone, serde::Serialize)]
pub enum Output {
    Novel(Novel),
    NovelList(Vec<Novel>),
    Setting(NovelSetting),
    Character(Character),
    CharacterList(Vec<Character>),
    Plugin(Plugin),
    OutlinePhaseList(Vec<OutlinePhase>),
    OutlineChapter(OutlineChapter),
    OutlineChapterList(Vec<OutlineChapter>),
    OutlineTree(Vec<OutlineTree>),
    TextPhaseList(Vec<TextPhase>),
    TextChapter(TextChapter),
    TextChapterList(Vec<TextChapter>),
    TextContent(String),
    TextChapterWithContent {
        chapter: TextChapter,
        content: String,
    },
    SampleList(Vec<DetailSample>),
    Status(String),
    StatusJson(serde_json::Value),
    Ok,
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] storage::StorageError),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Patch failed: old_text not found in {0}")]
    PatchFailed(String),
}

pub struct Handler {
    pub conn: Connection,
    pub project_path: PathBuf,
}

impl Handler {
    pub fn new(conn: Connection, project_path: PathBuf) -> Self {
        Self { conn, project_path }
    }

    pub fn execute(&self, cmd: DataCommand) -> Result<Output, HandlerError> {
        match cmd {
            // ── Novel ──
            DataCommand::CreateNovel {
                id,
                name,
                total_char,
                chapter_char,
                sensitivity,
            } => {
                let now = chrono::Utc::now().to_rfc3339();
                let novel = Novel {
                    id,
                    name,
                    created: now.clone(),
                    modified: now,
                    active: true,
                    total_char,
                    chapter_char,
                    sensitivity: Sensitivity::from_i32(sensitivity),
                };
                crud::create_novel(&self.conn, &novel)?;
                Ok(Output::Novel(novel))
            }
            DataCommand::GetNovel { id } => {
                let novel = crud::get_novel(&self.conn, &id)?
                    .ok_or_else(|| HandlerError::NotFound(format!("Novel {}", id)))?;
                Ok(Output::Novel(novel))
            }
            DataCommand::ListNovels => {
                let list = crud::list_novels(&self.conn)?;
                Ok(Output::NovelList(list))
            }
            DataCommand::UpdateNovel {
                id,
                name,
                total_char,
                chapter_char,
                sensitivity,
            } => {
                crud::update_novel(
                    &self.conn,
                    &id,
                    name.as_deref(),
                    total_char,
                    chapter_char,
                    sensitivity,
                )?;
                Ok(Output::Ok)
            }

            // ── Setting ──
            DataCommand::WriteSetting {
                novel_id,
                title,
                inspiration,
                description,
                novel_type,
                tags,
            } => {
                let s = NovelSetting {
                    novel_id,
                    title,
                    inspiration,
                    description,
                    novel_type: NovelType::from_i32(novel_type),
                    tags,
                };
                crud::upsert_setting(&self.conn, &s)?;
                Ok(Output::Setting(s))
            }
            DataCommand::GetSetting { novel_id } => {
                let s = crud::get_setting(&self.conn, &novel_id)?.ok_or_else(|| {
                    HandlerError::NotFound(format!("Setting for novel {}", novel_id))
                })?;
                Ok(Output::Setting(s))
            }

            // ── Character ──
            DataCommand::CreateCharacter {
                id,
                novel_id,
                name,
                char_type,
                age,
                relationship,
            } => {
                let c = Character {
                    id,
                    novel_id,
                    name,
                    char_type: CharacterType::from_i32(char_type),
                    age,
                    relationship,
                };
                crud::create_character(&self.conn, &c)?;
                Ok(Output::Character(c))
            }
            DataCommand::GetCharacter { id } => {
                let c = crud::get_character(&self.conn, &id)?
                    .ok_or_else(|| HandlerError::NotFound(format!("Character {}", id)))?;
                Ok(Output::Character(c))
            }
            DataCommand::ListCharacters { novel_id } => {
                let list = crud::list_characters(&self.conn, &novel_id)?;
                Ok(Output::CharacterList(list))
            }
            DataCommand::UpdateCharacter {
                id,
                novel_id,
                name,
                char_type,
                age,
                relationship,
            } => {
                let c = Character {
                    id,
                    novel_id,
                    name,
                    char_type: CharacterType::from_i32(char_type),
                    age,
                    relationship,
                };
                crud::update_character(&self.conn, &c)?;
                Ok(Output::Ok)
            }
            DataCommand::DeleteCharacter { id } => {
                crud::delete_character(&self.conn, &id)?;
                Ok(Output::Ok)
            }

            // ── Plugin ──
            DataCommand::WritePlugin {
                novel_id,
                name,
                plugin_type,
                description,
                benefit,
                cost,
            } => {
                let p = Plugin {
                    novel_id,
                    name,
                    plugin_type: PluginType::from_i32(plugin_type),
                    description,
                    benefit,
                    cost,
                };
                crud::upsert_plugin(&self.conn, &p)?;
                Ok(Output::Plugin(p))
            }
            DataCommand::GetPlugin { novel_id } => {
                let p = crud::get_plugin(&self.conn, &novel_id)?;
                Ok(match p {
                    Some(p) => Output::Plugin(p),
                    None => {
                        return Err(HandlerError::NotFound(format!(
                            "Plugin for novel {}",
                            novel_id
                        )))
                    }
                })
            }
            DataCommand::DeletePlugin { novel_id } => {
                crud::delete_plugin(&self.conn, &novel_id)?;
                Ok(Output::Ok)
            }

            // ── Outline Phase ──
            DataCommand::CreateOutlinePhase {
                id,
                novel_id,
                sort,
                name,
                description,
            } => {
                let p = OutlinePhase {
                    id,
                    novel_id,
                    sort,
                    name,
                    description,
                };
                crud::create_outline_phase(&self.conn, &p)?;
                Ok(Output::Status(format!("Created phase: {}", p.name)))
            }
            DataCommand::ListOutlinePhases { novel_id } => {
                let list = crud::list_outline_phases(&self.conn, &novel_id)?;
                Ok(Output::OutlinePhaseList(list))
            }
            DataCommand::DeleteOutlinePhase { phase_id } => {
                crud::delete_outline_phase(&self.conn, &phase_id)?;
                Ok(Output::Ok)
            }
            DataCommand::UpdateOutlinePhase {
                id,
                novel_id,
                sort,
                name,
                description,
            } => {
                let p = OutlinePhase {
                    id,
                    novel_id,
                    sort,
                    name,
                    description,
                };
                crud::update_outline_phase(&self.conn, &p)?;
                Ok(Output::Ok)
            }

            // ── Outline Chapter ──
            DataCommand::CreateOutlineChapter {
                id,
                phase_id,
                sort,
                chapter_name,
                content,
                hook,
            } => {
                let c = OutlineChapter {
                    id,
                    phase_id,
                    sort,
                    chapter_name,
                    content,
                    hook,
                    text_chapter_id: None,
                };
                crud::create_outline_chapter(&self.conn, &c)?;
                Ok(Output::OutlineChapter(c))
            }
            DataCommand::ListOutlineChapters { phase_id } => {
                let list = crud::list_outline_chapters(&self.conn, &phase_id)?;
                Ok(Output::OutlineChapterList(list))
            }
            DataCommand::GetOutlineChapter { id } => {
                let c = crud::get_outline_chapter(&self.conn, &id)?
                    .ok_or_else(|| HandlerError::NotFound(format!("Outline chapter {}", id)))?;
                Ok(Output::OutlineChapter(c))
            }
            DataCommand::UpdateOutlineChapter {
                id,
                phase_id,
                sort,
                chapter_name,
                content,
                hook,
                text_chapter_id,
            } => {
                let c = OutlineChapter {
                    id,
                    phase_id,
                    sort,
                    chapter_name,
                    content,
                    hook,
                    text_chapter_id,
                };
                crud::update_outline_chapter(&self.conn, &c)?;
                Ok(Output::Ok)
            }
            DataCommand::DeleteOutlineChapter { id } => {
                crud::delete_outline_chapter(&self.conn, &id)?;
                Ok(Output::Ok)
            }

            // ── Outline Tree ──
            DataCommand::GetOutlineTree { novel_id, phase_id } => {
                let phases = if let Some(pid) = phase_id {
                    let all = crud::list_outline_phases(&self.conn, &novel_id)?;
                    all.into_iter().filter(|p| p.id == pid).collect()
                } else {
                    crud::list_outline_phases(&self.conn, &novel_id)?
                };
                let mut tree = Vec::new();
                for phase in phases {
                    let chapters = crud::list_outline_chapters(&self.conn, &phase.id)?;
                    tree.push(OutlineTree { phase, chapters });
                }
                Ok(Output::OutlineTree(tree))
            }

            // ── Text Phase ──
            DataCommand::CreateTextPhase {
                id,
                novel_id,
                sort,
                name,
            } => {
                let p = TextPhase {
                    id,
                    novel_id,
                    sort,
                    name,
                };
                crud::create_text_phase(&self.conn, &p)?;
                Ok(Output::Status(format!("Created text phase: {}", p.name)))
            }
            DataCommand::ListTextPhases { novel_id } => {
                let list = crud::list_text_phases(&self.conn, &novel_id)?;
                Ok(Output::TextPhaseList(list))
            }
            DataCommand::DeleteTextPhase { phase_id } => {
                crud::delete_text_phase(&self.conn, &phase_id)?;
                Ok(Output::Ok)
            }

            // ── Text Chapter ──
            DataCommand::CreateTextChapter {
                id,
                phase_id,
                sort,
                name,
                file_path,
            } => {
                let c = TextChapter {
                    id,
                    phase_id,
                    sort,
                    name,
                    file_path,
                    word_count: 0,
                };
                crud::create_text_chapter(&self.conn, &c)?;
                Ok(Output::Status(format!("Created text chapter: {}", c.name)))
            }
            DataCommand::GetTextChapter { id } => {
                let c = crud::get_text_chapter(&self.conn, &id)?
                    .ok_or_else(|| HandlerError::NotFound(format!("Text chapter {}", id)))?;
                Ok(Output::TextChapter(c))
            }
            DataCommand::ListTextChapters { phase_id } => {
                let list = crud::list_text_chapters(&self.conn, &phase_id)?;
                Ok(Output::TextChapterList(list))
            }
            DataCommand::UpdateTextChapter {
                id,
                name,
                word_count,
            } => {
                let mut c = crud::get_text_chapter(&self.conn, &id)?
                    .ok_or_else(|| HandlerError::NotFound(format!("Text chapter {}", id)))?;
                c.name = name;
                c.word_count = word_count;
                crud::update_text_chapter(&self.conn, &c)?;
                Ok(Output::Ok)
            }
            DataCommand::DeleteTextChapter { id, file_path } => {
                // 先删 DB 记录，再删文件（保证一致性）
                crud::delete_text_chapter(&self.conn, &id)?;
                let full_path = self.project_path.join(&file_path);
                storage::delete_file(&full_path)?;
                Ok(Output::Ok)
            }

            // ── DetailSample ──
            DataCommand::CreateSample {
                id,
                novel_id,
                title,
                content,
            } => {
                let s = DetailSample {
                    id,
                    novel_id,
                    title,
                    content,
                };
                crud::create_sample(&self.conn, &s)?;
                Ok(Output::Status(format!("Created sample: {}", s.title)))
            }
            DataCommand::ListSamples { novel_id } => {
                let list = crud::list_samples(&self.conn, &novel_id)?;
                Ok(Output::SampleList(list))
            }
            DataCommand::DeleteSample { id } => {
                crud::delete_sample(&self.conn, &id)?;
                Ok(Output::Ok)
            }

            // ── Patch Outline Chapter ──
            DataCommand::PatchOutlineChapter {
                chapter_id,
                field,
                old_text,
                new_text,
            } => {
                let mut c =
                    crud::get_outline_chapter(&self.conn, &chapter_id)?.ok_or_else(|| {
                        HandlerError::NotFound(format!("Outline chapter {}", chapter_id))
                    })?;
                let target = match field.as_str() {
                    "content" => &mut c.content,
                    "hook" => &mut c.hook,
                    _ => {
                        return Err(HandlerError::PatchFailed(format!(
                            "unknown field {}",
                            field
                        )))
                    }
                };
                if !target.contains(&old_text) {
                    return Err(HandlerError::PatchFailed(format!(
                        "outline chapter {}",
                        chapter_id
                    )));
                }
                *target = target.replace(&old_text, &new_text);
                crud::update_outline_chapter(&self.conn, &c)?;
                Ok(Output::Ok)
            }

            // ── Patch Text Chapter ──
            DataCommand::PatchTextChapter {
                chapter_id,
                old_text,
                new_text,
            } => {
                let c = crud::get_text_chapter(&self.conn, &chapter_id)?.ok_or_else(|| {
                    HandlerError::NotFound(format!("Text chapter {}", chapter_id))
                })?;
                let full_path = self.project_path.join(&c.file_path);
                let mut content = storage::read_text(&full_path)?;
                if !content.contains(&old_text) {
                    return Err(HandlerError::PatchFailed(format!(
                        "text chapter {}",
                        chapter_id
                    )));
                }
                content = content.replace(&old_text, &new_text);
                let wc = storage::count_chars(&content);
                storage::write_text(&full_path, &content)?;
                let mut tc = c;
                tc.word_count = wc;
                crud::update_text_chapter(&self.conn, &tc)?;
                Ok(Output::Ok)
            }
        }
    }
}
