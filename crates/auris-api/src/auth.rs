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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone)]
pub enum PollOutcome {
    /// Пользователь подтвердил вход — вот токен.
    Token(OAuthToken),
    /// Ещё не подтверждено — продолжать опрос.
    Pending,
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
/// * `Err` — код истёк / отменён пользователем / иная ошибка OAuth.
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

    // 400 с error=authorization_pending — норма для Device Flow.
    if !status.is_success() {
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            let err = v
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown_oauth_error");
            if err == "authorization_pending" || err == "slow_down" {
                return Ok(PollOutcome::Pending);
            }
            return Err(ApiError::OAuth(err.to_string()));
        }
        return Err(ApiError::OAuth(format!("token poll failed: {status}")));
    }

    let token: OAuthToken = serde_json::from_str(&text)
        .map_err(|e| ApiError::OAuth(format!("bad token response: {e}")))?;
    Ok(PollOutcome::Token(token))
}
