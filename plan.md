# Volter — детальный план реализации

> Источник требований: `README.md`. Учтены два существующих прототипа:
> - **`/opt/volt/repo`** (далее **VOLT**) — рабочий control-plane: Rust/Axum + React/Vite +
>   Postgres + Docker. Диалоги, планы, runs, фазы clarify/plan/execute/review, test/prod деплой,
>   runtime-plane, артефакт-гейт, git internal/external + ssh-ключи на проект, аналитика cost/time.
> - **`/opt/cply-agent`** (далее **ENGINE / «нейроцех»**) — TS-движок оркестрации: 15 фаз SDLC,
>   18 ролей, 21 валидатор (TDD-гейты `tests-green`/`test-honesty`), 8 runners, микроцикл
>   plan→synth→gate (shift-left), event-sourcing, манифесты, author-планирование, energy/CAP-экономика,
>   three-layers инфра + systemd provisioning, LLM-gateway.
>
> Этот документ — **последовательный набор шагов**. Пройдя их все, получаем систему из README.
> Каждый шаг: цель → задачи → опора на существующий код → Definition of Done (DoD).
>
> Принцип №0 (README п.77): **делать качественно с первого раза.** Каждый шаг сам закрывается
> реальными (не mock) тестами, автодокой и фиксацией в git с тегами/индексом.

---

## Часть I. Стратегия и целевая архитектура

### 1. Ключевое архитектурное решение: консолидация двух прототипов

**VOLT** — это фундамент (control-plane, UI, инфраструктура, git, деплой, аналитика).
Развиваем его, а не пишем с нуля. **ENGINE («нейроцех»)** — это «мозг качества»: его модель
фаз/ролей/гейтов/валидаторов и есть тот самый TDD-flow и SDLC, который требует README
(e2e/контракты, «тесты реальные не mock», shift-left, классификация задач, план через research).

Нынешний движок VOLT (упрощённый `clarify/plan/execute/review` в `infrastructure/codex.rs`)
заменяется/обогащается полноценным конвейером ENGINE.

**Способ интеграции (выбран):** ENGINE запускается как **контейнеризованный execution-runner на
воркер-ноде**, VOLT оркестрирует его через **runtime-plane в HTTP-режиме** (сейчас это заглушка
в `backend/src/domain/runtime_plane.rs` — её реализация и есть мост к распределённости). Это:
- переиспользует 257 готовых тестов и проверенную FSM ENGINE (не переписываем на Rust сразу);
- естественно ложится на требование «воркеры на других серверах, управление с RU-ноды»;
- оставляет открытым поздний порт движка в Rust (см. §17, открытый вопрос).

```
┌──────────────────────────── CONTROL PLANE (RU-VPS) — ставится вручную ──────────────────────┐
│  React/Vite UI ──HTTP/WS──► VOLT API (Rust/Axum) ──► VOLT Worker (Rust) ──► Orchestrator      │
│  Postgres │ secrets-store │ git internal/external │ node-manager │ analytics │ LLM-gateway     │
└───────────────────────────────────────────┬───────────────────────────────────────────────────┘
                          runtime-plane (gRPC/HTTP over mTLS, Docker overlay)
        ┌──────────────────────────┬─────────────────────────┬──────────────────────────┐
        ▼                          ▼                         ▼                          ▼
 ┌────────────────┐       ┌────────────────┐        ┌────────────────┐        ┌────────────────┐
 │ WORKER (EU)    │       │ WORKER (EU)    │        │ STAND test     │        │ STAND prod     │
 │ ENGINE(TS) +   │       │ ENGINE(TS) +   │        │ деплой проекта │        │ деплой проекта │
 │ claude-code CLI│       │ codex / aider  │        │ + e2e-прогон   │        │ (app+db compose)│
 │ git workspace  │       │ git workspace  │        │                │        │                │
 └────────────────┘       └────────────────┘        └────────────────┘        └────────────────┘
```

### 2. Что уже готово (не переписывать, опираться)

