use crate::backends::types::SearchResult;
use crate::config::Settings;
use crate::error::{NetRailError, NetRailResult};
use crate::crypto::{decrypt_text, encrypt_text, ensure_encryption_key, encryption_active};
use chrono::Utc;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static ENCRYPTION_DEGRADED: AtomicBool = AtomicBool::new(false);

pub const ENCRYPTION_DEGRADED_MESSAGE: &str =
    "System keyring unavailable (common on WSL, i3, or headless). History is stored unencrypted this session.";

pub fn encryption_degraded() -> bool {
    ENCRYPTION_DEGRADED.load(Ordering::Relaxed)
}

pub fn encryption_degraded_message() -> &'static str {
    ENCRYPTION_DEGRADED_MESSAGE
}

fn mark_encryption_degraded() {
    ENCRYPTION_DEGRADED.store(true, Ordering::Relaxed);
}

static STORE: OnceCell<Mutex<Option<HistoryStore>>> = OnceCell::new();

const SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    query_text_enc BLOB NOT NULL,
    mode TEXT NOT NULL,
    backends_used TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query_id INTEGER NOT NULL,
    url TEXT NOT NULL,
    url_norm TEXT NOT NULL,
    title_enc BLOB NOT NULL,
    snippet_enc BLOB,
    source_backend TEXT NOT NULL,
    FOREIGN KEY (query_id) REFERENCES queries(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_results_url_norm ON results(url_norm);
CREATE INDEX IF NOT EXISTS idx_results_query_id ON results(query_id);

CREATE TABLE IF NOT EXISTS visits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    result_id INTEGER,
    url TEXT NOT NULL,
    url_norm TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    browser_id TEXT,
    private_mode INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (result_id) REFERENCES results(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_visits_url_norm ON visits(url_norm);

CREATE TABLE IF NOT EXISTS collections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS collection_items (
    collection_id INTEGER NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    notes TEXT,
    saved_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (collection_id, url),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE VIRTUAL TABLE IF NOT EXISTS queries_fts USING fts5(
    query_text,
    content='',
    tokenize='porter unicode61'
);
"#;

pub fn db_path() -> PathBuf {
    env::var("NETRAIL_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("netrail")
                .join("netrail.db")
        })
}

pub fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

pub fn connect() -> Result<Connection, rusqlite::Error> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub struct HistoryStore {
    conn: Connection,
    encrypt: bool,
}

impl HistoryStore {
    pub fn open(settings: &Settings) -> NetRailResult<Self> {
        let want_encrypt = settings.history_encrypt;
        let mut use_encrypt = false;
        if want_encrypt {
            ensure_encryption_key();
            if encryption_active() {
                use_encrypt = true;
            } else {
                tracing::warn!(
                    "keyring unavailable; degrading history encryption for this session"
                );
                mark_encryption_degraded();
            }
        }
        let store = Self {
            conn: connect()?,
            encrypt: use_encrypt,
        };
        let ttl = settings.history_ttl_days;
        let _ = store.purge_expired(ttl);
        Ok(store)
    }

    fn enc(&self, text: &str) -> Vec<u8> {
        encrypt_text(text, self.encrypt)
    }

    fn dec(&self, blob: Option<&[u8]>) -> String {
        decrypt_text(blob.unwrap_or_default(), self.encrypt)
    }

    pub fn purge_expired(&self, ttl_days: u32) -> NetRailResult<usize> {
        if ttl_days == 0 {
            return Ok(0);
        }
        let offset = format!("-{ttl_days} days");
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM queries WHERE timestamp < datetime('now', ?1)")?;
        let ids: Vec<i64> = stmt
            .query_map(params![offset], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        for id in &ids {
            self.conn
                .execute("DELETE FROM queries_fts WHERE rowid = ?1", params![id])?;
            self.conn
                .execute("DELETE FROM queries WHERE id = ?1", params![id])?;
        }
        Ok(ids.len())
    }

    pub fn record_search(
        &self,
        query: &str,
        mode: &str,
        backends_used: &[String],
        results: &[SearchResult],
    ) -> NetRailResult<(i64, HashMap<String, i64>)> {
        let backends_json = serde_json::to_string(backends_used)?;
        self.conn
            .execute(
                "INSERT INTO queries (query_text_enc, mode, backends_used) VALUES (?1, ?2, ?3)",
                params![self.enc(query), mode, backends_json],
            )?;
        let query_id = self.conn.last_insert_rowid();

        self.conn
            .execute(
                "INSERT INTO queries_fts(rowid, query_text) VALUES (?1, ?2)",
                params![query_id, query],
            )?;

        let mut url_to_result_id = HashMap::new();
        for item in results {
            let url_norm = normalize_url(&item.url);
            self.conn
                .execute(
                    "INSERT INTO results (query_id, url, url_norm, title_enc, snippet_enc, source_backend)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        query_id,
                        item.url,
                        url_norm,
                        self.enc(&item.title),
                        if item.snippet.is_empty() {
                            None::<Vec<u8>>
                        } else {
                            Some(self.enc(&item.snippet))
                        },
                        item.backend,
                    ],
                )?;
            url_to_result_id.insert(item.url.clone(), self.conn.last_insert_rowid());
        }
        Ok((query_id, url_to_result_id))
    }

    pub fn get_visit_metadata(
        &self,
        urls: &[String],
    ) -> NetRailResult<HashMap<String, serde_json::Value>> {
        if urls.is_empty() {
            return Ok(HashMap::new());
        }
        let norms: Vec<String> = urls.iter().map(|u| normalize_url(u)).collect();
        let placeholders = norms.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT url_norm, MAX(timestamp) AS last_visited, COUNT(*) AS visit_count
             FROM visits WHERE url_norm IN ({placeholders}) GROUP BY url_norm"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = norms
            .iter()
            .map(|n| n as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;

        let mut norm_to_meta = HashMap::new();
        for row in rows.flatten() {
            norm_to_meta.insert(
                row.0,
                serde_json::json!({
                    "last_visited": row.1,
                    "visit_count": row.2,
                }),
            );
        }

        let mut output = HashMap::new();
        for url in urls {
            if let Some(meta) = norm_to_meta.get(&normalize_url(url)) {
                output.insert(url.clone(), meta.clone());
            }
        }
        Ok(output)
    }

    pub fn record_visit(
        &self,
        url: &str,
        result_id: Option<i64>,
        browser_id: Option<&str>,
        private_mode: bool,
    ) -> NetRailResult<()> {
        let url_norm = normalize_url(url);
        self.conn
            .execute(
                "INSERT INTO visits (result_id, url, url_norm, browser_id, private_mode)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    result_id,
                    url,
                    url_norm,
                    browser_id,
                    i64::from(private_mode)
                ],
            )?;
        Ok(())
    }

    pub fn list_history(
        &self,
        q: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> NetRailResult<serde_json::Value> {
        let rows: Vec<(i64, String, Vec<u8>, String, String, i64)> = if let Some(fts_q) = q {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT q.id, q.timestamp, q.query_text_enc, q.mode, q.backends_used,
                            (SELECT COUNT(*) FROM results r WHERE r.query_id = q.id) AS result_count
                     FROM queries_fts fts
                     JOIN queries q ON q.id = fts.rowid
                     WHERE fts.query_text MATCH ?1
                     ORDER BY q.timestamp DESC
                     LIMIT ?2 OFFSET ?3",
                )?;
            let mapped = stmt
                .query_map(params![fts_q, limit, offset], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })?;
            mapped.filter_map(Result::ok).collect()
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT q.id, q.timestamp, q.query_text_enc, q.mode, q.backends_used,
                            (SELECT COUNT(*) FROM results r WHERE r.query_id = q.id) AS result_count
                     FROM queries q
                     ORDER BY q.timestamp DESC
                     LIMIT ?1 OFFSET ?2",
                )?;
            let mapped = stmt
                .query_map(params![limit, offset], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })?;
            mapped.filter_map(Result::ok).collect()
        };

        let mut items = Vec::new();
        for (id, timestamp, query_enc, mode, backends_used, result_count) in rows {
            let backends: Vec<String> =
                serde_json::from_str(&backends_used).unwrap_or_default();
            items.push(serde_json::json!({
                "id": id,
                "timestamp": timestamp,
                "query": self.dec(Some(&query_enc)),
                "mode": mode,
                "backends_used": backends,
                "result_count": result_count,
            }));
        }

        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM queries", [], |row| row.get(0))?;

        Ok(serde_json::json!({
            "items": items,
            "total": total,
            "limit": limit,
            "offset": offset,
        }))
    }

    pub fn delete_history_entry(&self, query_id: i64) -> NetRailResult<bool> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM queries WHERE id = ?1",
                params![query_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Ok(false);
        }
        self.conn
            .execute("DELETE FROM queries_fts WHERE rowid = ?1", params![query_id])?;
        self.conn
            .execute("DELETE FROM queries WHERE id = ?1", params![query_id])?;
        Ok(true)
    }

    pub fn purge_all_history(&self) -> NetRailResult<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM queries", [], |row| row.get(0))?;
        self.conn
            .execute("DELETE FROM queries_fts", [])?;
        self.conn
            .execute("DELETE FROM queries", [])?;
        Ok(count)
    }

    pub fn list_collections(&self) -> NetRailResult<Vec<serde_json::Value>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.id, c.name, c.created_at,
                        (SELECT COUNT(*) FROM collection_items ci WHERE ci.collection_id = c.id) AS item_count
                 FROM collections c
                 ORDER BY c.name COLLATE NOCASE",
            )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "created_at": row.get::<_, String>(2)?,
                    "item_count": row.get::<_, i64>(3)?,
                }))
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn create_collection(&self, name: &str) -> NetRailResult<serde_json::Value> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NetRailError::MissingField {
                code: "COLLECTION_NAME_REQUIRED",
                field: "name".into(),
            });
        }
        self.conn
            .execute("INSERT INTO collections (name) VALUES (?1)", params![name])
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    NetRailError::InvalidConfig {
                        code: "COLLECTION_EXISTS",
                        message: format!("Collection '{name}' already exists."),
                    }
                } else {
                    NetRailError::from(e)
                }
            })?;
        let id = self.conn.last_insert_rowid();
        Ok(serde_json::json!({
            "id": id,
            "name": name,
            "created_at": Utc::now().to_rfc3339(),
            "item_count": 0,
        }))
    }

    pub fn add_collection_item(
        &self,
        collection_id: i64,
        url: &str,
        title: &str,
        notes: Option<&str>,
    ) -> NetRailResult<serde_json::Value> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM collections WHERE id = ?1",
                params![collection_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(NetRailError::NotFound {
                code: "COLLECTION_NOT_FOUND",
                entity: format!("collection {collection_id}"),
            });
        }

        self.conn
            .execute(
                "INSERT INTO collection_items (collection_id, url, title, notes)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(collection_id, url) DO UPDATE SET
                     title = excluded.title,
                     notes = COALESCE(excluded.notes, collection_items.notes),
                     saved_at = datetime('now')",
                params![collection_id, url, title, notes],
            )?;

        Ok(serde_json::json!({
            "collection_id": collection_id,
            "url": url,
            "title": title,
            "notes": notes,
        }))
    }

    pub fn export_collection(&self, collection_id: i64, fmt: &str) -> NetRailResult<String> {
        let collection: Option<(i64, String, String)> = self
            .conn
            .query_row(
                "SELECT id, name, created_at FROM collections WHERE id = ?1",
                params![collection_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((id, name, created_at)) = collection else {
            return Err(NetRailError::NotFound {
                code: "COLLECTION_NOT_FOUND",
                entity: format!("collection {collection_id}"),
            });
        };

        let mut stmt = self
            .conn
            .prepare(
                "SELECT url, title, notes, saved_at
                 FROM collection_items
                 WHERE collection_id = ?1
                 ORDER BY saved_at DESC",
            )?;
        let items: Vec<serde_json::Value> = stmt
            .query_map(params![collection_id], |row| {
                Ok(serde_json::json!({
                    "url": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "notes": row.get::<_, Option<String>>(2)?,
                    "saved_at": row.get::<_, String>(3)?,
                }))
            })?
            .filter_map(Result::ok)
            .collect();

        if fmt == "csv" {
            let mut out = String::from("url,title,notes,saved_at\n");
            for item in &items {
                let url = item["url"].as_str().unwrap_or("");
                let title = item["title"].as_str().unwrap_or("");
                let notes = item["notes"].as_str().unwrap_or("");
                let saved_at = item["saved_at"].as_str().unwrap_or("");
                out.push_str(&format!(
                    "{},{},{},{}\n",
                    csv_escape(url),
                    csv_escape(title),
                    csv_escape(notes),
                    csv_escape(saved_at)
                ));
            }
            return Ok(out);
        }

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "collection": {
                "id": id,
                "name": name,
                "created_at": created_at,
            },
            "items": items,
            "exported_at": Utc::now().to_rfc3339(),
        }))?)
    }

    pub fn stats(&self) -> serde_json::Value {
        let queries: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM queries", [], |row| row.get(0))
            .unwrap_or(0);
        let visits: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM visits", [], |row| row.get(0))
            .unwrap_or(0);
        let collections: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))
            .unwrap_or(0);
        serde_json::json!({
            "queries": queries,
            "visits": visits,
            "collections": collections,
        })
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(['"', ',', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn get_store(settings: &Settings) -> Option<HistoryStore> {
    if !settings.history_enabled {
        return None;
    }
    HistoryStore::open(settings).ok()
}

pub fn init_history_on_startup(settings: &Settings) {
    let cell = STORE.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock();
    if guard.is_none() {
        *guard = get_store(settings);
    }
}

pub fn with_store<F, T>(settings: &Settings, f: F) -> Option<T>
where
    F: FnOnce(&HistoryStore) -> T,
{
    get_store(settings).map(|store| f(&store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::types::SearchResult;
    use fernet::Fernet;
    use tempfile::TempDir;

    fn temp_store(dir: &TempDir, key: &str) -> HistoryStore {
        std::env::set_var("NETRAIL_DB_KEY", key);
        std::env::set_var(
            "NETRAIL_DB_PATH",
            dir.path().join("netrail.db").to_string_lossy().as_ref(),
        );
        HistoryStore::open(&Settings {
            history_encrypt: true,
            ..Settings::default()
        })
        .expect("open store")
    }

    #[test]
    #[serial_test::serial]
    fn decrypts_python_fernet_blob() {
        let dir = TempDir::new().unwrap();
        let key = Fernet::generate_key();
        let fernet = Fernet::new(&key).unwrap();
        let encrypted = fernet.encrypt(b"battery regulations EU");

        std::env::set_var("NETRAIL_DB_KEY", &key);
        std::env::set_var(
            "NETRAIL_DB_PATH",
            dir.path().join("netrail.db").to_string_lossy().as_ref(),
        );

        let conn = connect().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute(
            "INSERT INTO queries (query_text_enc, mode, backends_used) VALUES (?1, 'web', '[]')",
            params![encrypted.into_bytes()],
        )
        .unwrap();
        drop(conn);

        let store = HistoryStore::open(&Settings {
            history_encrypt: true,
            ..Settings::default()
        })
        .unwrap();
        let listed = store.list_history(None, 10, 0).unwrap();
        assert_eq!(listed["items"][0]["query"], "battery regulations EU");
        std::env::remove_var("NETRAIL_DB_KEY");
        std::env::remove_var("NETRAIL_DB_PATH");
    }

    #[test]
    #[serial_test::serial]
    fn decrypts_python_generated_database() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let key = Fernet::generate_key();
        let db_path = dir.path().join("netrail.db");

        let script = r#"
from cryptography.fernet import Fernet
import os, sqlite3
key = os.environ["NETRAIL_DB_KEY"].encode()
f = Fernet(key)
enc = f.encrypt(b"python tutorial")
conn = sqlite3.connect(os.environ["NETRAIL_DB_PATH"])
conn.executescript('''
PRAGMA foreign_keys = ON;
CREATE TABLE queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    query_text_enc BLOB NOT NULL,
    mode TEXT NOT NULL,
    backends_used TEXT NOT NULL
);
CREATE VIRTUAL TABLE queries_fts USING fts5(query_text, content='', tokenize='porter unicode61');
''')
conn.execute(
    "INSERT INTO queries (query_text_enc, mode, backends_used) VALUES (?, 'web', '[\"ddgs\"]')",
    (enc,),
)
conn.execute("INSERT INTO queries_fts(rowid, query_text) VALUES (1, 'python tutorial')")
conn.commit()
"#;

        let output = Command::new("python3")
            .env("NETRAIL_DB_KEY", &key)
            .env("NETRAIL_DB_PATH", db_path.to_string_lossy().to_string())
            .args(["-c", script])
            .output();

        if output.as_ref().map(|o| o.status.success()).unwrap_or(false) {
            std::env::set_var("NETRAIL_DB_KEY", &key);
            std::env::set_var(
                "NETRAIL_DB_PATH",
                db_path.to_string_lossy().as_ref(),
            );
            let store = HistoryStore::open(&Settings {
                history_encrypt: true,
                ..Settings::default()
            })
            .unwrap();
            let listed = store.list_history(None, 10, 0).unwrap();
            assert_eq!(listed["items"][0]["query"], "python tutorial");
            std::env::remove_var("NETRAIL_DB_KEY");
            std::env::remove_var("NETRAIL_DB_PATH");
        }
    }

    #[test]
    #[serial_test::serial]
    fn record_search_roundtrip() {
        let dir = TempDir::new().unwrap();
        let key = Fernet::generate_key();
        let store = temp_store(&dir, &key);
        let results = vec![
            SearchResult {
                title: "A".into(),
                url: "https://example.com/a".into(),
                snippet: "sa".into(),
                image: None,
                source: String::new(),
                backend: "ddgs".into(),
                provenance: String::new(),
            },
            SearchResult {
                title: "B".into(),
                url: "https://example.com/b".into(),
                snippet: "sb".into(),
                image: None,
                source: String::new(),
                backend: "ddgs".into(),
                provenance: String::new(),
            },
        ];
        let (query_id, url_map) = store
            .record_search("python tutorial", "web", &["ddgs".into()], &results)
            .unwrap();
        assert!(query_id > 0);
        assert_eq!(url_map.len(), 2);
        let listed = store.list_history(None, 10, 0).unwrap();
        assert_eq!(listed["items"][0]["query"], "python tutorial");
        std::env::remove_var("NETRAIL_DB_KEY");
        std::env::remove_var("NETRAIL_DB_PATH");
    }
}