//! Сквозной HTTP-тест закрытого доступа (Ш0а DoD):
//! без логина доступны только health/setup/login; полный цикл setup→me→login→logout.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use volter_control_plane::store::MemoryUserStore;
use volter_control_plane::{build_router, AppState};

fn app() -> axum::Router {
    let state = AppState::new(
        Arc::new(MemoryUserStore::default()),
        b"test-secret".to_vec(),
    );
    build_router(state)
}

async fn send(router: &axum::Router, req: Request<Body>) -> (StatusCode, Option<String>, Value) {
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, cookie, body)
}

fn get(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

fn get_with_cookie(path: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

fn post_json(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

/// Извлекает `name=value` из заголовка Set-Cookie для использования в Cookie.
fn cookie_pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

#[tokio::test]
async fn full_auth_lifecycle() {
    let app = app();

    // health публичен
    let (status, _, body) = send(&app, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    // до setup — нужен setup
    let (_, _, body) = send(&app, get("/api/setup/status")).await;
    assert_eq!(body["needs_setup"], true);

    // защищённый /me без cookie → 401
    let (status, _, _) = send(&app, get("/api/auth/me")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // короткий пароль отклоняется
    let (status, _, _) = send(
        &app,
        post_json(
            "/api/setup/complete",
            json!({"username":"admin","password":"short"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // валидный setup → 200 + cookie
    let (status, cookie, body) = send(
        &app,
        post_json(
            "/api/setup/complete",
            json!({"username":"admin","password":"correct horse battery"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "admin");
    let session = cookie_pair(&cookie.expect("set-cookie on setup"));
    assert!(session.starts_with("volter_session="));

    // setup уже не нужен
    let (_, _, body) = send(&app, get("/api/setup/status")).await;
    assert_eq!(body["needs_setup"], false);

    // повторный setup → 409
    let (status, _, _) = send(
        &app,
        post_json(
            "/api/setup/complete",
            json!({"username":"x","password":"another password!"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // /me с cookie → 200
    let (status, _, body) = send(&app, get_with_cookie("/api/auth/me", &session)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "admin");

    // неверный пароль → 401
    let (status, _, _) = send(
        &app,
        post_json(
            "/api/auth/login",
            json!({"username":"admin","password":"nope"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // верный логин → 200 + cookie
    let (status, cookie, _) = send(
        &app,
        post_json(
            "/api/auth/login",
            json!({"username":"admin","password":"correct horse battery"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let session2 = cookie_pair(&cookie.expect("set-cookie on login"));

    // logout очищает cookie
    let (status, cookie, _) = send(&app, post_json("/api/auth/logout", json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(cookie.unwrap().contains("Max-Age=0"));

    // мусорный токен → 401
    let (status, _, _) = send(
        &app,
        get_with_cookie("/api/auth/me", "volter_session=garbage"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // свежая сессия после логина всё ещё валидна
    let (status, _, _) = send(&app, get_with_cookie("/api/auth/me", &session2)).await;
    assert_eq!(status, StatusCode::OK);
}
