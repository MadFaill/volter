//! Control-plane API (Ш0/Ш0а): каркас сервиса + закрытый доступ (логин/сессии).
//!
//! Публичны только `/api/health`, `/api/setup/*` и `/api/auth/login|logout`.
//! Всё остальное требует валидной session-cookie (см. [`Auth`]).

pub mod auth;
pub mod store;

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::{header, request::Parts, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::store::{AdminRecord, UserStore};

/// Минимальная длина пароля администратора.
pub const MIN_PASSWORD_LEN: usize = 12;

/// Разделяемое состояние сервиса.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn UserStore>,
    pub jwt_secret: Arc<Vec<u8>>,
}

impl AppState {
    pub fn new(store: Arc<dyn UserStore>, jwt_secret: Vec<u8>) -> Self {
        Self {
            store,
            jwt_secret: Arc::new(jwt_secret),
        }
    }
}

/// Ошибка API → JSON `{ "error": "..." }`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

/// Извлекатель аутентифицированного администратора из session-cookie.
/// Возвращает 401, если cookie отсутствует или токен невалиден/просрочен.
pub struct Auth(pub Claims);

impl<S> FromRequestParts<S> for Auth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        let cookie = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(auth::token_from_cookie_header);
        let token =
            cookie.ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Требуется вход"))?;
        let claims = auth::verify_token(&app.jwt_secret, &token)
            .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Сессия истекла"))?;
        Ok(Auth(claims))
    }
}

/// Собирает роутер API.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/setup/status", get(setup_status))
        .route("/api/setup/complete", post(setup_complete))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "volter-control-plane",
        "version": volter_shared_types::VERSION,
    }))
}

#[derive(Serialize)]
struct SetupStatus {
    needs_setup: bool,
}

async fn setup_status(State(state): State<AppState>) -> Result<Json<SetupStatus>, ApiError> {
    let needs_setup = state.store.admin()?.is_none();
    Ok(Json(SetupStatus { needs_setup }))
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AdminView {
    username: String,
}

async fn setup_complete(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, ApiError> {
    if state.store.admin()?.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "Администратор уже создан",
        ));
    }
    let username = body.username.trim();
    if username.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Укажите логин"));
    }
    if body.password.chars().count() < MIN_PASSWORD_LEN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("Пароль короче {MIN_PASSWORD_LEN} символов"),
        ));
    }
    let password_hash = auth::hash_password(&body.password)?;
    state.store.set_admin(AdminRecord {
        username: username.to_string(),
        password_hash,
    })?;
    logged_in_response(&state, username)
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, ApiError> {
    let invalid = || ApiError::new(StatusCode::UNAUTHORIZED, "Неверный логин или пароль");
    let admin = state.store.admin()?.ok_or_else(invalid)?;
    if admin.username != body.username.trim()
        || !auth::verify_password(&body.password, &admin.password_hash)
    {
        return Err(invalid());
    }
    logged_in_response(&state, &admin.username)
}

fn logged_in_response(state: &AppState, username: &str) -> Result<Response, ApiError> {
    let token = auth::issue_token(&state.jwt_secret, username, auth::SESSION_TTL_SECONDS)?;
    let mut resp = Json(AdminView {
        username: username.to_string(),
    })
    .into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        auth::session_cookie(&token)
            .parse()
            .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cookie"))?,
    );
    Ok(resp)
}

async fn logout() -> Result<Response, ApiError> {
    let mut resp = Json(serde_json::json!({ "ok": true })).into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        auth::clear_cookie()
            .parse()
            .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cookie"))?,
    );
    Ok(resp)
}

async fn me(Auth(claims): Auth) -> impl IntoResponse {
    Json(AdminView {
        username: claims.sub,
    })
}
