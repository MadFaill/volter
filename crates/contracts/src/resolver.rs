//! Резолвер связки: приоритет переопределений (§7.3) + деградация (§7.4).

use thiserror::Error;

use crate::bindings::{Action, BindingProfile, Preserve};

/// Доступность связки. Любое не-`Ok` запускает fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Ok,
    Unauthorized,
    Unavailable,
    RateLimited,
    BudgetLow,
    NodeDown,
}

impl Availability {
    fn reason(self) -> &'static str {
        match self {
            Availability::Ok => "ok",
            Availability::Unauthorized => "unauthorized",
            Availability::Unavailable => "unavailable",
            Availability::RateLimited => "rate_limited",
            Availability::BudgetLow => "budget_low",
            Availability::NodeDown => "node_down",
        }
    }
}

/// Переопределения уровня сообщения/диалога (§7.3).
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub message: Option<String>,
    pub dialog: Option<String>,
}

/// Результат разрешения связки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub binding_id: String,
    /// Если сработал fallback — id исходной (недоступной) связки.
    pub fallback_from: Option<String>,
    /// Причина fallback (для бейджа в UI §8.6).
    pub reason: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResolveError {
    #[error("нет связки для действия {0:?}")]
    NoBindingForAction(Action),
    #[error("неизвестная связка: {0}")]
    UnknownBinding(String),
    #[error("нет доступной связки (исходная {primary} недоступна, fallback исчерпан)")]
    NoAvailableBinding { primary: String },
    #[error("связка {binding} нарушает require_kind роли {role}")]
    KindViolation { role: String, binding: String },
}

/// Разрешает связку для `action` (и опционально роли) по приоритету и применяет fallback.
///
/// Приоритет выбора исходной связки: message → dialog → role_pin.prefer → defaults[action].
/// `available` — детерминированный провайдер доступности (авторизация/лимиты/нода).
pub fn resolve(
    profile: &BindingProfile,
    action: Action,
    role: Option<&str>,
    ov: &Overrides,
    available: &dyn Fn(&str) -> Availability,
) -> Result<Resolution, ResolveError> {
    let pin = role.and_then(|r| profile.role_pins.get(r));

    // 1. Выбор исходной связки по приоритету.
    let primary = ov
        .message
        .clone()
        .or_else(|| ov.dialog.clone())
        .or_else(|| pin.and_then(|p| p.prefer.clone()))
        .or_else(|| profile.defaults.get(&action).cloned())
        .ok_or(ResolveError::NoBindingForAction(action))?;

    let primary_b = profile
        .binding(&primary)
        .ok_or_else(|| ResolveError::UnknownBinding(primary.clone()))?;

    // 2. Доступность + деградация.
    let chosen = if available(&primary) == Availability::Ok {
        Resolution {
            binding_id: primary.clone(),
            fallback_from: None,
            reason: None,
        }
    } else {
        let av = available(&primary);
        let preserve_tier = profile.fallback_policy.preserve == Preserve::Tier;
        let mut picked = None;
        for cand_id in &primary_b.fallback {
            let Some(cand) = profile.binding(cand_id) else {
                return Err(ResolveError::UnknownBinding(cand_id.clone()));
            };
            if preserve_tier && cand.tier != primary_b.tier {
                continue; // держим tier (qualified-план не падает на fast)
            }
            if available(cand_id) == Availability::Ok {
                picked = Some(cand_id.clone());
                break;
            }
        }
        let binding_id = picked.ok_or_else(|| ResolveError::NoAvailableBinding {
            primary: primary.clone(),
        })?;
        Resolution {
            binding_id,
            fallback_from: Some(primary.clone()),
            reason: Some(av.reason().to_string()),
        }
    };

    // 3. Жёсткий require_kind роли — на финальной связке.
    if let Some(p) = pin {
        if let Some(req) = p.require_kind {
            let kind = profile.binding(&chosen.binding_id).unwrap().agent.kind();
            if kind != req {
                return Err(ResolveError::KindViolation {
                    role: role.unwrap_or("").to_string(),
                    binding: chosen.binding_id,
                });
            }
        }
    }
    Ok(chosen)
}

