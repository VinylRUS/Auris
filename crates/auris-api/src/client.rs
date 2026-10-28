//! HTTP-клиент API Яндекс.Музыки.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde_json::Value;

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

    /// Проверяет статус и разбирает JSON-ответ.
    ///
    /// API заворачивает полезную нагрузку в `{"result": …,
    /// "invocationInfo": …}` — сначала разворачиваем обёртку
    /// (см. [`unwrap_result`]), затем разбираем `T` из полезной нагрузки.
    async fn parse<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(ApiError::Http {
                status,
                body: readable_error_body(&text),
            });
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| {
            let ctx = crate::auth::truncate_chars(&text, 256);
            ApiError::Unexpected(format!("{e}; body: {ctx}"))
        })?;
        serde_json::from_value(unwrap_result(v.clone()))
            .map_err(|e| ApiError::Unexpected(format!("{e}; payload: {}", payload_context(&v))))
    }
}

/// Контекст для ошибки разбора: первые ключи полезной нагрузки —
/// без длинного дампа, чтобы журнал оставался читаемым.
fn payload_context(v: &Value) -> String {
    let keys: Vec<&str> = v
        .as_object()
        .map(|o| o.keys().take(8).map(String::as_str).collect())
        .unwrap_or_default();
    if v.is_array() {
        format!("array[{}]", v.as_array().map(|a| a.len()).unwrap_or(0))
    } else {
        format!("object{{{}}}", keys.join(", "))
    }
}

/// Разворачивает обёртку API `{"result": …}`.
///
/// Все эндпоинты `api.music.yandex.net` отдают полезную нагрузку
/// внутри поля `result`; ошибки приходят на верхнем уровне
/// (`{"error": …}`). Правило зеркалит эталонную библиотеку
/// сообщества (MarshalX/yandex-music-api): поле `result` есть и
/// не null — полезная нагрузка в нём; иначе — используем весь
/// ответ как полезную нагрузку.
pub fn unwrap_result(v: Value) -> Value {
    match v.get("result") {
        Some(result) if !result.is_null() => result.clone(),
        _ => v,
    }
}

/// Человекочитаемое тело ошибки.
///
/// Ошибки API приходят как `{"error": {"name": …, "message": …}}`
/// или `{"error": "код"}` — достаём их; иначе урезаем сырой текст
/// до 256 символов, не разрывая UTF-8 (байтовый срез по границе
/// символа паникует на кириллице и убивает async-задачу молча).
fn readable_error_body(text: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        match v.get("error") {
            Some(Value::String(s)) if !s.is_empty() => return s.clone(),
            Some(Value::Object(o)) => {
                let name = o.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let message = o.get("message").and_then(|x| x.as_str()).unwrap_or("");
                let joined = match (name.is_empty(), message.is_empty()) {
                    (false, false) => format!("{name}: {message}"),
                    (false, true) => name.to_string(),
                    _ => String::new(),
                };
                if !joined.is_empty() {
                    return joined;
                }
            }
            _ => {}
        }
    }
    crate::auth::truncate_chars(text, 256).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Регрессия 0.0.1a–0.0.1d («данные не подтягиваются»): API
    /// заворачивает всё в `result`, а фикстуры моделей писались по
    /// внутренней форме — без разворачивания serde молча подставлял
    /// дефолты и все данные приходили пустыми.
    #[test]
    fn unwraps_result_envelope() {
        let wrapped = serde_json::json!({
            "invocationInfo": {"req-id": "123"},
            "result": {"uid": 42, "login": "user@ya.ru"}
        });
        let payload = unwrap_result(wrapped);
        assert_eq!(payload.get("uid").and_then(|v| v.as_i64()), Some(42));
        assert!(payload.get("invocationInfo").is_none());
    }

    /// Ответ без `result` (массив, скаляр, ошибка) проходит как есть —
    /// так же поступает эталонная библиотека сообщества.
    #[test]
    fn passes_through_without_result() {
        let arr = serde_json::json!([{"id": 1}, {"id": 2}]);
        assert!(unwrap_result(arr.clone()) == arr);
        let plain = serde_json::json!({"error": "not-found"});
        assert!(unwrap_result(plain.clone()) == plain);
        let null_result = serde_json::json!({"result": null, "x": 1});
        assert!(unwrap_result(null_result.clone()) == null_result);
    }

    /// Полный прогон «тело ответа → модель»: обёртка разворачивается,
    /// модель наполняется реальными полями, а не дефолтами.
    #[test]
    fn parses_status_from_wrapped_response() {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Probe {
            #[serde(default)]
            account: Option<serde_json::Value>,
        }
        let body =
            r#"{"invocationInfo":{"req-id":"1"},"result":{"account":{"uid":7,"region":225}}}"#;
        let v: Value = serde_json::from_str(body).unwrap();
        let p: Probe = serde_json::from_value(unwrap_result(v)).unwrap();
        let account = p.account.expect("account must be inside result");
        assert_eq!(account.get("uid").and_then(|x| x.as_i64()), Some(7));
    }

    /// Тело ошибки достаётся из `error.name/message`, а не валится
    /// сырым JSON; кириллица в теле не рвёт срез по байтам.
    #[test]
    fn extracts_error_body_safely() {
        let geo = r#"{"invocationInfo":{},"error":{"name":"Unavailable For Legal Reasons","message":""}}"#;
        assert_eq!(readable_error_body(geo), "Unavailable For Legal Reasons");
        let coded = r#"{"error":"not-found"}"#;
        assert_eq!(readable_error_body(coded), "not-found");
        // Многобайтовый UTF-8 на границе усечения — не паника.
        let cyrillic = "П".repeat(600);
        assert_eq!(readable_error_body(&cyrillic).chars().count(), 256);
    }
}
