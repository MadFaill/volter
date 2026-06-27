# Контрактная YAML-модель (крейт `volter-contracts`)

Реализует `plan.md` §6: правила/критерии и нарезка шагов. Все артефакты строго валидируются на Rust —
невалидный YAML отклоняется до исполнения. Модули: `manifest`, `role`, `plan` (+ `bindings`/`resolver`,
см. `bindings.md`).

## Слой 1 — Канон: manifest units (`manifest.rs`)
Адресуемое правило проекта (`.agent/manifest/<section>.yaml`):
```yaml
units:
  - id: ARCH.LAYER.ddd          # стабильный адрес
    section: architecture
    severity: hard              # hard | soft
    tags: [layering, ddd]
    text: "Бэкенд по DDD; зависимости только внутрь."
    code_anchors: ["backend/src/domain"]
    anchors_to: [PRODUCT.QUALITY.maintainable]   # цепочка к корню PRODUCT.*
```
- `Manifest::traces_to_product(id)` — BFS по `anchors_to` до `PRODUCT.*` (для гейта `product-anchored`).
- Валидация: непустые `id`/`text`, уникальность `id`.

## Слой 1 — Роли (`role.rs`)
Роль = декларативные **правила** (`guard`) + **критерии** (`gate`); код роли не нужен:
```yaml
id: architect
purpose: "Привязать каждое требование к реальным символам кода."
manifest_sections: [architecture, core]
code_access: anchored            # search | anchored
guard:
  does: "Привязывает каждое требование к реальным символам."
  must_not: "Менять anchor или продуктовый смысл требований."
gate: [symbol-contract, product-anchored]
prompt_template: roles/architect.md
```
Движок собирает промпт из `prompt_template` + срез манифеста по `manifest_sections` + код по anchors +
feedback гейта (Ш7). Валидация: непустые `purpose`, `guard.does`, `guard.must_not`.

## Слой 2 — План задачи (`plan.rs`)
Нарезка **под задачу**; каждый шаг несёт контракт `do`/`verify`/`check` + `satisfies` (id правил канона):
```yaml
class: feature                   # bug|feature|content|infra|research|refactor
stages:
  - id: S1
    kind: tests                  # implementation|refactor|scaffold|tests|validation|root_cause|e2e|docs|deploy
    files: [backend/src/domain/teaser_test.rs]
    steps:
      - do: "написать red-тест teaser_returns_by_uuid"
        verify: "тест падает до реализации"
        check: { cmd: "cargo test teaser_returns_by_uuid", expect_contains: "test result" }
        satisfies: [PRODUCT.SCENARIO.public_teaser]
    done: "Контракт теста красный."
```
Детерминированные правила нарезки (`Plan::validate`, без LLM):
- каждый шаг имеет непустые `do`, `verify` и `check` (либо `cmd`, либо `requires_validator` + `validator_file`);
- `kind ∈ {implementation,refactor,scaffold}` обязан называть `files`;
- наличие код-стадий → обязательна `tests`-стадия;
- `class: bug` → первая стадия `root_cause`.

`Plan::satisfied_units()` — все id правил, которые план обещает закрыть (вход для гейта `contract-coverage`, Ш8).

## Покрытие тестами
`cargo test -p volter-contracts` (19 тестов): трассировка к PRODUCT.* и дубликаты манифеста; guard роли;
все правила нарезки плана (валидный/битый); парсинг профиля связок и резолвер (см. `bindings.md`).
