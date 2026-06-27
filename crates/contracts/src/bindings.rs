//! Связки `agent+model` и профиль `action → binding` (plan.md §7).

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::{from_yaml, ContractError};

/// Тип агента в связке.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    ClaudeCode,
    Claude,
    Codex,
    Aider,
    Shell,
}

/// Класс агента: агентный (пишет код), текстовый (трансформер), безмодельный (shell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Agentic,
    Text,
    Toolless,
}

impl Agent {
    pub fn kind(self) -> AgentKind {
        match self {
            Agent::ClaudeCode | Agent::Codex | Agent::Aider => AgentKind::Agentic,
            Agent::Claude => AgentKind::Text,
            Agent::Shell => AgentKind::Toolless,
        }
    }
}

/// Уровень модели (для tier-резолва и сохранения tier при fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Fast,
    Medium,
    Qualified,
}

/// Действие (фаза), к которому привязывается связка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Chat,
    Research,
    Planning,
    TestModeling,
    TestWriting,
    Development,
    Review,
    Verification,
    E2e,
}

/// Связка — атомарная единица исполнения (§7.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub id: String,
    pub agent: Agent,
    #[serde(default)]
    pub model: Option<String>,
    pub tier: Tier,
    #[serde(default)]
    pub mode_params: BTreeMap<String, String>,
    /// Где авторизована (Ш6); определяет доступность.
    #[serde(default)]
    pub auth_ref: Option<String>,
    /// Цепочка деградации (ссылки на id связок).
    #[serde(default)]
    pub fallback: Vec<String>,
}

/// Точечный пин роли (перебивает phase default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RolePin {
    /// Жёсткое ограничение вида агента (напр. developer обязан быть `agentic`).
    #[serde(default)]
    pub require_kind: Option<AgentKind>,
    /// Мягкий выбор связки (переопределяется message/dialog override).
    #[serde(default)]
    pub prefer: Option<String>,
}

/// Что сохраняем при fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Preserve {
    #[default]
    Tier,
    None,
}

/// Политика деградации (§7.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackPolicy {
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub preserve: Preserve,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            triggers: vec![],
            preserve: Preserve::Tier,
        }
    }
}

/// Профиль связок проекта (`.agent/bindings.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingProfile {
    pub bindings: Vec<Binding>,
    #[serde(default)]
    pub defaults: HashMap<Action, String>,
    #[serde(default)]
    pub role_pins: HashMap<String, RolePin>,
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
}

impl BindingProfile {
    pub fn parse(yaml: &str) -> Result<Self, ContractError> {
        let p: BindingProfile = from_yaml(yaml)?;
        p.validate()?;
        Ok(p)
    }

    pub fn binding(&self, id: &str) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.id == id)
    }

    /// Уникальность id связок и существование всех ссылок (defaults/pins/fallback).
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut seen = std::collections::HashSet::new();
        for b in &self.bindings {
            if !seen.insert(b.id.as_str()) {
                return Err(ContractError::DuplicateId(b.id.clone()));
            }
        }
        let known = |id: &str| self.binding(id).is_some();
        for id in self.defaults.values() {
            if !known(id) {
                return Err(ContractError::Plan(format!(
                    "defaults → неизвестная связка {id}"
                )));
            }
        }
        for pin in self.role_pins.values() {
            if let Some(id) = &pin.prefer {
                if !known(id) {
                    return Err(ContractError::Plan(format!(
                        "role_pin → неизвестная связка {id}"
                    )));
                }
            }
        }
        for b in &self.bindings {
            for f in &b.fallback {
                if !known(f) {
                    return Err(ContractError::Plan(format!(
                        "fallback связки {} → неизвестная связка {f}",
                        b.id
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const PROFILE: &str = r#"
bindings:
  - { id: claude-haiku,  agent: claude,       model: claude-haiku-4-5,  tier: fast }
  - { id: claude-sonnet, agent: claude,       model: claude-sonnet-4-6, tier: medium, fallback: [codex-gpt5] }
  - { id: claude-opus,   agent: claude,       model: claude-opus-4-8,   tier: qualified, fallback: [codex-gpt5] }
  - { id: codex-gpt5,    agent: codex,        model: gpt-5,             tier: qualified }
  - { id: codex-mini,    agent: codex,        model: gpt-5-mini,        tier: fast }
  - { id: dev-claude,    agent: claude-code,  model: claude-sonnet-4-6, tier: medium, fallback: [codex-gpt5] }
  - { id: shell,         agent: shell,        tier: fast }
defaults:
  chat: claude-haiku
  research: claude-sonnet
  planning: claude-opus
  development: dev-claude
  verification: shell
role_pins:
  developer: { require_kind: agentic, prefer: dev-claude }
fallback_policy:
  triggers: [unauthorized, unavailable, rate_limited, budget_low, node_down]
  preserve: tier
"#;

    #[test]
    fn parses_profile() {
        let p = BindingProfile::parse(PROFILE).unwrap();
        assert_eq!(p.bindings.len(), 7);
        assert_eq!(p.defaults.get(&Action::Planning).unwrap(), "claude-opus");
        assert_eq!(
            p.binding("dev-claude").unwrap().agent.kind(),
            AgentKind::Agentic
        );
    }

    #[test]
    fn unknown_default_rejected() {
        let y = "bindings: [{id: a, agent: shell, tier: fast}]\ndefaults: {chat: missing}\n";
        assert!(BindingProfile::parse(y).is_err());
    }
}