| Область | Готово в прототипе | Где |
|---|---|---|
| Control-plane API | Axum 0.7, 40+ эндпоинтов, JWT-cookie auth, setup wizard | `volt/backend/src/app.rs` |
| Доменка | projects, dialogs, messages, plans, runs, jobs, deployments, runtime_nodes | `volt/backend/src/domain/*`, миграции 0001–0019 |
| Мультипроект | slug-ключ, домены, ssh-ключ на проект | `domain/project.rs` |
| Git двусторонний | internal (`*.git` bare) + external (GitHub), mirror, work-ветки | `domain/git_origin.rs` |
| Деплой test/prod | jobs, контейнерный switch, rollback, backup | `domain/deployment.rs`, `ops/deploy-{test,prod}` |
| Артефакт-гейт | физический `check.cmd`, expect_contains/absent, validator-stage | `domain/validation.rs` |
| Runtime-plane | workspace prep, artifact collect (local-режим) | `domain/runtime_plane.rs` |
| Runner-бандлы | codex/claude/aider, фазовый роутинг clarify/plan/execute/review | `project_runtime_settings` |
| UI диалогов | DialogThread, DialogsPage (cost/time), RunStatusPanel, аналитика, просмотр md+highlight | `volt/frontend/src/{pages,components}` |
| **Движок качества** | 15 фаз, 18 ролей, 21 валидатор, plan→synth→gate, event-sourcing | `cply-agent/src/{phases,roles,validators,core}` |
| **TDD-гейты** | `tests-green` (real, baseline-diff), `test-honesty`, `contract-coverage` | `cply-agent/src/validators/*` |
| **Author-планирование** | research→product→perspectives→main.md→manifests | `cply-agent/src/author/*` |
| **Git meta-sync** | коммит/пуш по шагам, delivery-ledger, resume/replay | `cply-agent/src/adapters/local/GitDataRepo.ts` |
| **Energy/CAP экономика** | SPEND/CHARGE ledger, bothub treasury, LLM-gateway, fallback-chain | `cply-agent` docs `energy.*`, `§34` |
| **Provisioning** | systemd `agent-poll@.service/.timer`, `provision.ts`, OS-user на проект | `cply-agent/ops`, `three-layers.design.md` |

### 3. Главные пробелы (gap) под README

1. **Распределённость** — runtime-plane HTTP-режим = заглушка; нет реальных воркеров на других VPS.
2. **Provisioning нод «по кнопке»** из UI control-plane (есть только systemd-шаблоны в ENGINE).
3. **Враппер авторизации агентов из web** (claude login / codex auth / aider api-key) — нет UI-флоу.
4. **Единый движок качества** — VOLT использует упрощённые фазы; полноценный ENGINE не подключён.
5. **UI**: нет голоса, нет Tailwind (кастомный CSS), polling вместо SSE/WS, нет «мыслей»-стрима,
   нет deep-research-прогресса, нет просмотра дерева файлов проекта.
6. **Research-флоу** «идея→исследование→сохранить в git→план» как первоклассный сценарий.
7. **DB-ассистент и infra-ассистент** внутри проекта.
8. **Расширяемые skills с генерацией агентом** (в ENGINE skills детерминированы, генерации нет).
9. **Память проекта**: индекс с тегами и осмысленные названия диалогов — частично, нужно довести.

### 4. Стек (фиксируем)

- CP backend/оркестратор: **Rust** (axum, tokio, sqlx, Postgres) — из VOLT.
- Execution-движок: **TypeScript** (ENGINE) в контейнере воркера; CLI-агенты claude-code/codex/aider.
- Frontend: **React + TypeScript + Vite + Tailwind CSS** (миграция с кастомного CSS — см. Ш9).
- Транспорт CP↔ноды: **gRPC/HTTP over mTLS** в Docker overlay; UI↔CP: **WebSocket/SSE** для стримов.
- Хранилища: Postgres (метаданные), object/files (md-память, артефакты), secrets-store.
- Всё через Docker; межхостовая связь через Docker overlay.

---

## Часть II. Последовательные шаги реализации

> Шаги сгруппированы в этапы. Внутри шага задачи можно вести параллельно, но порядок шагов —
> это порядок получения работающей системы. Каждый шаг заканчивается тегом в git и записью в индекс.

