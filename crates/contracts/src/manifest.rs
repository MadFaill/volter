//! Канон проекта: manifest units = адресуемые правила/критерии (plan.md §6.2).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{from_yaml, ContractError};

/// Слой, к которому относится правило.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Scanner,
    Platform,
    Both,
}

/// Жёсткость правила: блокирует (`hard`) или предупреждает (`soft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Hard,
    #[default]
    Soft,
}

/// Единица канона — одно правило с адресом и трассировкой к продуктовому корню.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestUnit {
    /// Стабильный адрес, напр. `ARCH.LAYER.ddd` или `PRODUCT.SCENARIO.public_teaser`.
    pub id: String,
    pub section: String,
    #[serde(default)]
    pub layer: Option<Layer>,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default)]
    pub tags: Vec<String>,
    pub text: String,
    /// Мост к реальному коду (пути/символы).
    #[serde(default)]
    pub code_anchors: Vec<String>,
    /// Цепочка к корню `PRODUCT.*`.
    #[serde(default)]
    pub anchors_to: Vec<String>,
}

/// Канон проекта: набор units (из `.agent/manifest/<section>.yaml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub units: Vec<ManifestUnit>,
}

impl Manifest {
    pub fn parse(yaml: &str) -> Result<Self, ContractError> {
        let m: Manifest = from_yaml(yaml)?;
        m.validate()?;
        Ok(m)
    }

    fn index(&self) -> HashMap<&str, &ManifestUnit> {
        self.units.iter().map(|u| (u.id.as_str(), u)).collect()
    }

    /// Прослеживается ли `id` до корня `PRODUCT.*` по `anchors_to` (BFS).
    pub fn traces_to_product(&self, id: &str) -> bool {
        let by_id = self.index();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut queue = vec![id];
        while let Some(cur) = queue.pop() {
            if !seen.insert(cur) {
                continue;
            }
            if cur.starts_with("PRODUCT.") {
                return true;
            }
            if let Some(u) = by_id.get(cur) {
                for parent in &u.anchors_to {
                    queue.push(parent.as_str());
                }
            }
        }
        false
    }

    /// Структурная валидация: непустые поля и уникальность id.
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut seen = HashSet::new();
        for u in &self.units {
            if u.id.trim().is_empty() {
                return Err(ContractError::Empty {
                    context: "manifest unit".into(),
                    field: "id".into(),
                });
            }
            if !seen.insert(u.id.as_str()) {
                return Err(ContractError::DuplicateId(u.id.clone()));
            }
            if u.text.trim().is_empty() {
                return Err(ContractError::Empty {
                    context: format!("unit {}", u.id),
                    field: "text".into(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
units:
  - id: PRODUCT.QUALITY.maintainable
    section: product
    text: "Сопровождаемость — продуктовое качество."
  - id: ARCH.LAYER.ddd
    section: architecture
    severity: hard
    tags: [layering, ddd]
    text: "Бэкенд по DDD; зависимости только внутрь."
    code_anchors: ["backend/src/domain"]
    anchors_to: [PRODUCT.QUALITY.maintainable]
"#;

    #[test]
    fn parses_and_traces_to_product() {
        let m = Manifest::parse(SAMPLE).unwrap();
        assert_eq!(m.units.len(), 2);
        assert!(m.traces_to_product("ARCH.LAYER.ddd"));
        assert!(m.traces_to_product("PRODUCT.QUALITY.maintainable"));
    }

    #[test]
    fn orphan_does_not_trace() {
        let m = Manifest::parse(
            "units:\n  - id: ARCH.X\n    section: a\n    text: t\n    anchors_to: [NOWHERE]\n",
        )
        .unwrap();
        assert!(!m.traces_to_product("ARCH.X"));
    }

    #[test]
    fn duplicate_id_rejected() {
        let y = "units:\n  - {id: A, section: s, text: t}\n  - {id: A, section: s, text: t}\n";
        assert!(matches!(
            Manifest::parse(y),
            Err(ContractError::DuplicateId(_))
        ));
    }
}
