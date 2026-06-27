//! Контрактная модель volter: правила/критерии и нарезка шагов (plan.md §6) и связки (§7).
//!
//! Три слоя §6:
//! - [`manifest`] — канон проекта: адресуемые правила (units) с трассировкой к `PRODUCT.*`;
//! - [`role`] — роли как декларативные правила (`guard`) + критерии (`gate`);
//! - [`plan`] — нарезка задачи на `stages[].steps[]` с контрактом `do`/`verify`/`check`.
//!
//! Слой связок §7: [`bindings`] (профиль `action → binding`, fallback) + [`resolver`]
//! (разрешение связки по приоритету и деградация).
//!
//! Все YAML строго валидируются на Rust (zod-подобно): невалидный артефакт отклоняется до исполнения.

pub mod bindings;
pub mod manifest;
pub mod plan;
pub mod resolver;
pub mod role;

use thiserror::Error;

/// Ошибка валидации контрактного артефакта.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContractError {
    #[error("дублирующийся id: {0}")]
    DuplicateId(String),
    #[error("пустое поле {field} в {context}")]
    Empty { context: String, field: String },
    #[error("требование {unit}: anchor «{anchor}» не прослеживается до PRODUCT.*")]
    AnchorOffProduct { unit: String, anchor: String },
    #[error("план: {0}")]
    Plan(String),
    #[error("ошибка разбора YAML: {0}")]
    Yaml(String),
}

/// Разбор YAML с приведением ошибки к [`ContractError::Yaml`].
pub(crate) fn from_yaml<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, ContractError> {
    serde_yaml::from_str(s).map_err(|e| ContractError::Yaml(e.to_string()))
}
