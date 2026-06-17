use rusqlite::{params, Connection, Result};

use crate::models::*;

// ── Novel ──

pub fn create_novel(conn: &Connection, novel: &Novel) -> Result<()> {
    conn.execute(
        "INSERT INTO novel (id, name, created, modified, active, total_char, chapter_char, sensitivity)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            novel.id,
            novel.name,
            novel.created,
            novel.modified,
            novel.active as i32,
            novel.total_char,
            novel.chapter_char,
            novel.sensitivity.to_i32(),
        ],
    )?;
    Ok(())
}

pub fn get_novel(conn: &Connection, id: &str) -> Result<Option<Novel>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, created, modified, active, total_char, chapter_char, sensitivity FROM novel WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Novel {
            id: row.get(0)?,
            name: row.get(1)?,
            created: row.get(2)?,
            modified: row.get(3)?,
            active: row.get::<_, i32>(4)? != 0,
            total_char: row.get(5)?,
            chapter_char: row.get(6)?,
            sensitivity: Sensitivity::from_i32(row.get(7)?),
        }))
    } else {
        Ok(None)
    }
}

pub fn list_novels(conn: &Connection) -> Result<Vec<Novel>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, created, modified, active, total_char, chapter_char, sensitivity FROM novel ORDER BY created DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Novel {
            id: row.get(0)?,
            name: row.get(1)?,
            created: row.get(2)?,
            modified: row.get(3)?,
            active: row.get::<_, i32>(4)? != 0,
            total_char: row.get(5)?,
            chapter_char: row.get(6)?,
            sensitivity: Sensitivity::from_i32(row.get(7)?),
        })
    })?;
    rows.collect()
}

pub fn update_novel(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    total_char: Option<i32>,
    chapter_char: Option<i32>,
    sensitivity: Option<i32>,
) -> Result<()> {
    let mut sets = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = name {
        sets.push("name = ?");
        vals.push(Box::new(n.to_string()));
    }
    if let Some(t) = total_char {
        sets.push("total_char = ?");
        vals.push(Box::new(t));
    }
    if let Some(c) = chapter_char {
        sets.push("chapter_char = ?");
        vals.push(Box::new(c));
    }
    if let Some(s) = sensitivity {
        sets.push("sensitivity = ?");
        vals.push(Box::new(s));
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("modified = ?");
    vals.push(Box::new(chrono::Utc::now().to_rfc3339()));

    let sql = format!("UPDATE novel SET {} WHERE id = ?", sets.join(", "));
    vals.push(Box::new(id.to_string()));

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|v| v.as_ref()).collect();
    stmt.execute(params.as_slice())?;
    Ok(())
}

// ── Setting ──

pub fn upsert_setting(conn: &Connection, s: &NovelSetting) -> Result<()> {
    conn.execute(
        "INSERT INTO setting (novel_id, title, inspiration, description, novel_type, tags_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(novel_id) DO UPDATE SET
           title=excluded.title, inspiration=excluded.inspiration,
           description=excluded.description, novel_type=excluded.novel_type,
           tags_json=excluded.tags_json",
        params![
            s.novel_id,
            s.title,
            s.inspiration,
            s.description,
            s.novel_type.to_i32(),
            serde_json::to_string(&s.tags).unwrap_or_default(),
        ],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, novel_id: &str) -> Result<Option<NovelSetting>> {
    let mut stmt = conn.prepare(
        "SELECT novel_id, title, inspiration, description, novel_type, tags_json FROM setting WHERE novel_id = ?1",
    )?;
    let mut rows = stmt.query(params![novel_id])?;
    if let Some(row) = rows.next()? {
        let tags_str: String = row.get(5)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(Some(NovelSetting {
            novel_id: row.get(0)?,
            title: row.get(1)?,
            inspiration: row.get(2)?,
            description: row.get(3)?,
            novel_type: NovelType::from_i32(row.get(4)?),
            tags,
        }))
    } else {
        Ok(None)
    }
}

// ── Character ──

pub fn create_character(conn: &Connection, c: &Character) -> Result<()> {
    conn.execute(
        "INSERT INTO character (id, novel_id, name, char_type, age, relationship)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            c.id,
            c.novel_id,
            c.name,
            c.char_type.to_i32(),
            c.age,
            c.relationship
        ],
    )?;
    Ok(())
}

