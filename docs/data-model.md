# Доменная модель и схема БД (Ш1)

Postgres-схема: `crates/control-plane/migrations/0001_init.sql` (применяется на старте через
`sqlx::migrate!`). Перечисления — `TEXT` + `CHECK`, идентификаторы — `uuid` (`gen_random_uuid()`).
sqlx подключён **без TLS-фичи** (локальный/compose Postgres без TLS — не тянем C-крипто).

## Таблицы
| Таблица | Назначение | Ключевые связи |
|---|---|---|
| `app_user` | учётка администратора (заменяет файловый стор Ш0а) | — |
| `projects` | проект = клон git; `key` (slug), `repo_url`, `ssh_key_ref`, `default_branch` | — |
| `dialogs` | диалог → git-ветка; `task_class`, `status`, агрегаты `cost_usd`/`time_ms` | `project_id` |
| `messages` | сообщения + стоимость/время каждого ответа; `action`, `binding_id` | `dialog_id` |
| `plans` | контрактный YAML плана (§6.3); `status` valid/frozen, `version` | `dialog_id` |
| `runs` | исполнение замороженного плана; `branch`, `commit_sha` | `dialog_id`, `plan_id` |
| `events` | event-sourcing лог рана (§6.4); `(run_id, seq)` уникален | `run_id` |
| `jobs` | очередь заданий воркера | — |
| `nodes` | ноды инфраструктуры (control/worker/stand_*) | — |
| `deployments` | деплои на test/prod; `previous_id` для rollback | `project_id`, `run_id` |
| `cost_entries` | материализованный учёт стоимости/времени для аналитики срезов (§18) | `dialog_id`, `message_id` |

## Инварианты (plan.md §6/§7/§9/§10)
- `dialog` → своя git-ветка; `run` → event-log (источник истины), состояние/стоимость = проекции.
- `plan` (frozen) → исполняемые шаги; `step.satisfies` → `manifest.unit.id`; `unit.anchors_to` → `PRODUCT.*`.
- `action` → `binding` разрешается резолвером (см. `bindings.md`), не хранится жёстко.

## Хранилище учётки (`UserStore`)
Async-трейт с тремя реализациями: `PgUserStore` (Postgres, основная), `FileUserStore`
(dev/фаза 1 без БД), `MemoryUserStore` (тесты). Выбор в `main.rs`: при заданном `DATABASE_URL` —
Postgres + миграции, иначе файл. Хендлеры зависят только от трейта.

## Канон проекта (YAML, в `.agent/`, версионируется в git)
manifest units / roles / plan / bindings — **не в БД**, а файлами в клоне проекта (контекст = git),
строго валидируются крейтом `volter-contracts` (см. `contracts.md`, `bindings.md`).

## Проверка
`cargo test --workspace` — зелёный (включая async стор-тесты и http-e2e). Миграции и `PgUserStore`
проверены против одноразового Postgres: применяются все 12 таблиц, цикл setup→me→login через БД работает.
