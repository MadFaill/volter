# Связки agent+model и резолвер (`volter-contracts::bindings`/`resolver`)

Реализует `plan.md` §7. Связка — атомарная единица исполнения; привязывается **к действию (фазе)**,
разрешается по приоритету и деградирует (fallback) при недоступности.

## Профиль связок (`.agent/bindings.yaml`)
```yaml
bindings:
  - { id: claude-opus, agent: claude, model: claude-opus-4-8, tier: qualified, fallback: [codex-gpt5] }
  - { id: codex-gpt5,  agent: codex,  model: gpt-5,           tier: qualified }
  - { id: dev-claude,  agent: claude-code, model: claude-sonnet-4-6, tier: medium, fallback: [codex-gpt5] }
  - { id: shell,       agent: shell,  tier: fast }
defaults:                          # action → binding id
  planning: claude-opus
  development: dev-claude
  verification: shell
role_pins:
  developer: { require_kind: agentic, prefer: dev-claude }
fallback_policy:
  triggers: [unauthorized, unavailable, rate_limited, budget_low, node_down]
  preserve: tier                   # держать tier при fallback
```
- `agent`: `claude-code`/`codex`/`aider` → **agentic**, `claude` → **text**, `shell` → **toolless**.
- Валидация: уникальные id; все ссылки (`defaults`, `role_pins.prefer`, `fallback`) существуют.

## Резолвер (`resolver::resolve`)
Сигнатура: `resolve(profile, action, role, overrides, available) -> Resolution | ResolveError`.

**Приоритет выбора исходной связки** (§7.3): `message` → `dialog` → `role_pin.prefer` → `defaults[action]`.

**Деградация** (§7.4): если `available(binding) != Ok`, идём по `fallback`-цепочке; при `preserve: tier`
кандидаты другого tier пропускаются (qualified-план не падает на fast). Результат несёт `fallback_from`
и `reason` (для бейджа UI §8.6). Если цепочка исчерпана → `NoAvailableBinding`.

**Жёсткий `require_kind`**: финальная связка обязана соответствовать виду агента роли; даже
override не может сделать `developer` не-агентным → `KindViolation`.

`Availability`: `Ok | Unauthorized | Unavailable | RateLimited | BudgetLow | NodeDown` — детерминированный
провайдер (авторизация Ш6 + лимиты + здоровье ноды). Поиск/деградация — **без LLM**.

## Покрытие
Резолвер протестирован: дефолт по действию; приоритет message>dialog>pin>default; fallback с сохранением
tier и причиной; пропуск кандидата неверного tier; `require_kind` violation; отсутствие связки для действия.
