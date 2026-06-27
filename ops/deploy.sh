#!/usr/bin/env bash
# Развёртывание стека volter на одной машине (plan.md §11, Ш0б).
# Использование: ops/deploy.sh [dev|test|prod]
# Контуры изолированы: свой compose-проект, порт и data-dir (как в /opt/volt — не повторяем
# инцидент пересечения данных).
set -euo pipefail

ENVN="${1:-dev}"
cd "$(dirname "$0")/.."

case "$ENVN" in
  dev)  FILE=docker-compose.yml;       PROJECT=volter;       PORT="${VOLTER_HTTP_PORT:-8090}";       DATADIR=./data ;;
  test) FILE=docker-compose.test.yml;  PROJECT=volter-test;  PORT="${VOLTER_TEST_HTTP_PORT:-3002}";  DATADIR=./data/test ;;
  prod) FILE=docker-compose.prod.yml;  PROJECT=volter-prod;  PORT="${VOLTER_PROD_HTTP_PORT:-8088}";  DATADIR=./data/prod ;;
  *) echo "unknown env: $ENVN (dev|test|prod)" >&2; exit 2 ;;
esac

# Предсоздаём data-dir под вызывающим пользователем (= uid контейнера), иначе docker
# создаст bind-mount от root и backend под uid volt не сможет в него писать.
mkdir -p "$DATADIR"

if [ "$ENVN" = prod ]; then
  echo "==> prod: бэкап перед раскаткой"
  ops/backup.sh prod || { echo "backup failed, aborting prod deploy" >&2; exit 1; }
fi

echo "==> build+up [$ENVN] project=$PROJECT port=$PORT"
docker compose -f "$FILE" -p "$PROJECT" up -d --build

echo "==> healthcheck http://127.0.0.1:$PORT/api/health"
for i in $(seq 1 30); do
  if curl -fsS "http://127.0.0.1:$PORT/api/health" >/dev/null 2>&1; then
    echo "OK ($i)"; exit 0
  fi
  sleep 2
done
echo "healthcheck FAILED" >&2
docker compose -f "$FILE" -p "$PROJECT" ps
exit 1
