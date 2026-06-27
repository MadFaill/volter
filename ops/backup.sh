#!/usr/bin/env bash
# Бэкап состояния контура перед раскаткой (plan.md §11.1).
# Ш0б: состояние — файловое (data-dir). В Ш1 здесь добавится pg_dump Postgres.
set -euo pipefail

ENVN="${1:-prod}"
cd "$(dirname "$0")/.."

case "$ENVN" in
  dev)  SRC=./data ;;
  test) SRC=./data/test ;;
  prod) SRC=./data/prod ;;
  *) echo "unknown env: $ENVN" >&2; exit 2 ;;
esac

mkdir -p ./backups
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEST="./backups/${ENVN}-${STAMP}.tar.gz"

if [ -d "$SRC" ]; then
  tar -czf "$DEST" -C "$(dirname "$SRC")" "$(basename "$SRC")"
  echo "backup: $DEST"
else
  echo "nothing to back up ($SRC missing) — пропускаю"
fi
