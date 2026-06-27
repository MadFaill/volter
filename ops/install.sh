#!/usr/bin/env bash
# Операторский bootstrap ноды под volter (фаза 1, одна машина — plan.md §11).
# Запускается оператором с sudo ОДИН раз на чистой VPS. По образцу /opt/volt:
# раскладка /opt/volter, root-owned deploy-врапперы, .env из шаблона (секреты не перезаписываются).
#
#   sudo ops/install.sh
set -euo pipefail

PREFIX="${VOLTER_PREFIX:-/opt/volter}"
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
USER_NAME="${VOLTER_USER:-volt}"   # уже существует, в группе docker, с авторизованными codex/claude

echo "==> раскладка $PREFIX"
install -d -m 0755 "$PREFIX" "$PREFIX/bin" "$PREFIX/test" "$PREFIX/prod" "$PREFIX/backups"

echo "==> .env (не перезаписываю существующий)"
[ -f "$PREFIX/.env" ] || install -m 0640 -o "$USER_NAME" "$REPO_DIR/.env.example" "$PREFIX/.env"

echo "==> root-owned deploy-врапперы → $PREFIX/bin"
for env in test prod; do
  cat > "$PREFIX/bin/deploy-$env" <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$REPO_DIR"
exec ops/deploy.sh $env
EOF
  chmod 0755 "$PREFIX/bin/deploy-$env"
done

cat <<EOF

Готово. Дальнейшие шаги оператора:
  1) Заполнить секреты:        nano $PREFIX/.env
  2) Прод-деплой (ручной):     sudo $PREFIX/bin/deploy-prod
  3) Reverse-proxy + TLS:      см. ops/Caddyfile.example (домен volter.comalert.pw)
Агенты codex/claude уже авторизованы под $USER_NAME (~/.codex, ~/.claude) и монтируются в контейнеры.
EOF
