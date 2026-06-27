-- Ш1: базовая доменная модель volter (plan.md §5.1 + §6/§7).
-- Перечисления — TEXT + CHECK (портируемо, без CREATE TYPE). UUID — gen_random_uuid() (PG16 встроена).

-- Учётка администратора (заменяет файловый UserStore из Ш0а).
CREATE TABLE app_user (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Проекты: контекст = склонированный git; ssh-ключ на проект.
CREATE TABLE projects (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key            TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    repo_url       TEXT,
    ssh_key_ref    TEXT,
    default_branch TEXT NOT NULL DEFAULT 'main',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Диалоги: каждый создаёт git-ветку; несёт агрегаты cost/time.
CREATE TABLE dialogs (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title      TEXT NOT NULL DEFAULT '',
    slug       TEXT,
    branch     TEXT,
    task_class TEXT CHECK (task_class IN ('bug','feature','content','infra','research','refactor')),
    status     TEXT NOT NULL DEFAULT 'chat',
    cost_usd   DOUBLE PRECISION NOT NULL DEFAULT 0,
    time_ms    BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX dialogs_project_idx ON dialogs(project_id, updated_at DESC);

-- Сообщения диалога + стоимость/время каждого ответа (§13/§18).
CREATE TABLE messages (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dialog_id     UUID NOT NULL REFERENCES dialogs(id) ON DELETE CASCADE,
    role          TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content       TEXT NOT NULL DEFAULT '',
    action        TEXT,
    binding_id    TEXT,
    input_tokens  BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_usd      DOUBLE PRECISION NOT NULL DEFAULT 0,
    time_ms       BIGINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX messages_dialog_idx ON messages(dialog_id, created_at);

-- Планы (контрактный YAML §6.3); замораживаются при «В работу».
CREATE TABLE plans (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dialog_id    UUID NOT NULL REFERENCES dialogs(id) ON DELETE CASCADE,
    content_yaml TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'valid' CHECK (status IN ('valid','frozen')),
    version      INT NOT NULL DEFAULT 1,
    frozen_at    TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX plans_dialog_idx ON plans(dialog_id, version DESC);

-- Раны: исполнение замороженного плана.
CREATE TABLE runs (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dialog_id  UUID NOT NULL REFERENCES dialogs(id) ON DELETE CASCADE,
    plan_id    UUID REFERENCES plans(id) ON DELETE SET NULL,
    status     TEXT NOT NULL DEFAULT 'queued',
    branch     TEXT,
    commit_sha TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX runs_dialog_idx ON runs(dialog_id, created_at DESC);

-- Event-sourcing лог рана (§6.4) — источник истины; состояние/стоимость = проекции.
CREATE TABLE events (
    id         BIGSERIAL PRIMARY KEY,
    run_id     UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq        INT NOT NULL,
    type       TEXT NOT NULL,
    payload    JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (run_id, seq)
);

-- Очередь заданий воркера.
CREATE TABLE jobs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type         TEXT NOT NULL,
    payload      JSONB NOT NULL DEFAULT '{}'::jsonb,
    status       TEXT NOT NULL DEFAULT 'queued',
    attempts     INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL DEFAULT 3,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX jobs_status_idx ON jobs(status, created_at);

-- Ноды инфраструктуры (control/worker/stand-*).
CREATE TABLE nodes (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL UNIQUE,
    type       TEXT NOT NULL CHECK (type IN ('control','worker','stand_test','stand_prod')),
    region     TEXT,
    endpoint   TEXT,
    status     TEXT NOT NULL DEFAULT 'unknown',
    capacity   JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Деплои на стенды test/prod.
CREATE TABLE deployments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id   UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    run_id       UUID REFERENCES runs(id) ON DELETE SET NULL,
    environment  TEXT NOT NULL CHECK (environment IN ('test','prod')),
    status       TEXT NOT NULL DEFAULT 'pending',
    slot         TEXT,
    image_ref    TEXT,
    previous_id  UUID REFERENCES deployments(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Учёт стоимости/времени (§18) — проекция, но материализуется для аналитики срезов.
CREATE TABLE cost_entries (
    id            BIGSERIAL PRIMARY KEY,
    dialog_id     UUID REFERENCES dialogs(id) ON DELETE CASCADE,
    message_id    UUID REFERENCES messages(id) ON DELETE SET NULL,
    phase         TEXT,
    binding_id    TEXT,
    input_tokens  BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_usd      DOUBLE PRECISION NOT NULL DEFAULT 0,
    time_ms       BIGINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX cost_entries_dialog_idx ON cost_entries(dialog_id);