### Этап A. Консолидация фундамента

#### Шаг 0 — Свести прототипы в монорепо `volter`
- Перенести VOLT в `volter/control-plane/` (backend+frontend+ops+nginx+compose).
- Перенести ENGINE в `volter/engine/` (TS-движок).
- Общий `docs/` (включая перенос `RESULT_VISION.md`, `neurotcekh.*`, `three-layers.design.md`,
  `energy.*` как референс-архитектуру).
- Корневой `Makefile`/`justfile`: `build`, `test`, `smoke`, `up`, `deploy-{test,prod}`.
- Прогнать существующие тесты обоих прототипов, зафиксировать зелёный baseline.
- **DoD:** оба прототипа собираются и проходят свои тесты внутри `volter/`; CI гоняет оба.

#### Шаг 1 — Единая доменная модель и миграции
- Свести модель: к VOLT-таблицам добавить понятия ENGINE: `phases`, `roles`, `gates/validators`,
  `events` (event-sourcing лог рана), `task_class`, `manifest_units`.
- Зафиксировать: `dialogs`→ветка, `runs`→event-log, `plans`→main.md/manifests.
- Новые миграции 0020+ (не ломать 0001–0019).
- **DoD:** миграции применяются, `control_plane_http_tests` зелёные, схема описана в `docs/data-model.md`.

### Этап B. Распределённость и ноды

#### Шаг 2 — Runtime-plane HTTP/gRPC: реальный воркер-протокол
- Реализовать HTTP/gRPC-режим в `domain/runtime_plane.rs` (сейчас заглушка): `prepare_workspace`,
  `run_stage`, `collect_artifacts`, `stream_logs`, `stream_thoughts` — поверх mTLS.
- На воркере — агент-демон (Rust или тонкий Node-сервис), принимающий задания и запускающий ENGINE.
- Сохранить серийность через `project_execution_locks`, перенести лизы на распределённый случай.
- **DoD:** один run исполняется на отдельной воркер-ноде (EU), логи/артефакты/коммит возвращаются на CP;
  e2e-тест «run на удалённом воркере» зелёный.

#### Шаг 3 — Docker overlay + mTLS между нодами
- Overlay-сеть, выпуск/ротация сертификатов нод, bootstrap-токены с TTL.
- Реестр нод `project_runtime_nodes` расширить: тип (control/worker/stand-test/stand-prod), регион
  (ru/eu), статус, capacity, endpoint, health.
- **DoD:** CP видит здоровье нод; трафик CP↔воркер только по mTLS; тест на отказ ноды (graceful).

#### Шаг 4 — Node-manager: добавление и сетап ноды «по кнопке»
- UI: форма «добавить ноду» (тип, регион, ssh-доступ для bootstrap).
- Сетап по кнопке под тип ноды: набор скриптов + **супервизор их выполнения** (статус/логи/ретраи) —
  взять systemd-подход ENGINE (`agent-poll@.service/.timer`, `provision.ts`) и обернуть в CP.
- Сетап умеет: создать OS-юзера на VPS, поставить Docker, поднять overlay/mTLS, развернуть образ
  (worker/stand), зарегистрировать ноду.
- **DoD:** с чистого EU-VPS нода поднимается одной кнопкой; в UI виден прогресс сетапа по шагам.

### Этап C. Агенты и движок качества

#### Шаг 5 — Враппер авторизации агентов из web
- UI-флоу без участия LLM: «авторизовать агента на ноде» →
  - **claude:** контейнер → `claude login` → показать ссылку/код в UI;
  - **codex:** `codex auth` → ссылка/код;
  - **aider:** форма API key + URL к API.
- Токены/ключи → secrets-store, привязка к ноде/проекту (учесть `volt-auth.json`, тома
  `~/.claude`/`~/.codex` из compose).
- **DoD:** с нуля авторизовать claude и codex на воркере целиком из web; ключ aider сохраняется и
  используется раннером.

#### Шаг 6 — Подключить ENGINE как execution-движок VOLT
- Заменить упрощённый `execute`-flow VOLT на конвейер ENGINE (15 фаз) на воркере.
- Маппинг: VOLT `start-work` → ENGINE `task poll/run`; VOLT `runs.events` ← ENGINE `events.jsonl`;
  VOLT `plans` ← ENGINE `main.md`/manifests.
