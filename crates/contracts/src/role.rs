//! Роли как декларативные правила (`guard`) + критерии (`gate`) (plan.md §6.2).
//!
//! Роль — данные, не код: движок собирает промпт из `prompt_template` + срез манифеста по
//! `manifest_sections` + код по anchors + feedback гейта.

use serde::{Deserialize, Serialize};

use crate::{from_yaml, ContractError};

/// Как роль получает код-реальность.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeAccess {
    Search,
    Anchored,
}

/// Границы актора (separation of powers, §6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    pub does: String,
    pub must_not: String,
}

/// Декларативная роль.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub purpose: String,
    #[serde(default)]
    pub manifest_sections: Vec<String>,
    #[serde(default)]
    pub code_access: Option<CodeAccess>,
    pub guard: Guard,
    /// Критерии (валидаторы), которые обязана пройти роль.
    #[serde(default)]
    pub gate: Vec<String>,
    #[serde(default)]
    pub prompt_template: Option<String>,
}

impl Role {
    pub fn parse(yaml: &str) -> Result<Self, ContractError> {
        let r: Role = from_yaml(yaml)?;
        r.validate()?;
        Ok(r)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        let empty = |field: &str| ContractError::Empty {
            context: format!("role {}", self.id),
            field: field.into(),
        };
        if self.id.trim().is_empty() {
            return Err(ContractError::Empty {
                context: "role".into(),
                field: "id".into(),
            });
        }
        if self.purpose.trim().is_empty() {
            return Err(empty("purpose"));
        }
        if self.guard.does.trim().is_empty() {
            return Err(empty("guard.does"));
        }
        if self.guard.must_not.trim().is_empty() {
            return Err(empty("guard.must_not"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCHITECT: &str = r#"
id: architect
purpose: "Привязать каждое требование к реальным символам кода."
manifest_sections: [architecture, core]
code_access: anchored
guard:
  does: "Привязывает каждое требование к реальным символам."
  must_not: "Менять anchor, состав или продуктовый смысл требований."
gate: [symbol-contract, product-anchored]
prompt_template: roles/architect.md
"#;

    #[test]
    fn parses_declarative_role() {
        let r = Role::parse(ARCHITECT).unwrap();
        assert_eq!(r.id, "architect");
        assert_eq!(r.code_access, Some(CodeAccess::Anchored));
        assert_eq!(r.gate, vec!["symbol-contract", "product-anchored"]);
    }

    #[test]
    fn guard_required() {
        let y = "id: x\npurpose: p\nguard:\n  does: ''\n  must_not: 'no'\n";
        assert!(matches!(Role::parse(y), Err(ContractError::Empty { .. })));
    }
}
