//! Ошибки API-клиента.

use thiserror::Error;

/// Ошибка любого вызова API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Сетевая ошибка / ошибка HTTP-транспорта.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Ошибка сериализации/десериализации JSON.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// API вернул неуспешный HTTP-статус.
    #[error("http {status}: {body}")]
    Http {
        /// HTTP-статус.
        status: reqwest::StatusCode,
        /// Тело ответа (обрезано до 512 символов).
        body: String,
    },

    /// OAuth-ошибка, не связанная с ожиданием подтверждения.
    #[error("oauth error: {0}")]
    OAuth(String),

    /// Не удалось разобрать ответ (неожиданная структура).
    #[error("unexpected response: {0}")]
    Unexpected(String),

    /// Вызов требует авторизации, но токен не задан.
    #[error("not authorized: no OAuth token")]
    NotAuthorized,
}

/// Псевдоним результата API-клиента.
pub type Result<T> = std::result::Result<T, ApiError>;