- Фазовый роутинг моделей (clarify/plan/execute/review → tier→model) свести с `§34` chain/fallback.
- **DoD:** диалог «сделай фичу» проходит intake→…→development→tests-green на воркере; в UI видны фазы.

#### Шаг 7 — TDD-гейты и «тесты реальные, не mock»
- Включить валидаторы ENGINE как обязательный Definition of Done шага:
  `test-modeled`→`tests-written`→`tests-green` (с baseline-diff), `test-honesty`, `contract-coverage`.
- Раздельные дорожки backend/frontend, контрактные тесты, e2e на стенде (фазы `deploy_stand`/`e2e_stand`).
- Свести с физическим артефакт-гейтом VOLT (`check.cmd`) — двойной контроль (модельный + безмодельный).
- **DoD:** шаг не закрывается, пока сьют не написан, не зелёный и не прошёл `test-honesty`; есть тест,
  что mock-сьют отбраковывается.

#### Шаг 8 — Классификация задач и планирование (author)
- Подключить `triage`+`estimate-size` (класс задачи S/M/L, bug/feature/контент/инфра/research).
- План = базовый SDLC + задаче-специфичные шаги; роутинг скилов/ролей по затронутым компонентам
  (архитектура/UI/вординг/контракты/регуляторика) — через manifest-секции и perspectives author.
- Простой случай (один мета-файл) → короткий rewrite-flow; код → полный flow (скорость↔архитектура).
- **DoD:** на одном вводе для разных классов генерируются разные планы; в UI план редактируемо-читаемый
  до «В работу», human-in-the-loop ретраи на гейтах.

### Этап D. UI/UX (диалог, mobile-first, аналитика)

#### Шаг 9 — Миграция фронта на Tailwind + дизайн-система
- Перевести `frontend` на **Tailwind CSS**, сохранив тёмную «приборную» палитру и шрифты.
- Принципы: extra-small, mobile-first, **адаптация под iPad**, компактные простые контролы.
- Сверить с `UI-KIT.md` ENGINE (sidebar project-centric: Проект/Задачи/Платформа/Ассистент).
- **DoD:** ключевые экраны на Tailwind, проходят на 320px и на iPad; визуальная регрессия зафиксирована.

#### Шаг 10 — Диалоговый движок: режимы, стрим мыслей, deep-research-прогресс
- Заменить polling на **WebSocket/SSE** (в ENGINE control-plane SSE уже есть — перенять).
- Переключатель режимов в диалоге: `chat`/`planning`/`execution` (+ под-режимы рассуждение/изменение).
- Стрим «мыслей» для простых диалогов («сейчас анализирую…»); прогресс задачи — как **deep research**
  (этапы=фазы ENGINE, под-шаги, живой апдейт).
- Мета-панель диалога: текущая **git-ветка**, стенд, агент+модель, статус тестов.
- **DoD:** в одном диалоге переключаются режимы; виден живой прогресс фаз и стрим мыслей.

#### Шаг 11 — Память проекта, индекс, список диалогов
- Довести `.agent/`: `index.md` (теги+мета для быстрого recall), `dialogs/<id>.md`, `research/<id>.md`,
  `architecture.md` (живая автодока), ignore-файлы (`.aiderignore`/`.codexignore`/`.claudeignore`).
- **Осмысленные авто-названия** диалогов (в VOLT уже есть генерация title — довести качество).
- Список диалогов как в ChatGPT/Claude: имя + **cost + time** (в VOLT есть — отполировать).
- Каждый шаг → коммит в ветку; «зафиксировать» из диалога → коммит диалога+индекса; теги, индекс
  «вектора изменений».
- **DoD:** новый диалог создаёт ветку; после работы в `.agent/index.md` появляется запись с тегами;
  агент в следующем диалоге поднимает контекст из индекса.

#### Шаг 12 — Просмотр файлов проекта и аналитика
- Дерево файлов проекта + превью с подсветкой (в VOLT есть md+highlight для `docs/fix` — расширить
  на произвольные файлы репо; редактирование — read-only по умолчанию, README п.41).
