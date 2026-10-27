//! OAuth Device Authorization Flow (вход без пароля).
//!
//! 1. [`request_device_code`] — приложение получает `user_code` и
//!    `verification_url`, пользователь открывает страницу и подтверждает вход;
//! 2. [`poll_token`] — приложение с интервалом опрашивает `/token`,
//!    пока пользователь не подтвердит (или код не истечёт).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::OAuthClient;
use crate::error::{ApiError, Result};

/// Публичные креды Android-приложения Яндекс.Музыки
/// (те же значения использует спецификация сообщества).
pub const CLIENT_ID: &str = "23cabbbdc6cd418abb4b39c32c41195d";
/// Секрет приложения.
///
/// РЕГРЕССИЯ 0.0.1a–0.0.1c: в секрете была опечатка — `…118a51f…`
/// вместо `…118e51f…`. Яндекс на любой grant type отвечал
/// `invalid_client: Wrong client secret`, поэтому вход не работал вовсе
/// (и в песочнице, и у пользователей из РФ — гео-блок был ни при чём).
/// Значение сверено с MarshalX/yandex-music-api и живым запросом
/// к `oauth.yandex.ru` (исправленный секрет даёт `authorization_pending`).
pub const CLIENT_SECRET: &str = "53bc75238f0c4d08a118e51fe9203300";

/// Ответ `POST /device/code`.
///
/// Яндекс отвечает в snake_case (`device_code`, `user_code`, …) —
/// имена полей структуры совпадают с ответом один в один.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    /// Непрозрачный код устройства для опроса токена.
    pub device_code: String,
    /// Код, который пользователь вводит на странице подтверждения.
    pub user_code: String,
    /// Страница подтверждения (уже содержит код).
    pub verification_url: String,
    /// Срок жизни `device_code`, с.
    pub expires_in: u64,
    /// Рекомендованный интервал опроса, с.
    pub interval: u64,
}

/// OAuth-токен.
///
/// Ответ `/token` — тоже snake_case (`access_token`, `refresh_token`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Токен доступа.
    pub access_token: String,
    /// Токен обновления (может отсутствовать).
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Срок жизни, с.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Тип токена (обычно `bearer`).
    #[serde(default)]
    pub token_type: Option<String>,
}

/// Результат опроса токена.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollOutcome {
    /// Пользователь подтвердил вход — вот токен.
    Token(OAuthToken),
    /// Ещё не подтверждено — продолжать опрос.
    Pending,
    /// Сервер просит опрашивать реже (RFC 8628: интервал +5 c).
    SlowDown,
    /// `device_code` истёк — нужен новый код.
    Expired,
    /// Пользователь отклонил вход на странице подтверждения.
    Denied,
}

impl PollOutcome {
    /// Разбирает поле `error` из ответа 400 `/token`.
    ///
    /// Ошибки Device Flow описаны в RFC 8628 §3.5; Яндекс их и возвращает.
    /// `None` — неизвестная ошибка (её обработает вызывающий как `Err`).
    pub fn from_error(error: &str) -> Option<Self> {
        match error {
            "authorization_pending" => Some(PollOutcome::Pending),
            "slow_down" => Some(PollOutcome::SlowDown),
            "expired_token" => Some(PollOutcome::Expired),
            "access_denied" => Some(PollOutcome::Denied),
            _ => None,
        }
    }
}

/// Человекочитаемая ошибка OAuth: статус + `error: error_description`.
///
/// Яндекс кладёт описание прямо в JSON (например,
/// `invalid_client: Wrong client secret`) — показываем его целиком:
/// оно попадает и на экран ошибки, и в журнал.
pub(crate) fn oauth_error(status: reqwest::StatusCode, text: &str) -> String {
    let parsed = serde_json::from_str::<Value>(text).ok().and_then(|v| {
        let err = v.get("error").and_then(|e| e.as_str());
        let desc = v.get("error_description").and_then(|d| d.as_str());
        match (err, desc) {
            (Some(e), Some(d)) => Some(format!("{e}: {d}")),
            (Some(e), None) => Some(e.to_string()),
            _ => None,
        }
    });
    match parsed {
        Some(msg) => format!("{status} {msg}"),
        // Не-JSON тело (HTML прокси, пусто) — урезаем для читаемости.
        None => format!("{status} {}", truncate_chars(text, 256)),
    }
}

