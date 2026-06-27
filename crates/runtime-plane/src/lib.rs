//! Runtime-plane — абстракция исполнения стадий (план `plan.md` §4, Ш2).
//!
//! Здесь описан только интерфейс и локальная заглушка (Ш0). Реальное исполнение
//! (local-режим, затем удалённый gRPC/HTTP) приходит в Ш2–Ш3.

use async_trait::async_trait;

/// Запрос на подготовку рабочего пространства под run.
#[derive(Debug, Clone)]
pub struct WorkspaceRequest {
    pub run_id: String,
}

/// Готовое рабочее пространство.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub run_id: String,
    pub path: String,
}

/// Единый интерфейс исполнения. Реализации: local (Ш2), remote/mTLS (Ш3).
#[async_trait]
pub trait RuntimePlane: Send + Sync {
    async fn prepare_workspace(&self, req: WorkspaceRequest) -> anyhow::Result<Workspace>;
}

/// Заглушка для каркаса/тестов: отдаёт детерминированный путь, ничего не пишет на диск.
#[derive(Debug, Default, Clone)]
pub struct NoopRuntimePlane;

#[async_trait]
impl RuntimePlane for NoopRuntimePlane {
    async fn prepare_workspace(&self, req: WorkspaceRequest) -> anyhow::Result<Workspace> {
        Ok(Workspace {
            path: format!("/tmp/volter/runtime-workspaces/run-{}", req.run_id),
            run_id: req.run_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_plane_returns_run_scoped_path() {
        let plane = NoopRuntimePlane;
        let ws = plane
            .prepare_workspace(WorkspaceRequest {
                run_id: "abc".into(),
            })
            .await
            .unwrap();
        assert_eq!(ws.run_id, "abc");
        assert!(ws.path.ends_with("run-abc"));
    }
}
