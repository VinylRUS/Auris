//! # auris-api
//!
//! Неофициальный асинхронный клиент публичного API Яндекс.Музыки на чистом Rust.
//!
//! Реализация опирается на открытую спецификацию сообщества
//! (MarshalX/yandex-music-api, LGPL) и повторяет поведение официальных
//! клиентов. Библиотека осознанно **не** реализует обход каких-либо
//! ограничений сервиса: доступны только те данные, которые API отдаёт
//! авторизованному пользователю с действующей подпиской.
//!
//! ## Правовая позиция
//! * никакого разбора/расшифровки DRM-контента (encraw);
//! * скачивание — только по официальным `download-info` ссылкам;
//! * токен получается штатным OAuth Device Authorization Flow.

#![warn(missing_docs)]

pub mod api;
pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod sign;

pub use auth::{DeviceCode, OAuthToken, PollOutcome};
pub use client::{unwrap_result, Client, OAuthClient};
pub use error::{ApiError, Result};
pub use models::*;

/// Базовый URL API Яндекс.Музыки.
pub const API_BASE: &str = "https://api.music.yandex.net";

/// Базовый URL OAuth Яндекса.
pub const OAUTH_BASE: &str = "https://oauth.yandex.ru";
