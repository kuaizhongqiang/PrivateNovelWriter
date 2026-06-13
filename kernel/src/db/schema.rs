pub fn init_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS novel (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            created      TEXT NOT NULL,
            modified     TEXT NOT NULL,
            active       INTEGER NOT NULL DEFAULT 0,
            total_char   INTEGER NOT NULL DEFAULT 0,
            chapter_char INTEGER NOT NULL DEFAULT 2000,
            sensitivity  INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS setting (
            novel_id     TEXT PRIMARY KEY REFERENCES novel(id),
            title        TEXT NOT NULL DEFAULT '',
            inspiration  TEXT NOT NULL DEFAULT '',
            description  TEXT NOT NULL DEFAULT '',
            novel_type   INTEGER NOT NULL DEFAULT 0,
            tags_json    TEXT NOT NULL DEFAULT '[]'
        );

        CREATE TABLE IF NOT EXISTS character (
            id           TEXT PRIMARY KEY,
            novel_id     TEXT NOT NULL REFERENCES novel(id),
            name         TEXT NOT NULL,
            char_type    INTEGER NOT NULL DEFAULT 0,
            age          INTEGER NOT NULL DEFAULT 0,
            relationship TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS plugin (
            novel_id     TEXT PRIMARY KEY REFERENCES novel(id),
            name         TEXT NOT NULL DEFAULT '',
            plugin_type  INTEGER NOT NULL DEFAULT 0,
            description  TEXT NOT NULL DEFAULT '',
            benefit      TEXT NOT NULL DEFAULT '',
            cost         TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS outline_phase (
            id           TEXT PRIMARY KEY,
            novel_id     TEXT NOT NULL REFERENCES novel(id),
            sort         INTEGER NOT NULL DEFAULT 0,
            name         TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS outline_chapter (
            id               TEXT PRIMARY KEY,
            phase_id         TEXT NOT NULL REFERENCES outline_phase(id),
            sort             INTEGER NOT NULL DEFAULT 0,
            chapter_name     TEXT NOT NULL,
            content          TEXT NOT NULL DEFAULT '',
            hook             TEXT NOT NULL DEFAULT '',
            text_chapter_id  TEXT REFERENCES text_chapter(id)
        );

        CREATE TABLE IF NOT EXISTS text_phase (
            id       TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL REFERENCES novel(id),
            sort     INTEGER NOT NULL DEFAULT 0,
            name     TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS text_chapter (
            id         TEXT PRIMARY KEY,
            phase_id   TEXT NOT NULL REFERENCES text_phase(id),
            sort       INTEGER NOT NULL DEFAULT 0,
            name       TEXT NOT NULL DEFAULT '',
            file_path  TEXT NOT NULL DEFAULT '',
            word_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS detail_sample (
            id       TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL REFERENCES novel(id),
            title    TEXT NOT NULL,
            content  TEXT NOT NULL DEFAULT ''
        );
        ",
    )
}
