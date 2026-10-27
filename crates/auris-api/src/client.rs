//! HTTP-клиент API Яндекс.Музыки.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

use crate::error::{ApiError, Result};
use crate::{API_BASE, OAUTH_BASE};

/// Значение `X-Yandex-Music-Client`, имитирующее официальный клиент
/// (то же значение использует спецификация сообщества).
const YM_CLIENT_HEADER: &str = "YandexMusicAndroid/24023607";

/// Аутентифицированный клиент API.
#[derive(Debug, Clone)]
pub struct Client {
    /// Общий HTTP-клиент (пул соединений).
    pub(crate) http: reqwest::Client,
    /// Базовый URL API.
    pub(crate) base: String,
    /// OAuth-токен.
    pub(crate) token: Option<String>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Создаёт клиент без авторизации.
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Auris/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base: API_BASE.to_string(),
            token: None,
        }
    }

    /// Создаёт клиент с OAuth-токеном.
    pub fn with_token(token: impl Into<String>) -> Self {
        let mut c = Self::new();
        c.token = Some(token.into());
        c
    }

    /// Подменяет токен (после повторного входа).
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    /// Токен задан и не пуст?
    ///
    /// ВАЖНО: пустая строка — не авторизация. Раньше `is_authorized()`
    /// возвращал true для `Some("")` (клиент, созданный через
    /// `with_token("".to_string())`), из-за чего приложение пропускало
    /// экран входа и работало анонимным гостевым аккаунтом.
    pub fn is_authorized(&self) -> bool {
        self.token.as_deref().is_some_and(|t| !t.is_empty())
    }

    /// Доступ к общему `reqwest`-клиенту (для обложек, XML, …).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Yandex-Music-Client",
            HeaderValue::from_static(YM_CLIENT_HEADER),
        );
        if let Some(token) = &self.token {
            let value = HeaderValue::from_str(&format!("OAuth {token}"))
                .map_err(|_| ApiError::OAuth("invalid token characters".into()))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
        Ok(headers)
    }

    /// GET к API с разбором JSON-ответа.
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .query(query)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST к API с пустым телом и разбором JSON-ответа.
    #[allow(dead_code)]
    pub(crate) async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .query(query)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST с JSON-телом.
    pub(crate) async fn post_json<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(body)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// POST с form-телом (повторяющиеся ключи допустимы).
    pub(crate) async fn post_form<T: DeserializeOwned>(
        &self,
        path: &str,
        form: Vec<(String, String)>,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .form(&form)
            .send()
            .await?;
        Self::parse(resp).await
    }

    /// Проверяет статус и разбирает JSON.
    async fn parse<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            let body = if text.len() > 512 {
                format!("{}…", &text[..512])
            } else {
                text
            };
            return Err(ApiError::Http { status, body });
        }
        serde_json::from_str(&text).map_err(|e| {
            let ctx = if text.len() > 256 {
                format!("{}…", &text[..256])
            } else {
                text.clone()
            };
            ApiError::Unexpected(format!("{e}; body: {ctx}"))
        })
    }
}

/// Клиент OAuth Яндекса (Device Authorization Flow).
///
/// Живёт отдельно от [`Client`], так как использует другой базовый URL
/// и не нуждается в заголовках API.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    /// Общий HTTP-клиент.
    pub http: reqwest::Client,
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthClient {
    /// Создаёт OAuth-клиент.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("Auris/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(20))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// POST form на OAuth-эндпоинт, возвращает сырой текст.
    pub(crate) async fn post_form_raw(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> Result<(reqwest::StatusCode, String)> {
        let url = format!("{OAUTH_BASE}{path}");
        // Секрет в журнал не пишем — только факт его наличия.
        let safe: Vec<(&str, &str)> = form
            .iter()
            .map(|(k, v)| (*k, if *k == "client_secret" { "***" } else { *v }))
            .collect();
        log::debug!("oauth POST {path} {safe:?}");
        let resp = self.http.post(&url).form(form).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status.is_success() {
            log::debug!("oauth {path} → {status}");
        } else {
            // Неуспех — на info: heartbeat опроса (authorization_pending)
            // и реальные ошибки OAuth должны попадать в logs/auris.log
            // без включения debug-уровня.
            log::info!(
                "oauth {path} → {status}: {}",
                crate::auth::truncate_chars(&text, 200)
            );
        }
        Ok((status, text))
    }
}
