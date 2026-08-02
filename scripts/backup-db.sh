#!/usr/bin/env bash
# Online (WAL-safe) backup of the NetRail history database.
#
# Uses the sqlite3 CLI `.backup` command, which is safe to run while the
# service is live — no need to stop NetRail first. For systemd deployments
# combine with a timer: run `systemctl edit netrail-api.timer` or a cron
# line such as:
#
#   17 3 * * * netrail bash /opt/netrail/backup-db.sh /var/lib/netrail/netrail.db /var/backups/netrail/db-$(date +\%Y\%m\%d).sqlite
#
# Restore (service stopped): sqlite3 "$DB" ".restore '$BACKUP'"
set -euo pipefail

DB="${1:-${NETRAIL_DB_PATH:-$HOME/.local/share/netrail/netrail.db}}"
OUT="${2:-${DB}.backup-$(date +%Y%m%d-%H%M%S)}"

if [[ ! -f "$DB" ]]; then
  echo "error: no database at $DB" >&2
  exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "error: sqlite3 CLI required (apt install sqlite3)" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"
sqlite3 "$DB" ".backup '$OUT'"
echo "backed up $DB -> $OUT"
