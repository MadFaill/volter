//! Общие типы, разделяемые между control-plane, движком и runtime-plane.
//!
//! Пока — минимальный каркас (Ш0). Доменная модель появится в Ш1.

use serde::{Deserialize, Serialize};

/// Роль участника диалога. Расширяется в Ш1+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    User,
    Assistant,
    System,
}

/// Семантическая версия volter (из workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_serializes_snake_case() {
        let json = serde_json::to_string(&Actor::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
