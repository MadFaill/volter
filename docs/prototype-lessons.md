# Конспект уроков прототипов (референс, код не переносим)

> ⛔ `/opt/volt` и `/opt/cply-agent` — **банк опыта**, не источник кода. Здесь — выжимка подходов,
> применяемых при постройке `volter` с нуля. Полные карты — в `plan.md` §2, §19.

## Из `/opt/volt` (control-plane, инфра)
- **Dialog-first SDLC**: идея → план (read-only) → «В работу» (freeze + run) → execution → checks →
  test-deploy → ручной prod-deploy.
- **Нарезка плана под задачу в YAML**: `stages[].steps[]` с контрактом `do`/`verify`/`check`
  (`cmd`/`expect_contains`/`expect_absent`/`requires_validator`), `kind`-правила нарезки, авто-синтез
  validator-стадии. → контрактная модель `plan.md` §6.
- **Безмодельная валидация** `check.cmd` (passed/failed/missing/skipped + таймаут) — не доверять
  «assert» от LLM.
- **Двусторонний git** (internal bare + external), ssh-ключ на проект; **runtime-workspace** на run
  (изоляция); серийность через execution-lock.
- **Single-VPS деплой**: docker compose, user `volt` в группе docker, монтаж `~/.codex`/`~/.claude`,
  root-owned deploy-врапперы, изоляция dev/prod PGDATA (инцидент «run снёс prod-БД» — не повторяем).
- **Single-admin auth**: Argon2 + httpOnly-JWT + setup-wizard. → реализовано в Ш0а.

## Из `/opt/cply-agent` («нейроцех», движок качества)
- **Конвейер 15 фаз SDLC** + микроцикл `plan→synth→gate` (shift-left).
- **Манифесты и роли как правила/критерии в YAML**; `do`/`verify`/`anchor`/`codeAnchor`/`trail`;
  трассировка `anchorsTo → PRODUCT.*`.
- **TDD-гейты, доказывающие реальность тестов**: `tests-green` (baseline-diff), `test-honesty`,
  `contract-coverage`.
- **Event-sourcing**: лог рана — источник истины; стоимость/прогресс/resume/stuck — проекции.
- **Feedback-injection**: провал гейта → `feedback` в промпт следующей попытки.
- **Каталог моделей tier→model + chain/fallback** (остаток лимита, 429). → связки `plan.md` §7.
- **Provisioning** нод (OS-user, systemd-таймеры). → сетап нод (позже).

## Анти-паттерны (не берём)
- Тёмная «приборная» палитра / кастомный CSS → **светлая минималистичная тема, ChatGPT-like, Tailwind**.
- Polling → **WebSocket/SSE**.
- Роли как императивный код → **декларативный YAML** (`plan.md` §6.2).
