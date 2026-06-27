from __future__ import annotations

import csv
import io
import json
import sqlite3
from datetime import datetime, timezone
from typing import Any

from netrail.backends.types import SearchResult
from netrail.config import load_settings
from netrail.history.crypto import decrypt_text, encrypt_text, ensure_encryption_key
from netrail.history.db import connect, normalize_url

_store: "HistoryStore | None" = None


class HistoryStore:
    def __init__(self, conn: sqlite3.Connection, *, encrypt: bool) -> None:
        self._conn = conn
        self._encrypt = encrypt

    def _enc(self, text: str) -> bytes:
        return encrypt_text(text, force_plain=not self._encrypt)

    def _dec(self, blob: bytes | None) -> str:
        return decrypt_text(blob, force_plain=not self._encrypt)

    def purge_expired(self, ttl_days: int) -> int:
        if ttl_days <= 0:
            return 0
        cursor = self._conn.execute(
            """
            SELECT id FROM queries
            WHERE timestamp < datetime('now', ?)
            """,
            (f"-{int(ttl_days)} days",),
        )
        ids = [row["id"] for row in cursor.fetchall()]
        for query_id in ids:
            self._conn.execute("DELETE FROM queries_fts WHERE rowid = ?", (query_id,))
            self._conn.execute("DELETE FROM queries WHERE id = ?", (query_id,))
        self._conn.commit()
        return len(ids)

    def record_search(
        self,
        query: str,
        mode: str,
        backends_used: list[str],
        results: list[SearchResult],
    ) -> tuple[int, dict[str, int]]:
        backends_json = json.dumps(backends_used)
        cursor = self._conn.execute(
            """
            INSERT INTO queries (query_text_enc, mode, backends_used)
            VALUES (?, ?, ?)
            """,
            (self._enc(query), mode, backends_json),
        )
        query_id = int(cursor.lastrowid)
        self._conn.execute(
            "INSERT INTO queries_fts(rowid, query_text) VALUES (?, ?)",
            (query_id, query),
        )

        url_to_result_id: dict[str, int] = {}
        for item in results:
            url_norm = normalize_url(item.url)
            row = self._conn.execute(
                """
                INSERT INTO results (query_id, url, url_norm, title_enc, snippet_enc, source_backend)
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (
                    query_id,
                    item.url,
                    url_norm,
                    self._enc(item.title),
                    self._enc(item.snippet) if item.snippet else None,
                    item.backend or "unknown",
                ),
            )
            url_to_result_id[item.url] = int(row.lastrowid)

        self._conn.commit()
        return query_id, url_to_result_id

    def get_visit_metadata(self, urls: list[str]) -> dict[str, dict[str, Any]]:
        if not urls:
            return {}

        norms = [normalize_url(url) for url in urls]
        placeholders = ",".join("?" for _ in norms)
        cursor = self._conn.execute(
            f"""
            SELECT url_norm,
                   MAX(timestamp) AS last_visited,
                   COUNT(*) AS visit_count
            FROM visits
            WHERE url_norm IN ({placeholders})
            GROUP BY url_norm
            """,
            norms,
        )
        norm_to_meta = {
            row["url_norm"]: {
                "last_visited": row["last_visited"],
                "visit_count": row["visit_count"],
            }
            for row in cursor.fetchall()
        }

        output: dict[str, dict[str, Any]] = {}
        for url in urls:
            meta = norm_to_meta.get(normalize_url(url))
            if meta:
                output[url] = meta
        return output

    def record_visit(
        self,
        url: str,
        *,
        result_id: int | None = None,
        browser_id: str | None = None,
        private_mode: bool = False,
    ) -> None:
        url_norm = normalize_url(url)
        self._conn.execute(
            """
            INSERT INTO visits (result_id, url, url_norm, browser_id, private_mode)
            VALUES (?, ?, ?, ?, ?)
            """,
            (result_id, url, url_norm, browser_id, int(private_mode)),
        )
        self._conn.commit()

    def list_history(
        self,
        *,
        q: str | None = None,
        limit: int = 50,
        offset: int = 0,
    ) -> dict[str, Any]:
        if q:
            cursor = self._conn.execute(
                """
                SELECT q.id, q.timestamp, q.query_text_enc, q.mode, q.backends_used,
                       (SELECT COUNT(*) FROM results r WHERE r.query_id = q.id) AS result_count
                FROM queries_fts fts
                JOIN queries q ON q.id = fts.rowid
                WHERE fts.query_text MATCH ?
                ORDER BY q.timestamp DESC
                LIMIT ? OFFSET ?
                """,
                (q, limit, offset),
            )
        else:
            cursor = self._conn.execute(
                """
                SELECT q.id, q.timestamp, q.query_text_enc, q.mode, q.backends_used,
                       (SELECT COUNT(*) FROM results r WHERE r.query_id = q.id) AS result_count
                FROM queries q
                ORDER BY q.timestamp DESC
                LIMIT ? OFFSET ?
                """,
                (limit, offset),
            )

        items = []
        for row in cursor.fetchall():
            items.append(
                {
                    "id": row["id"],
                    "timestamp": row["timestamp"],
                    "query": self._dec(row["query_text_enc"]),
                    "mode": row["mode"],
                    "backends_used": json.loads(row["backends_used"]),
                    "result_count": row["result_count"],
                }
            )

        total = self._conn.execute("SELECT COUNT(*) AS c FROM queries").fetchone()["c"]
        return {"items": items, "total": total, "limit": limit, "offset": offset}

    def delete_history_entry(self, query_id: int) -> bool:
        exists = self._conn.execute("SELECT 1 FROM queries WHERE id = ?", (query_id,)).fetchone()
        if not exists:
            return False
        self._conn.execute("DELETE FROM queries_fts WHERE rowid = ?", (query_id,))
        self._conn.execute("DELETE FROM queries WHERE id = ?", (query_id,))
        self._conn.commit()
        return True

    def purge_all_history(self) -> int:
        count = self._conn.execute("SELECT COUNT(*) AS c FROM queries").fetchone()["c"]
        self._conn.execute("DELETE FROM queries_fts")
        self._conn.execute("DELETE FROM queries")
        self._conn.commit()
        return count

    def list_collections(self) -> list[dict[str, Any]]:
        cursor = self._conn.execute(
            """
            SELECT c.id, c.name, c.created_at,
                   (SELECT COUNT(*) FROM collection_items ci WHERE ci.collection_id = c.id) AS item_count
            FROM collections c
            ORDER BY c.name COLLATE NOCASE
            """
        )
        return [
            {
                "id": row["id"],
                "name": row["name"],
                "created_at": row["created_at"],
                "item_count": row["item_count"],
            }
            for row in cursor.fetchall()
        ]

    def create_collection(self, name: str) -> dict[str, Any]:
        name = name.strip()
        if not name:
            from netrail.errors import NetRailError

            raise NetRailError(
                "COLLECTION_NAME_INVALID",
                "Collection name must be 1-120 characters.",
            )
        try:
            cursor = self._conn.execute(
                "INSERT INTO collections (name) VALUES (?)",
                (name,),
            )
        except sqlite3.IntegrityError as exc:
            from netrail.errors import NetRailError

            raise NetRailError(
                "COLLECTION_EXISTS",
                f"Collection '{name}' already exists.",
            ) from exc
        self._conn.commit()
        collection_id = int(cursor.lastrowid)
        return {"id": collection_id, "name": name, "created_at": datetime.now(timezone.utc).isoformat(), "item_count": 0}

    def add_collection_item(
        self,
        collection_id: int,
        *,
        url: str,
        title: str,
        notes: str | None = None,
    ) -> dict[str, Any]:
        exists = self._conn.execute("SELECT 1 FROM collections WHERE id = ?", (collection_id,)).fetchone()
        if not exists:
            from netrail.errors import NetRailError

            raise NetRailError("COLLECTION_NOT_FOUND", "Collection not found.", status=404)

        self._conn.execute(
            """
            INSERT INTO collection_items (collection_id, url, title, notes)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(collection_id, url) DO UPDATE SET
                title = excluded.title,
                notes = COALESCE(excluded.notes, collection_items.notes),
                saved_at = datetime('now')
            """,
            (collection_id, url, title, notes),
        )
        self._conn.commit()
        return {"collection_id": collection_id, "url": url, "title": title, "notes": notes}

    def export_collection(self, collection_id: int, fmt: str = "json") -> str:
        collection = self._conn.execute(
            "SELECT id, name, created_at FROM collections WHERE id = ?",
            (collection_id,),
        ).fetchone()
        if not collection:
            from netrail.errors import NetRailError

            raise NetRailError("COLLECTION_NOT_FOUND", "Collection not found.", status=404)

        items = self._conn.execute(
            """
            SELECT url, title, notes, saved_at
            FROM collection_items
            WHERE collection_id = ?
            ORDER BY saved_at DESC
            """,
            (collection_id,),
        ).fetchall()

        payload = {
            "collection": {
                "id": collection["id"],
                "name": collection["name"],
                "created_at": collection["created_at"],
            },
            "items": [dict(row) for row in items],
            "exported_at": datetime.now(timezone.utc).isoformat(),
        }

        if fmt == "csv":
            buffer = io.StringIO()
            writer = csv.DictWriter(buffer, fieldnames=["url", "title", "notes", "saved_at"])
            writer.writeheader()
            for row in payload["items"]:
                writer.writerow(row)
            return buffer.getvalue()

        return json.dumps(payload, indent=2)

    def stats(self) -> dict[str, int]:
        return {
            "queries": self._conn.execute("SELECT COUNT(*) AS c FROM queries").fetchone()["c"],
            "visits": self._conn.execute("SELECT COUNT(*) AS c FROM visits").fetchone()["c"],
            "collections": self._conn.execute("SELECT COUNT(*) AS c FROM collections").fetchone()["c"],
        }


def get_store() -> HistoryStore | None:
    global _store
    settings = load_settings()
    if not settings.get("history_enabled", True):
        return None

    if _store is None:
        want_encrypt = settings.get("history_encrypt", True)
        if want_encrypt:
            ensure_encryption_key()
        from netrail.history.crypto import encryption_active

        if want_encrypt and not encryption_active():
            return None
        use_encrypt = want_encrypt and encryption_active()
        _store = HistoryStore(connect(), encrypt=use_encrypt)
        ttl = int(settings.get("history_ttl_days", 90))
        _store.purge_expired(ttl)

    return _store


def init_history_on_startup() -> None:
    get_store()