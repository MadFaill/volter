#!/usr/bin/env bash
# Сквозной smoke закрытого доступа через работающий стек (plan.md Ш0б DoD).
# Использование: ops/smoke.sh [BASE_URL]   (по умолчанию http://127.0.0.1:8090)
# Идемпотентен: если админ уже создан — проверяет вход существующими кредами из env.
set -euo pipefail

BASE="${1:-http://127.0.0.1:8090}"
USER="${VOLTER_SMOKE_USER:-smoke-admin}"
PASS="${VOLTER_SMOKE_PASS:-smoke secret pass 123}"
JAR="$(mktemp)"
trap 'rm -f "$JAR"' EXIT

say() { printf '%-22s %s\n' "$1" "$2"; }
fail() { echo "SMOKE FAILED: $1" >&2; exit 1; }

# health публичен
curl -fsS "$BASE/api/health" >/dev/null || fail "health"
say "health" "ok"

# /me без сессии обязан быть 401 (доступ закрыт)
code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/auth/me")
[ "$code" = 401 ] || fail "/me без сессии вернул $code, ожидался 401"
say "me (no session)" "401 ok"

needs=$(curl -fsS "$BASE/api/setup/status" | grep -o '"needs_setup":[a-z]*' || true)
if echo "$needs" | grep -q true; then
  curl -fsS -c "$JAR" -X POST "$BASE/api/setup/complete" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$USER\",\"password\":\"$PASS\"}" >/dev/null || fail "setup"
  say "setup" "created admin"
else
  curl -fsS -c "$JAR" -X POST "$BASE/api/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$USER\",\"password\":\"$PASS\"}" >/dev/null || fail "login (existing admin)"
  say "login" "ok"
fi

# защищённый /me с сессией обязан быть 200
curl -fsS -b "$JAR" "$BASE/api/auth/me" | grep -q "$USER" || fail "/me с сессией"
say "me (session)" "ok"

# фронт за логином отдаётся
curl -fsS "$BASE/" | grep -qi "volter\|<div id=\"root\"" || fail "frontend index"
say "frontend" "served"

echo "SMOKE OK ($BASE)"
