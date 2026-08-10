from __future__ import annotations

import os
import sqlite3
from pathlib import Path

def _data_dir() -> Path:
    # Resolved per call (not at import): tests and processes may change $HOME
    # after import (config.py and browsers.py follow the same convention).
    return Path.home() / ".local" / "share" / "netrail"


def db_path() -> Path:
    return Path(os.environ.get("NETRAIL_DB_PATH", str(_data_dir() / "netrail.db")))

SCHEMA_SQL = """
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
"""


SCHEMA_VERSION = 1


def normalize_url(url: str) -> str:
    return url.strip().rstrip("/").lower()


def _migrate(conn: sqlite3.Connection) -> None:
    """Version schema via PRAGMA user_version (Rust parity).

    Version 0 (fresh DB or any pre-migration database) applies the full
    idempotent schema and stamps 1. Future schema changes append
    `if version < N: ...; conn.execute("PRAGMA user_version = N")`.
    """
    version = conn.execute("PRAGMA user_version").fetchone()[0]
    if version < SCHEMA_VERSION:
        conn.executescript(SCHEMA_SQL)
        conn.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")


def connect() -> sqlite3.Connection:
    path = db_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path, check_same_thread=False, timeout=5)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA busy_timeout = 5000")
    conn.execute("PRAGMA journal_mode = WAL")
    _migrate(conn)
    return conn