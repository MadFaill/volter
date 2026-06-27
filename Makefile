.PHONY: build test fmt fmt-check lint fe-install fe-build fe-typecheck check up down

# --- Rust (control-plane / engine / runtime-plane / shared-types) ---
build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

# --- Frontend ---
fe-install:
	cd frontend && npm install --no-audit --no-fund

fe-typecheck:
	cd frontend && npm run typecheck

fe-build:
	cd frontend && npm run build

# --- Полная проверка (как в CI) ---
check: fmt-check lint test fe-typecheck fe-build

# --- Docker (фаза 1, одна машина) ---
up:
	docker compose up -d --build

down:
	docker compose down
