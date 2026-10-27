//! # auris-core
//!
//! Прикладное ядро Auris: настройки, темы, хранилище токена,
//! кеш обложек, менеджер загрузок и локальная история прослушиваний.
//!
//! Крейт не знает ничего о UI — только инфраструктура и асинхронные
//! сервисы, которыми управляет приложение.

#![warn(missing_docs)]

pub mod cache;
pub mod config;
pub mod downloads;
pub mod history;
pub mod theme;
pub mod token;

pub use config::{Config, Quality};
pub use theme::{Theme, ThemePreset};
pub use token::TokenStorage;