/// Урезает строку до `n` символов, не разрывая UTF-8.
pub(crate) fn truncate_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// Запрашивает код устройства.
///
/// `device_id` — случайная строка (генерируется приложением),
/// `device_name` — человекочитаемое имя устройства.
pub async fn request_device_code(
    oauth: &OAuthClient,
    device_id: &str,
    device_name: &str,
) -> Result<DeviceCode> {
    let (status, text) = oauth
        .post_form_raw(
            "/device/code",
            &[
                ("client_id", CLIENT_ID),
                ("device_id", device_id),
                ("device_name", device_name),
            ],
        )
        .await?;

    if !status.is_success() {
        return Err(ApiError::OAuth(oauth_error(status, &text)));
    }
    serde_json::from_str(&text)
        .map_err(|e| ApiError::OAuth(format!("bad device code response: {e}")))
}

/// Однократный опрос статуса подтверждения.
///
/// * `Ok(PollOutcome::Token(_))` — вход подтверждён;
/// * `Ok(PollOutcome::Pending)` — ждём дальше;
/// * `Ok(PollOutcome::SlowDown)` — сервер просит опрашивать реже;
/// * `Ok(PollOutcome::Expired)` — код истёк, нужен новый;
/// * `Ok(PollOutcome::Denied)` — пользователь отклонил вход;
/// * `Err` — сетевая ошибка или неизвестная ошибка OAuth.
pub async fn poll_token(oauth: &OAuthClient, device_code: &str) -> Result<PollOutcome> {
    let (status, text) = oauth
        .post_form_raw(
            "/token",
            &[
                ("grant_type", "device_code"),
                ("code", device_code),
                ("client_id", CLIENT_ID),
                ("client_secret", CLIENT_SECRET),
            ],
        )
        .await?;

    // 400 с error=… — норма для Device Flow (RFC 8628 §3.5).
    if !status.is_success() {
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            let err = v
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown_oauth_error");
            if let Some(outcome) = PollOutcome::from_error(err) {
                return Ok(outcome);
            }
            // Неизвестная ошибка OAuth — забираем и описание сервера:
            // именно оно отличает «Wrong client secret» от сетевой заминки.
            let desc = v
                .get("error_description")
                .and_then(|d| d.as_str())
                .unwrap_or_default();
            let msg = if desc.is_empty() {
                err.to_string()
            } else {
                format!("{err}: {desc}")
            };
            return Err(ApiError::OAuth(msg));
        }
        return Err(ApiError::OAuth(format!("token poll failed: {status}")));
    }

    let token: OAuthToken = serde_json::from_str(&text)
        .map_err(|e| ApiError::OAuth(format!("bad token response: {e}")))?;
    Ok(PollOutcome::Token(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 8628 §3.5: известные коды ошибок опроса должны превращаться
    /// в типизированные исходы, неизвестные — оставаться ошибками,
    /// чтобы приложение могло решить, ретраить их или показать пользователю.
    #[test]
    fn poll_error_mapping() {
        assert_eq!(
            PollOutcome::from_error("authorization_pending"),
            Some(PollOutcome::Pending)
        );
        assert_eq!(
            PollOutcome::from_error("slow_down"),
            Some(PollOutcome::SlowDown)
        );
        assert_eq!(
            PollOutcome::from_error("expired_token"),
            Some(PollOutcome::Expired)
        );
        assert_eq!(
            PollOutcome::from_error("access_denied"),
            Some(PollOutcome::Denied)
        );
        assert_eq!(PollOutcome::from_error("invalid_client"), None);
        assert_eq!(PollOutcome::from_error("server_error"), None);
        assert_eq!(PollOutcome::from_error(""), None);
    }

    /// Регрессия входа 0.0.1a–0.0.1c: одна опечатанная буква в секрете —
    /// и любой grant type падает с `invalid_client: Wrong client secret`.
    /// Эталон — креды официального Android-приложения из спецификации
    /// сообщества (MarshalX/yandex-music-api, v3.2.2).
    #[test]
    fn client_credentials_are_exact() {
        assert_eq!(CLIENT_ID, "23cabbbdc6cd418abb4b39c32c41195d");
        assert_eq!(CLIENT_SECRET, "53bc75238f0c4d08a118e51fe9203300");
    }

    /// Ошибки OAuth несут описание сервера — его видно в UI и в журнале.
    #[test]
    fn oauth_errors_carry_description() {
        let status = reqwest::StatusCode::BAD_REQUEST;
        assert_eq!(
            oauth_error(
                status,
                r#"{"error":"invalid_client","error_description":"Wrong client secret"}"#
            ),
            "400 Bad Request invalid_client: Wrong client secret"
        );
        assert_eq!(
            oauth_error(status, r#"{"error":"server_error"}"#),
            "400 Bad Request server_error"
        );
        assert!(oauth_error(status, "<html>proxy</html>").starts_with("400 Bad Request <html"));
    }
}