pub fn get_character(conn: &Connection, id: &str) -> Result<Option<Character>> {
    let mut stmt = conn.prepare(
        "SELECT id, novel_id, name, char_type, age, relationship FROM character WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Character {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            name: row.get(2)?,
            char_type: CharacterType::from_i32(row.get(3)?),
            age: row.get(4)?,
            relationship: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_characters(conn: &Connection, novel_id: &str) -> Result<Vec<Character>> {
    let mut stmt = conn.prepare(
        "SELECT id, novel_id, name, char_type, age, relationship FROM character WHERE novel_id = ?1 ORDER BY char_type, name",
    )?;
    let rows = stmt.query_map(params![novel_id], |row| {
        Ok(Character {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            name: row.get(2)?,
            char_type: CharacterType::from_i32(row.get(3)?),
            age: row.get(4)?,
            relationship: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn update_character(conn: &Connection, c: &Character) -> Result<()> {
    conn.execute(
        "UPDATE character SET name=?1, char_type=?2, age=?3, relationship=?4 WHERE id=?5",
        params![c.name, c.char_type.to_i32(), c.age, c.relationship, c.id],
    )?;
    Ok(())
}

pub fn delete_character(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM character WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Plugin ──

pub fn upsert_plugin(conn: &Connection, p: &Plugin) -> Result<()> {
    conn.execute(
        "INSERT INTO plugin (novel_id, name, plugin_type, description, benefit, cost)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(novel_id) DO UPDATE SET
           name=excluded.name, plugin_type=excluded.plugin_type,
           description=excluded.description, benefit=excluded.benefit, cost=excluded.cost",
        params![
            p.novel_id,
            p.name,
            p.plugin_type.to_i32(),
            p.description,
            p.benefit,
            p.cost
        ],
    )?;
    Ok(())
}

pub fn get_plugin(conn: &Connection, novel_id: &str) -> Result<Option<Plugin>> {
    let mut stmt = conn.prepare(
        "SELECT novel_id, name, plugin_type, description, benefit, cost FROM plugin WHERE novel_id = ?1",
    )?;
    let mut rows = stmt.query(params![novel_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Plugin {
            novel_id: row.get(0)?,
            name: row.get(1)?,
            plugin_type: PluginType::from_i32(row.get(2)?),
            description: row.get(3)?,
            benefit: row.get(4)?,
            cost: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete_plugin(conn: &Connection, novel_id: &str) -> Result<()> {
    conn.execute("DELETE FROM plugin WHERE novel_id = ?1", params![novel_id])?;
    Ok(())
}

// ── Outline Phase ──

pub fn create_outline_phase(conn: &Connection, p: &OutlinePhase) -> Result<()> {
    conn.execute(
        "INSERT INTO outline_phase (id, novel_id, sort, name, description)
         VALUES (?1, ?2, COALESCE((SELECT MAX(sort) FROM outline_phase WHERE novel_id = ?2), -1) + 1, ?3, ?4)",
        params![p.id, p.novel_id, p.name, p.description],
    )?;
    Ok(())
}

pub fn list_outline_phases(conn: &Connection, novel_id: &str) -> Result<Vec<OutlinePhase>> {
    let mut stmt = conn.prepare(
        "SELECT id, novel_id, sort, name, description FROM outline_phase WHERE novel_id = ?1 ORDER BY sort",
    )?;
    let rows = stmt.query_map(params![novel_id], |row| {
        Ok(OutlinePhase {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            sort: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn delete_outline_phase(conn: &Connection, phase_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM outline_chapter WHERE phase_id = ?1",
        params![phase_id],
    )?;
    conn.execute("DELETE FROM outline_phase WHERE id = ?1", params![phase_id])?;
    Ok(())
}

pub fn update_outline_phase(conn: &Connection, p: &OutlinePhase) -> Result<()> {
    conn.execute(
        "UPDATE outline_phase SET name=?1, description=?2, sort=?3 WHERE id=?4",
        params![p.name, p.description, p.sort, p.id],
    )?;
    Ok(())
}

// ── Outline Chapter ──

pub fn create_outline_chapter(conn: &Connection, c: &OutlineChapter) -> Result<()> {
    conn.execute(
        "INSERT INTO outline_chapter (id, phase_id, sort, chapter_name, content, hook, text_chapter_id)
         VALUES (?1, ?2, COALESCE((SELECT MAX(sort) FROM outline_chapter WHERE phase_id = ?2), -1) + 1, ?3, ?4, ?5, ?6)",
        params![c.id, c.phase_id, c.chapter_name, c.content, c.hook, c.text_chapter_id],
    )?;
    Ok(())
}

pub fn list_outline_chapters(conn: &Connection, phase_id: &str) -> Result<Vec<OutlineChapter>> {
    let mut stmt = conn.prepare(
        "SELECT id, phase_id, sort, chapter_name, content, hook, text_chapter_id
         FROM outline_chapter WHERE phase_id = ?1 ORDER BY sort",
    )?;
    let rows = stmt.query_map(params![phase_id], |row| {
        Ok(OutlineChapter {
            id: row.get(0)?,
            phase_id: row.get(1)?,
            sort: row.get(2)?,
            chapter_name: row.get(3)?,
            content: row.get(4)?,
            hook: row.get(5)?,
            text_chapter_id: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_outline_chapter(conn: &Connection, id: &str) -> Result<Option<OutlineChapter>> {
    let mut stmt = conn.prepare(
        "SELECT id, phase_id, sort, chapter_name, content, hook, text_chapter_id
         FROM outline_chapter WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(OutlineChapter {
            id: row.get(0)?,
            phase_id: row.get(1)?,
            sort: row.get(2)?,
            chapter_name: row.get(3)?,
            content: row.get(4)?,
            hook: row.get(5)?,
            text_chapter_id: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_outline_chapter(conn: &Connection, c: &OutlineChapter) -> Result<()> {
    conn.execute(
        "UPDATE outline_chapter SET chapter_name=?1, content=?2, hook=?3, sort=?4, text_chapter_id=?5 WHERE id=?6",
        params![c.chapter_name, c.content, c.hook, c.sort, c.text_chapter_id, c.id],
    )?;
    Ok(())
}

pub fn delete_outline_chapter(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM outline_chapter WHERE id = ?1", params![id])?;
    Ok(())
}

/// 通过 text_chapter_id 查找关联的 outline_chapter (JOIN, 1 次查询)
pub fn find_outline_chapter_by_text_id(
    conn: &Connection,
    novel_id: &str,
    text_chapter_id: &str,
) -> Result<Option<(OutlinePhase, OutlineChapter)>> {
    let mut stmt = conn.prepare(
        "SELECT oc.id, oc.phase_id, oc.sort, oc.chapter_name, oc.content, oc.hook, oc.text_chapter_id,
                op.id, op.novel_id, op.sort, op.name, op.description
         FROM outline_chapter oc
         JOIN outline_phase op ON oc.phase_id = op.id
         WHERE op.novel_id = ?1 AND oc.text_chapter_id = ?2
         LIMIT 1"
    )?;
    let mut rows = stmt.query(params![novel_id, text_chapter_id])?;
    if let Some(row) = rows.next()? {
        let oc = OutlineChapter {
            id: row.get(0)?,
            phase_id: row.get(1)?,
            sort: row.get(2)?,
            chapter_name: row.get(3)?,
            content: row.get(4)?,
            hook: row.get(5)?,
            text_chapter_id: row.get(6)?,
        };
        let op = OutlinePhase {
            id: row.get(7)?,
            novel_id: row.get(8)?,
            sort: row.get(9)?,
            name: row.get(10)?,
            description: row.get(11)?,
        };
        Ok(Some((op, oc)))
    } else {
        Ok(None)
    }
}

// ── Text Phase ──

pub fn create_text_phase(conn: &Connection, p: &TextPhase) -> Result<()> {
    conn.execute(
        "INSERT INTO text_phase (id, novel_id, sort, name) VALUES (?1, ?2, COALESCE((SELECT MAX(sort) FROM text_phase WHERE novel_id = ?2), -1) + 1, ?3)",
        params![p.id, p.novel_id, p.name],
    )?;
    Ok(())
}

pub fn list_text_phases(conn: &Connection, novel_id: &str) -> Result<Vec<TextPhase>> {
    let mut stmt = conn.prepare(
        "SELECT id, novel_id, sort, name FROM text_phase WHERE novel_id = ?1 ORDER BY sort",
    )?;
    let rows = stmt.query_map(params![novel_id], |row| {
        Ok(TextPhase {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            sort: row.get(2)?,
            name: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn delete_text_phase(conn: &Connection, phase_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM text_chapter WHERE phase_id = ?1",
        params![phase_id],
    )?;
    conn.execute("DELETE FROM text_phase WHERE id = ?1", params![phase_id])?;
    Ok(())
}

// ── Text Chapter ──

pub fn create_text_chapter(conn: &Connection, c: &TextChapter) -> Result<()> {
    conn.execute(
        "INSERT INTO text_chapter (id, phase_id, sort, name, file_path, word_count)
         VALUES (?1, ?2, COALESCE((SELECT MAX(sort) FROM text_chapter WHERE phase_id = ?2), -1) + 1, ?3, ?4, ?5)",
        params![c.id, c.phase_id, c.name, c.file_path, c.word_count],
    )?;
    Ok(())
}

pub fn get_text_chapter(conn: &Connection, id: &str) -> Result<Option<TextChapter>> {
    let mut stmt = conn.prepare(
        "SELECT id, phase_id, sort, name, file_path, word_count FROM text_chapter WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(TextChapter {
            id: row.get(0)?,
            phase_id: row.get(1)?,
            sort: row.get(2)?,
            name: row.get(3)?,
            file_path: row.get(4)?,
            word_count: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_text_chapters(conn: &Connection, phase_id: &str) -> Result<Vec<TextChapter>> {
    let mut stmt = conn.prepare(
        "SELECT id, phase_id, sort, name, file_path, word_count
         FROM text_chapter WHERE phase_id = ?1 ORDER BY sort",
    )?;
    let rows = stmt.query_map(params![phase_id], |row| {
        Ok(TextChapter {
            id: row.get(0)?,
            phase_id: row.get(1)?,
            sort: row.get(2)?,
            name: row.get(3)?,
            file_path: row.get(4)?,
            word_count: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn update_text_chapter(conn: &Connection, c: &TextChapter) -> Result<()> {
    conn.execute(
        "UPDATE text_chapter SET name=?1, word_count=?2 WHERE id=?3",
        params![c.name, c.word_count, c.id],
    )?;
    Ok(())
}

/// 尝试将 ID 解析为正文章节（支持 outline 或 text chapter ID）
/// 如果是 outline chapter ID 且有已关联的正文章节，返回关联的正文章节
pub fn resolve_text_chapter(conn: &Connection, id: &str) -> Result<Option<TextChapter>> {
    // 先直接查正文章节表
    if let Some(tc) = get_text_chapter(conn, id)? {
        return Ok(Some(tc));
    }
    // 再查是否是大纲章节且有已关联的正文章节
    if let Some(oc) = get_outline_chapter(conn, id)? {
        if let Some(tc_id) = oc.text_chapter_id {
            return get_text_chapter(conn, &tc_id);
        }
    }
    Ok(None)
}

pub fn delete_text_chapter(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM text_chapter WHERE id = ?1", params![id])?;
    Ok(())
}

// ── DetailSample ──

pub fn create_sample(conn: &Connection, s: &DetailSample) -> Result<()> {
    conn.execute(
        "INSERT INTO detail_sample (id, novel_id, title, content) VALUES (?1, ?2, ?3, ?4)",
        params![s.id, s.novel_id, s.title, s.content],
    )?;
    Ok(())
}

pub fn list_samples(conn: &Connection, novel_id: &str) -> Result<Vec<DetailSample>> {
    let mut stmt = conn.prepare(
        "SELECT id, novel_id, title, content FROM detail_sample WHERE novel_id = ?1 ORDER BY title",
    )?;
    let rows = stmt.query_map(params![novel_id], |row| {
        Ok(DetailSample {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            title: row.get(2)?,
            content: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn delete_sample(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM detail_sample WHERE id = ?1", params![id])?;
    Ok(())
}
