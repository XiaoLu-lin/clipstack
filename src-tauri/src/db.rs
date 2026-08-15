use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub text: String,
    pub meta: Option<String>,
    pub thumb: Option<String>,
    pub source: Option<String>,
    pub source_icon: Option<String>,
    pub source_bundle: Option<String>,
    pub pinned: bool,
    pub hash: String,
    pub created_at: i64,
}

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS clips (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    kind        TEXT NOT NULL,
    text        TEXT NOT NULL DEFAULT '',
    meta        TEXT,
    thumb       TEXT,
    source      TEXT,
    source_icon TEXT,
    source_bundle TEXT,
    html        TEXT,
    pinned      INTEGER NOT NULL DEFAULT 0,
    hash        TEXT NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_clips_created ON clips(created_at DESC);
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

pub fn init(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    // 存量库迁移：补充列（已存在则忽略错误）
    let _ = conn.execute_batch("ALTER TABLE clips ADD COLUMN source_icon TEXT");
    let _ = conn.execute_batch("ALTER TABLE clips ADD COLUMN source_bundle TEXT");
    let _ = conn.execute_batch("ALTER TABLE clips ADD COLUMN html TEXT");
    Ok(conn)
}

fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<ClipItem> {
    Ok(ClipItem {
        id: row.get("id")?,
        kind: row.get("kind")?,
        text: row.get("text")?,
        meta: row.get("meta")?,
        thumb: row.get("thumb")?,
        source: row.get("source")?,
        source_icon: row.get("source_icon").ok(),
        source_bundle: row.get("source_bundle").ok(),
        pinned: row.get::<_, i64>("pinned")? != 0,
        hash: row.get("hash")?,
        created_at: row.get("created_at")?,
    })
}

/// 插入新记录；若 hash 已存在则只刷新时间（提到最前）
pub fn upsert_clip(
    conn: &Connection,
    kind: &str,
    text: &str,
    meta: Option<&str>,
    thumb: Option<&str>,
    source: Option<&str>,
    source_icon: Option<&str>,
    source_bundle: Option<&str>,
    html: Option<&str>,
    hash: &str,
    now: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO clips (kind, text, meta, thumb, source, source_icon, source_bundle, html, pinned, hash, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10)
         ON CONFLICT(hash) DO UPDATE SET created_at = excluded.created_at",
        params![kind, text, meta, thumb, source, source_icon, source_bundle, html, hash, now],
    )?;
    Ok(())
}

/// 仅取 HTML 字段（富文本粘贴用；列表查询不返回它以保持轻量）
pub fn get_html(conn: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT html FROM clips WHERE id = ?1", params![id], |r| r.get(0))
}

pub fn list(conn: &Connection, query: Option<&str>, limit: i64) -> rusqlite::Result<Vec<ClipItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, text, meta, thumb, source, source_icon, source_bundle, pinned, hash, created_at
         FROM clips
         WHERE (?1 IS NULL OR text LIKE '%' || ?1 || '%')
         ORDER BY pinned DESC, created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![query, limit], row_to_item)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<ClipItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, text, meta, thumb, source, source_icon, source_bundle, pinned, hash, created_at FROM clips WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_item)?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE clips SET pinned = ?2 WHERE id = ?1",
        params![id, pinned as i64],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
    Ok(())
}

/// 清空全部未置顶记录，返回被删除记录的 hash 列表（用于清理图片文件）
pub fn clear_unpinned(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT hash FROM clips WHERE pinned = 0")?;
    let hashes: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    conn.execute("DELETE FROM clips WHERE pinned = 0", [])?;
    Ok(hashes)
}

pub fn prune(conn: &Connection, capacity: i64) -> rusqlite::Result<Vec<String>> {
    if capacity <= 0 {
        return Ok(vec![]); // 0 = 无限
    }
    let mut stmt = conn.prepare(
        "SELECT hash FROM clips WHERE pinned = 0 AND id NOT IN (
             SELECT id FROM clips WHERE pinned = 0 ORDER BY created_at DESC LIMIT ?1
         )",
    )?;
    let hashes: Vec<String> = stmt
        .query_map(params![capacity], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    conn.execute(
        "DELETE FROM clips WHERE pinned = 0 AND id NOT IN (
             SELECT id FROM clips WHERE pinned = 0 ORDER BY created_at DESC LIMIT ?1
         )",
        params![capacity],
    )?;
    Ok(hashes)
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
        r.get(0)
    })
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
