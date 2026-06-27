# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

Volter is a **dialog-first control plane that wraps coding agents** (claude/codex/aider) to take software
from idea → research → plan → TDD development → test/prod deploy across a (future) distributed
infrastructure. It is being **built from scratch** in this repo.

Three documents are the source of truth — read them before non-trivial work:
- **`README.md`** — the owner's requirements (in Russian). The authority on *what* to build.
- **`plan.md`** — the detailed, numbered implementation roadmap (steps `Ш0…Ш20`). The authority on *how*
  and *in what order*. Has a table of contents; §1–§11 are architecture, Part II are the steps.
- **`ui-concept.md`** — the visual/UX concept (design tokens, wireframes, copy, login screen).

### Hard rule: the prototypes are reference-only (taboo to touch)
`/opt/volt` (a Rust/React control-plane) and `/opt/cply-agent` (a TS "нейроцех" quality engine) are
**previous iterations**. They are a **bank of architectural experience, not code to import or migrate**.
**Never edit, copy, or depend on their code.** Read them only to learn proven approaches; everything is
written fresh here. `docs/prototype-lessons.md` is the distilled conspect.

### Locked decisions (see plan.md "Принятые решения")
- **Single stack: Rust + React.** The quality engine is (re)written in Rust, not ported from TS.
- **No monetization** — only cost/time accounting (no energy/CAP/billing).
- **Light, minimalist, ChatGPT-like UI** on Tailwind. The prototype's dark "приборная" theme is rejected.
- Conventions: **docs, comments, and commit messages are written in Russian.**

## Repository layout

- `crates/control-plane` — axum HTTP API + orchestrator; binary `volter-api`. Auth lives here.
- `crates/engine` — quality engine (SDLC phases / contract model). Skeleton today, grows per plan §6/Ш7.
- `crates/runtime-plane` — execution abstraction (local now, remote/mTLS later). Trait + noop impl.
- `crates/shared-types` — types shared across crates.
- `frontend` — React + TS + Vite + Tailwind. Light theme; tokens in `tailwind.config.js` mirror ui-concept §3.
- `docker-compose.yml` (dev) + `*.test.yml`/`*.prod.yml`; `nginx/`, Dockerfiles per crate/frontend.

Implementation status: **Ш0, Ш0а, Ш0б, and Ш1 are done.** Deploy/smoke tooling lives in `ops/`
(see `ops/README.md`).

- `crates/contracts` (`volter-contracts`) — the contract YAML model (§6: manifest/role/plan) and
  bindings + resolver (§7). Strict validation; 19 tests. This is the architectural core — read
  `docs/contracts.md` and `docs/bindings.md` before engine/planning work.
- Postgres schema in `crates/control-plane/migrations/0001_init.sql`, applied via `sqlx::migrate!` on
  start (`docs/data-model.md`). sqlx is built **without a TLS feature** (local/compose PG, no TLS) to
  avoid C-crypto deps. `UserStore` is async with `PgUserStore`/`FileUserStore`/`MemoryUserStore`;
  `main.rs` picks Postgres when `DATABASE_URL` is set, else file-backed.

## Commands

⚠️ **Environment quirks in this sandbox** (do not commit workarounds; CI does not need them):
- `cargo`/`rustc` are not on `PATH` by default — run `source "$HOME/.cargo/env"` first.
- `/usr/local/bin/cc` is a shim that launches `claude`, **not a C compiler**. Rust linking fails unless
  you point the linker at the real gcc. Prefix cargo commands with:
  `export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/gcc CC=/usr/bin/gcc CXX=/usr/bin/g++`

Backend (Rust workspace):
```bash
cargo build --workspace
cargo test  --workspace
cargo test  -p volter-control-plane full_auth_lifecycle   # a single test by name
cargo fmt --all -- --check                                # CI gate
cargo clippy --workspace --all-targets -- -D warnings     # CI gate (warnings are errors)
```

Frontend (`cd frontend`):
```bash
npm install --no-audit --no-fund
npm run typecheck        # tsc --noEmit
npm run build            # tsc --noEmit && vite build
npm run dev              # vite, proxies /api → VITE_API_TARGET (default http://localhost:8080)
```

`Makefile` wraps these: `make test`, `make lint`, `make fmt-check`, `make fe-build`, `make check`
(full CI-equivalent), `make up`/`make down` (docker compose). CI is `.github/workflows/ci.yml`.

Deploy / smoke (single machine, plan §11). `deploy.sh` builds images, brings the contour up, and
healthchecks; `smoke.sh` exercises closed access through nginx. Pick a free port — `80/8080/8088/8099`
are taken by the running prototypes:
```bash
VOLTER_HTTP_PORT=8090 ops/deploy.sh dev    # build+up+healthcheck (also test|prod)
ops/smoke.sh http://127.0.0.1:8090         # health, /me→401, setup/login, /me→200, frontend
docker compose -p volter down              # teardown
```

Run the API directly (file-backed admin):
```bash
VOLTER_DATA_DIR=/tmp/v VOLTER_BIND=127.0.0.1:8231 ./target/debug/volter-api
```
Env: `VOLTER_BIND` (default `0.0.0.0:8080`), `VOLTER_DATA_DIR` (default `./data`),
`VOLTER_JWT_SECRET` (auto-generated into `<data>/jwt.secret` if unset).
Note: ports `8099`/`8080` may be occupied by the running prototypes — pick a free port for smoke tests.

## Architecture notes that span files

- **Auth (control-plane).** `lib.rs` builds the router and exposes the `Auth` extractor that gates every
  route except `/api/health`, `/api/setup/*`, and `/api/auth/login|logout`. `auth.rs` implements Argon2
  hashing and a hand-rolled **HS256 JWT** (HMAC-SHA256 — intentionally avoids `ring`/C deps) plus
  httpOnly cookie helpers. `store.rs` defines `UserStore` (object-safe trait) with `FileUserStore` and
  `MemoryUserStore`; handlers depend on the trait so Ш1 can swap in Postgres without touching them.
- **Contract YAML model (plan §6) — the core to internalize before engine work.** Three layers:
  manifest+roles = standing rules/criteria in YAML (canon); per-task plan = `stages[].steps[]` with a
  `do`/`verify`/`check` contract sliced to the task; execution = run steps against **model-free `check`s**
  plus role gates, injecting gate feedback into the next attempt. Quality is enforced structurally
  (tests must be real, not mock — `test-honesty`/`contract-coverage` gates).
- **Bindings (plan §7).** An `agent+model` binding is the atomic execution unit, attached **per action/
  phase** (chat/research/planning/development/…), resolved by precedence
  (message → dialog → role-pin → project default → system) with a fallback chain when unavailable.
- **Memory (plan §9).** Two scopes (project-wide + per-dialog) under `.agent/`, with a tag/FTS meta-index
  and **model-free retrieval**: the LLM only states what to search; a Rust+unix retriever (ripgrep/git/
  SQLite-FTS) finds and returns context structurally. No embeddings on the LLM side.
- **Git & artifacts (plan §10).** One **git worktree per dialog** (decided); artifacts are committed
  granularly "as created" so git operations never lose work.

## Working conventions

- Each plan step ends with: real (non-mock) tests green, fmt+clippy clean, docs updated, and a git
  commit (steps may be tagged). Mark completed steps as ✅ in `plan.md`.
- The local working branch is `master`; the remote default is `main`. Pushes use `git push origin master:main`.
  Only commit/push when asked.
