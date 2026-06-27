//! План задачи: нарезка на `stages[].steps[]` с контрактом `do`/`verify`/`check` (plan.md §6.3).
//!
//! Детерминированные правила нарезки проверяются здесь — невалидный план отклоняется без LLM.

use serde::{Deserialize, Serialize};

use crate::{from_yaml, ContractError};

/// Класс задачи (из triage) — задаёт скелет стадий.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskClass {
    Bug,
    Feature,
    Content,
    Infra,
    Research,
    Refactor,
}

/// Тип стадии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    Implementation,
    Refactor,
    Scaffold,
    Tests,
    Validation,
    RootCause,
    E2e,
    Docs,
    Deploy,
}

impl StageKind {
    /// Стадии, требующие написания/изменения кода (обязаны называть файлы).
    fn needs_files(self) -> bool {
        matches!(
            self,
            StageKind::Implementation | StageKind::Refactor | StageKind::Scaffold
        )
    }
}

/// Машинная (безмодельная) проверка шага.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Check {
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub expect_contains: String,
    #[serde(default)]
    pub expect_absent: String,
    #[serde(default)]
    pub requires_validator: bool,
    #[serde(default)]
    pub validator_file: String,
}

/// Шаг: что сделать + как проверить (словами и машинно) + какие правила канона закрывает.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    #[serde(rename = "do")]
    pub do_action: String,
    pub verify: String,
    #[serde(default)]
    pub check: Option<Check>,
    /// id правил манифеста, которые шаг удовлетворяет.
    #[serde(default)]
    pub satisfies: Vec<String>,
}

/// Стадия плана.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub id: String,
    pub kind: StageKind,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub done: String,
}

/// План задачи.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub class: TaskClass,
    #[serde(default)]
    pub stages: Vec<Stage>,
}

impl Plan {
    pub fn parse(yaml: &str) -> Result<Self, ContractError> {
        let p: Plan = from_yaml(yaml)?;
        p.validate()?;
        Ok(p)
    }

    /// Детерминированная валидация нарезки (§6.3).
    pub fn validate(&self) -> Result<(), ContractError> {
        let plan_err = |m: String| ContractError::Plan(m);
        if self.stages.is_empty() {
            return Err(plan_err("нет стадий".into()));
        }

        // bug → первая стадия root_cause.
        if self.class == TaskClass::Bug && self.stages[0].kind != StageKind::RootCause {
            return Err(plan_err(
                "класс bug: первая стадия должна быть root_cause".into(),
            ));
        }

        let mut has_code = false;
        let mut has_tests = false;
        for stage in &self.stages {
            if stage.kind.needs_files() {
                has_code = true;
                if stage.files.iter().all(|f| f.trim().is_empty()) {
                    return Err(plan_err(format!(
                        "стадия {}: kind={:?} обязана называть файлы",
                        stage.id, stage.kind
                    )));
                }
            }
            if stage.kind == StageKind::Tests {
                has_tests = true;
            }
            if stage.steps.is_empty() {
                return Err(plan_err(format!("стадия {}: нет шагов", stage.id)));
            }
            for (i, step) in stage.steps.iter().enumerate() {
                let where_ = format!("стадия {} шаг {}", stage.id, i + 1);
                if step.do_action.trim().is_empty() {
                    return Err(plan_err(format!("{where_}: пустое do")));
                }
                if step.verify.trim().is_empty() {
                    return Err(plan_err(format!("{where_}: пустое verify")));
                }
                match &step.check {
                    None => return Err(plan_err(format!("{where_}: нет check"))),
                    Some(c) => {
                        if c.requires_validator {
                            if c.validator_file.trim().is_empty() {
                                return Err(plan_err(format!(
                                    "{where_}: requires_validator без validator_file"
                                )));
                            }
                        } else if c.cmd.trim().is_empty() {
                            return Err(plan_err(format!("{where_}: пустой check.cmd")));
                        }
                    }
                }
            }
        }

        // Есть код-стадии → обязательна tests-стадия.
        if has_code && !has_tests {
            return Err(plan_err(
                "план с код-стадиями обязан включать tests-стадию".into(),
            ));
        }
        Ok(())
    }

    /// Все id правил, которые план обещает закрыть (для contract-coverage).
    pub fn satisfied_units(&self) -> Vec<&str> {
        self.stages
            .iter()
            .flat_map(|s| s.steps.iter())
            .flat_map(|st| st.satisfies.iter())
            .map(String::as_str)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEATURE: &str = r#"
class: feature
stages:
  - id: S1
    kind: tests
    files: [backend/src/domain/teaser_test.rs]
    steps:
      - do: "написать red-тест teaser_returns_by_uuid"
        verify: "тест падает до реализации"
        check: { cmd: "cargo test teaser_returns_by_uuid", expect_contains: "test result" }
        satisfies: [PRODUCT.SCENARIO.public_teaser]
    done: "Контракт теста красный."
  - id: S2
    kind: implementation
    files: [backend/src/domain/teaser.rs]
    steps:
      - do: "реализовать by_uuid"
        verify: "тест зелёный"
        check: { cmd: "cargo test teaser_returns_by_uuid", expect_contains: "ok" }
        satisfies: [PRODUCT.SCENARIO.public_teaser]
    done: "Сценарий проходит."
"#;

    #[test]
    fn valid_feature_plan_parses() {
        let p = Plan::parse(FEATURE).unwrap();
        assert_eq!(p.class, TaskClass::Feature);
        assert_eq!(
            p.satisfied_units(),
            vec!["PRODUCT.SCENARIO.public_teaser"; 2]
        );
    }

    #[test]
    fn impl_without_tests_stage_rejected() {
        let y = r#"
class: feature
stages:
  - id: S1
    kind: implementation
    files: [a.rs]
    steps:
      - {do: "x", verify: "y", check: {cmd: "true"}}
"#;
        assert!(matches!(Plan::parse(y), Err(ContractError::Plan(_))));
    }

    #[test]
    fn impl_without_files_rejected() {
        let y = r#"
class: feature
stages:
  - id: S0
    kind: tests
    steps: [{do: a, verify: b, check: {cmd: "true"}}]
  - id: S1
    kind: implementation
    steps: [{do: a, verify: b, check: {cmd: "true"}}]
"#;
        assert!(matches!(Plan::parse(y), Err(ContractError::Plan(_))));
    }

    #[test]
    fn bug_requires_root_cause_first() {
        let y = r#"
class: bug
stages:
  - id: S1
    kind: tests
    steps: [{do: a, verify: b, check: {cmd: "true"}}]
"#;
        assert!(matches!(Plan::parse(y), Err(ContractError::Plan(_))));
    }

    #[test]
    fn step_without_check_rejected() {
        let y = r#"
class: content
stages:
  - id: S1
    kind: docs
    steps: [{do: a, verify: b}]
"#;
        assert!(matches!(Plan::parse(y), Err(ContractError::Plan(_))));
    }

    #[test]
    fn requires_validator_needs_file() {
        let y = r#"
class: content
stages:
  - id: S1
    kind: docs
    steps: [{do: a, verify: b, check: {requires_validator: true}}]
"#;
        assert!(matches!(Plan::parse(y), Err(ContractError::Plan(_))));
    }
}