/// Удобный провайдер: все связки доступны.
pub fn all_ok(_id: &str) -> Availability {
    Availability::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::BindingProfile;

    // Профиль из тестов bindings.rs.
    fn profile() -> BindingProfile {
        BindingProfile::parse(crate::bindings::tests::PROFILE).unwrap()
    }

    fn ov(message: Option<&str>, dialog: Option<&str>) -> Overrides {
        Overrides {
            message: message.map(str::to_string),
            dialog: dialog.map(str::to_string),
        }
    }

    #[test]
    fn default_for_action() {
        let r = resolve(
            &profile(),
            Action::Planning,
            None,
            &Overrides::default(),
            &all_ok,
        )
        .unwrap();
        assert_eq!(r.binding_id, "claude-opus");
        assert!(r.fallback_from.is_none());
    }

    #[test]
    fn precedence_message_over_dialog_over_pin_over_default() {
        let p = profile();
        // message побеждает всё
        let r = resolve(
            &p,
            Action::Development,
            Some("developer"),
            &ov(Some("codex-gpt5"), Some("claude-sonnet")),
            &all_ok,
        )
        .unwrap();
        assert_eq!(r.binding_id, "codex-gpt5");
        // без message — dialog побеждает pin и default (codex-mini агентна → require_kind ок)
        let r = resolve(
            &p,
            Action::Development,
            Some("developer"),
            &ov(None, Some("codex-mini")),
            &all_ok,
        )
        .unwrap();
        assert_eq!(r.binding_id, "codex-mini");
        // без override — role pin (dev-claude) побеждает default
        let r = resolve(
            &p,
            Action::Development,
            Some("developer"),
            &Overrides::default(),
            &all_ok,
        )
        .unwrap();
        assert_eq!(r.binding_id, "dev-claude");
    }

    #[test]
    fn fallback_preserves_tier_and_reports_reason() {
        let p = profile();
        // claude-opus (qualified) недоступна → codex-gpt5 (qualified) ок
        let avail = |id: &str| {
            if id == "claude-opus" {
                Availability::RateLimited
            } else {
                Availability::Ok
            }
        };
        let r = resolve(&p, Action::Planning, None, &Overrides::default(), &avail).unwrap();
        assert_eq!(r.binding_id, "codex-gpt5");
        assert_eq!(r.fallback_from.as_deref(), Some("claude-opus"));
        assert_eq!(r.reason.as_deref(), Some("rate_limited"));
    }

    #[test]
    fn fallback_skips_wrong_tier() {
        // claude-sonnet (medium) fallback=[codex-gpt5(qualified)] → tier не совпадает, пропускается,
        // других кандидатов нет → нет доступной связки.
        let p = profile();
        let avail = |id: &str| {
            if id == "claude-sonnet" {
                Availability::NodeDown
            } else {
                Availability::Ok
            }
        };
        let err = resolve(&p, Action::Research, None, &Overrides::default(), &avail).unwrap_err();
        assert!(matches!(err, ResolveError::NoAvailableBinding { .. }));
    }

    #[test]
    fn require_kind_violation() {
        // developer require_kind=agentic; форсим текстовую связку через message override.
        let p = profile();
        let err = resolve(
            &p,
            Action::Development,
            Some("developer"),
            &ov(Some("claude-haiku"), None),
            &all_ok,
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::KindViolation { .. }));
    }

    #[test]
    fn no_binding_for_action_errors() {
        let p = profile();
        let err = resolve(&p, Action::E2e, None, &Overrides::default(), &all_ok).unwrap_err();
        assert_eq!(err, ResolveError::NoBindingForAction(Action::E2e));
    }
}
