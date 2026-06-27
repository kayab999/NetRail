from __future__ import annotations

import os
import sqlite3
from pathlib import Path

DATA_DIR = Path.home() / ".local" / "share" / "netrail"
DB_PATH = Path(os.environ.get("NETRAIL_DB_PATH", str(DATA_DIR / "netrail.db")))

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


def normalize_url(url: str) -> str:
    return url.strip().rstrip("/").lower()


def connect() -> sqlite3.Connection:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(DB_PATH, check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.executescript(SCHEMA_SQL)
    return conn