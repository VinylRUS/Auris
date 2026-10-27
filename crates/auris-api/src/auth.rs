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
pub const CLIENT_SECRET: &str = "53bc75238f0c4d08a118a51fe9203300";

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
        return Err(ApiError::OAuth(format!(
            "device code failed: {status} {text}"
        )));
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
            return Err(ApiError::OAuth(err.to_string()));
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
}
