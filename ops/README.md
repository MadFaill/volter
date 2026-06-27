# Развёртывание volter (фаза 1: одна машина)

Соответствует `plan.md` §11 и шагу Ш0б. Всё co-located на одной VPS; распределённость (EU-воркеры) —
позже (Ш3–Ш5) через тот же интерфейс runtime-plane, без переписывания.

## Контуры
| Контур | compose | проект | порт (nginx) | data-dir |
|--------|---------|--------|--------------|----------|
| dev    | `docker-compose.yml`      | `volter`      | `VOLTER_HTTP_PORT` (8090)      | `./data`      |
| test   | `docker-compose.test.yml` | `volter-test` | `VOLTER_TEST_HTTP_PORT` (3002) | `./data/test` |
| prod   | `docker-compose.prod.yml` | `volter-prod` | `VOLTER_PROD_HTTP_PORT` (8088) | `./data/prod` |

Контуры изолированы (разные compose-проекты, порты и data-dir) — пересечения данных нет.

## Сервисы (на контур)
`backend` (`volter-api`, Rust) → `frontend` (статика за nginx) → `nginx` (reverse-proxy: `/api/` →
backend, `/` → frontend). Postgres появится в Ш1 (сейчас админ хранится файлом в data-dir).

## Команды
```bash
ops/deploy.sh dev          # build + up + healthcheck (то же для test|prod)
ops/smoke.sh http://127.0.0.1:8090   # сквозной smoke закрытого доступа
ops/backup.sh prod         # бэкап data-dir перед раскаткой (в Ш1 — pg_dump)
sudo ops/install.sh        # операторский bootstrap /opt/volter (один раз на VPS)
```
Прод-деплой запускает бэкап автоматически и остаётся **ручным**.

## TLS / домен
`volter.comalert.pw` терминируется вышестоящим Caddy — см. `ops/Caddyfile.example`
(проксирует на порт контура). Существующие домены прототипов не затрагиваются.

## Агенты
codex/claude уже авторизованы под пользователем `volt` (`~/.codex`, `~/.claude`) и монтируются в
контейнеры (раскомментировать тома в `docker-compose.yml` при подключении движка — Ш6/Ш7).
