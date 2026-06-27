//! Аутентификация: хеширование пароля (Argon2), компактный HS256-JWT и cookie-хелперы.
//!
//! JWT реализован вручную на HMAC-SHA256, чтобы не тянуть тяжёлые C-зависимости
//! (`ring`) на этапе каркаса. Контракт совместим с обычным JWT (header.payload.sig).

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

/// Имя session-cookie.
pub const SESSION_COOKIE: &str = "volter_session";
/// Срок жизни сессии — 7 дней.
pub const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Хеширует пароль Argon2id. Возвращает строку PHC.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash: {e}"))?;
    Ok(hash.to_string())
}

/// Проверяет пароль против PHC-хеша.
pub fn verify_password(password: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Полезная нагрузка токена.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// subject — имя администратора.
    pub sub: String,
    /// expiry, unix-секунды.
    pub exp: u64,
    /// issued-at, unix-секунды.
    pub iat: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtHeader {
    alg: &'static str,
    typ: &'static str,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sign(secret: &[u8], message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts any key length");
    mac.update(message.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// Кодирует HS256-JWT для администратора `sub`, действительный `ttl` секунд.
pub fn issue_token(secret: &[u8], sub: &str, ttl: u64) -> anyhow::Result<String> {
    let iat = now_secs();
    let claims = Claims {
        sub: sub.to_string(),
        iat,
        exp: iat + ttl,
    };
    let header = JwtHeader {
        alg: "HS256",
        typ: "JWT",
    };
    let h = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
    let p = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let signing_input = format!("{h}.{p}");
    let s = sign(secret, &signing_input);
    Ok(format!("{signing_input}.{s}"))
}

/// Проверяет подпись и срок жизни токена, возвращает claims.
pub fn verify_token(secret: &[u8], token: &str) -> Option<Claims> {
    let mut parts = token.splitn(3, '.');
    let h = parts.next()?;
    let p = parts.next()?;
    let s = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let expected = sign(secret, &format!("{h}.{p}"));
    // сравнение постоянного времени
    if !constant_time_eq(expected.as_bytes(), s.as_bytes()) {
        return None;
    }
    let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(p).ok()?).ok()?;
    if claims.exp <= now_secs() {
        return None;
    }
    Some(claims)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Строит значение заголовка `Set-Cookie` для сессии.
pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}"
    )
}

/// Строит `Set-Cookie`, очищающий сессию.
pub fn clear_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

/// Достаёт токен сессии из заголовка `Cookie`.
pub fn token_from_cookie_header(header: &str) -> Option<String> {
    header.split(';').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        if k.trim() == SESSION_COOKIE {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let phc = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &phc));
        assert!(!verify_password("wrong", &phc));
    }

    #[test]
    fn token_roundtrip_and_subject() {
        let secret = b"test-secret";
        let token = issue_token(secret, "admin", 60).unwrap();
        let claims = verify_token(secret, &token).expect("valid");
        assert_eq!(claims.sub, "admin");
    }

    #[test]
    fn token_rejected_with_wrong_secret() {
        let token = issue_token(b"a", "admin", 60).unwrap();
        assert!(verify_token(b"b", &token).is_none());
    }

    #[test]
    fn token_rejected_when_expired() {
        let token = issue_token(b"s", "admin", 0).unwrap();
        // exp == iat, и проверка exp <= now → отклоняется.
        assert!(verify_token(b"s", &token).is_none());
    }

    #[test]
    fn token_rejected_when_tampered() {
        let secret = b"s";
        let token = issue_token(secret, "admin", 60).unwrap();
        let tampered = format!("{token}x");
        assert!(verify_token(secret, &tampered).is_none());
    }

    #[test]
    fn cookie_parse_extracts_session() {
        let h = "other=1; volter_session=abc.def.ghi; x=2";
        assert_eq!(token_from_cookie_header(h).as_deref(), Some("abc.def.ghi"));
    }

    #[test]
    fn cookie_set_is_httponly() {
        assert!(session_cookie("t").contains("HttpOnly"));
        assert!(clear_cookie().contains("Max-Age=0"));
    }
}
