//! Движок качества (порт модели «нейроцеха» на Rust) — план `plan.md` §6, §7, Ш7.
//!
//! Каркас (Ш0): фиксируем перечень фаз SDLC как точку роста. Контрактная модель
//! (manifest/roles/plan, гейты, feedback-петля) реализуется в Ш7–Ш9.

use serde::{Deserialize, Serialize};

/// Фазы SDLC конвейера качества (см. `plan.md` §2, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Intake,
    Triage,
    Research,
    Requirements,
    WaitingApproval,
    TestModeling,
    TestWriting,
    Development,
    Build,
    Verification,
    DeployStand,
    E2eStand,
    ReleaseVerdict,
    Promotion,
    PostMortem,
}

impl Phase {
    /// Полный конвейер в порядке исполнения.
    pub const PIPELINE: [Phase; 15] = [
        Phase::Intake,
        Phase::Triage,
        Phase::Research,
        Phase::Requirements,
        Phase::WaitingApproval,
        Phase::TestModeling,
        Phase::TestWriting,
        Phase::Development,
        Phase::Build,
        Phase::Verification,
        Phase::DeployStand,
        Phase::E2eStand,
        Phase::ReleaseVerdict,
        Phase::Promotion,
        Phase::PostMortem,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_starts_at_intake_ends_at_post_mortem() {
        assert_eq!(Phase::PIPELINE.first(), Some(&Phase::Intake));
        assert_eq!(Phase::PIPELINE.last(), Some(&Phase::PostMortem));
        assert_eq!(Phase::PIPELINE.len(), 15);
    }
}