- Аналитика: срезы по проекту, по всем проектам, по агенту и **связке agent+model** (cost/time/успех
  тестов) — расширить `DialogAnalyticsSummary` и эндпоинт `/api/dialogs/analytics`.
- **DoD:** можно открыть любой файл проекта с подсветкой; дашборд показывает срезы по agent+model.

#### Шаг 13 — Голосовой ввод
- Контрол микрофона в композере → STT → текст агенту (провайдер STT — см. §17).
- **DoD:** голосовое сообщение распознаётся и обрабатывается агентом в диалоге.

### Этап E. Research, ассистенты, скилы, экономика

#### Шаг 14 — Research-флоу первого класса
- Сценарий: обсудить → **запустить исследование** (роль `researcher`, web-discovery) → прогресс как
  deep research → **«сохранить в git» кнопкой** (`.agent/research/<id>.md`) → обсудить → сформировать
  план работ → «В работу».
- **DoD:** исследование запускается из диалога, результат коммитится в ветку и доступен в плане.

#### Шаг 15 — Project env, DB-ассистент, infra-ассистент
- Project env с доступами (БД и т.д.), секреты в secrets-store (не в git) — расширить настройки проекта.
- **DB-ассистент**: внутри проекта собрать аналитику/запросы (напр. база tecdoc) через project env.
- **Infra-ассистент**: быстрые правки инфры (nginx и т.п.) в контексте проекта.
- **DoD:** в проекте с настроенным env DB-ассистент выполняет аналитический запрос; infra-ассистент
  правит nginx-конфиг через диалог.

#### Шаг 16 — Расширяемые skills с генерацией агентом
- Реестр скилов ENGINE (`ALL_SKILLS`) сделать расширяемым из проекта; добавить путь, когда **агент сам
  создаёт скил** по запросу (генерация + регистрация + тест скила) — поверх детерминированной модели ENGINE.
- **DoD:** по запросу «нужен инструмент X» агент создаёт скил, он регистрируется и используется в фазе.

#### Шаг 17 — Экономика и точная стоимость
- Подключить energy/CAP-модель ENGINE: `SPEND/CHARGE` ledger, стоимость на **каждый ответ LLM** и
  время его работы, агрегаты по диалогу/проекту; LLM-gateway `/llm/v1` + fallback-chain `§34`.
- Показать реальную стоимость и время диалога и каждого ответа (в UI частично есть — довести до «каждый ответ»).
- **DoD:** для каждого ответа агента видны cost и time; ledger сходится (CHARGE−SPEND) в тестах.

### Этап F. Деплой, стенды, релиз системы

#### Шаг 18 — Деплой проектов на test/prod и полноценные e2e на стенде
- Довести фазы `deploy_stand`/`e2e_stand`/`promotion` ENGINE поверх VOLT-деплоя (`deploy-test`/`deploy-prod`,
  rollback, backup, nginx wildcard-роутинг проектов).
- Полноценные e2e гоняются на **тестовом стенде**; prod — только по явному подтверждению (manual approval).
- **DoD:** проект едет идея→план→разработка→test-стенд(e2e зелёные)→prod по кнопке; rollback работает.

#### Шаг 19 — Развёртывание control-plane и приёмка всей системы
- Один скрипт/кнопка для разворачивания **только CP** (RU-VPS), дальше всё — из UI (Ш4/Ш5).
- Сквозной приёмочный e2e: поднять CP → добавить EU-воркер кнопкой → авторизовать агента из web →
  добавить git-проект → диалог → research → план → разработка с TDD → test-стенд → prod.
- Обновить автодоку и `docs/`; финальный тег релиза.
- **DoD:** сценарий из README §62 («поднять инфру, агентов на разных VPS, CP — и работать») проходит
  целиком одним сквозным тестом.

---

## Часть III. Сводки

### 16. Definition of Done для каждого шага (инвариант качества)
- Канонная архитектура; код читается как окружение.
- Реальные тесты написаны, зелёные, прошли `test-honesty` (не mock); раздельно backend/frontend/contract/e2e.
- Автодока (`architecture.md`/`docs/`) обновлена.
- Зафиксировано в git: коммиты по шагам, тег, запись в индекс памяти (`vector of changes`).

### 17. Открытые вопросы (решить по ходу)
1. **Порт ENGINE в Rust** — оставляем TS-движок на воркере (рекоменд.) или со временем переписываем
   на Rust ради единого стека? Влияет на Ш6 и далее.
2. **«Нейроцех»** — energy/CAP-экономика и биллинг (СБП/ЮKassa) нужны в volter или это была отдельная
   коммерческая обвязка? Если не нужна монетизация — берём только cost/time-аналитику (Ш17 урезается).
3. **STT-провайдер** (Ш13): локальный (whisper) vs внешний API.
4. **Secrets-store**: Vault / sops / age / встроенное.
5. **Регионы/провайдеры VPS** (RU и EU), сетевые ограничения для overlay/mTLS.
6. **Редактирование файлов** в UI — только просмотр (README п.41) или всё же правка.
7. **Модель cross-host транспорта** — gRPC vs HTTP для runtime-plane (Ш2).

### 18. Карта «требование README → шаг»
- git-проекты, контекст=клон, лёгкое добавление → Ш1, Ш11
- отдельные VPS под codex/claude в EU, CP в РФ, управление с РФ → Ш2, Ш3, Ш4
- связка agent+model+mode, выбор связки → Ш5, Ш6 (+ runner-бандлы VOLT)
- режимы рассуждение/планирование/изменение, выбор «как делать» → Ш8, Ш10
- планирование: архитектура/ui/вординг/skills по компонентам → Ш8 (author/perspectives)
- TDD на каждом шаге, e2e/контракты, backend/frontend, реальные тесты → Ш7, Ш18
- живая архитектура + автодок, фиксация с тегами, индекс → Ш11, §16
- ssh-ключ на проект, ветка на диалог, коммит на шаг → Ш1, Ш11 (+ git_origin VOLT)
- деплой test/prod, e2e на стенде → Ш18
- исследования, идеи, консультации, план агентом, этапы → Ш8, Ш10, Ш14
- human-in-the-loop ретраи, классификация задач → Ш7, Ш8
- UI: диалог, mobile-first/iPad, Tailwind, extra-small → Ш9, Ш10
- cost/time на ответ и диалог, аналитика по agent+model → Ш12, Ш17
- просмотр файлов с подсветкой, мета-панель ветки → Ш10, Ш12
- память: md-диалоги + индекс с тегами, project env, БД → Ш11, Ш15
- голос → Ш13
- расширяемые skills (агент создаёт) → Ш16
- volt на Rust + React → §4
- трансляция «мыслей», прогресс как deep research → Ш10
- всё через Docker, межхостовая связь, provisioning нод → Ш2, Ш3, Ш4
- враппер авторизации claude/codex/aider из web → Ш5
- список диалогов с осмысленными именами, cost/time → Ш11
- DB-ассистент, infra-ассистент → Ш15
- учесть фишки нейроцеха и текущего вольта → вся Часть I (консолидация)
- качество с первого раза → §16, Принцип №0

### 19. Карта компонентов прототипов → роль в volter
- `volt/backend` (Rust/Axum) → ядро control-plane (API, оркестрация, git, деплой, аналитика).
- `volt/frontend` (React/Vite) → UI (мигрируется на Tailwind, +голос/стримы/дерево файлов).
- `volt/domain/runtime_plane.rs` → мост к воркерам (HTTP/gRPC-режим — Ш2).
- `volt/ops`, `compose.*`, `nginx` → деплой и роутинг (база для Ш4, Ш18).
- `cply-agent/src/{phases,roles,validators}` → движок качества/TDD (Ш6, Ш7, Ш8).
- `cply-agent/src/author` → планирование/исследование (Ш8, Ш14).
- `cply-agent/src/adapters/local/GitDataRepo` → git meta-sync по шагам (Ш11).
- `cply-agent/ops` + `three-layers.design.md` → provisioning нод (Ш4).
- `cply-agent` energy/LLM-gateway → экономика и стоимость (Ш17).
