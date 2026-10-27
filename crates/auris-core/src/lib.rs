//! # auris-core
//!
//! Прикладное ядро Auris: настройки, темы, хранилище токена,
//! кеш обложек, менеджер загрузок и локальная история прослушиваний.
//! Все данные приложение держит рядом с исполняемым файлом
//! (см. [`paths`]) — на системный диск не пишем.
//!
//! Крейт не знает ничего о UI — только инфраструктура и асинхронные
//! сервисы, которыми управляет приложение.

#![warn(missing_docs)]

pub mod cache;
pub mod config;
pub mod downloads;
pub mod history;
pub mod paths;
pub mod theme;
pub mod token;
pub mod update;

pub use config::{Config, Quality};
pub use theme::{Theme, ThemePreset};
pub use token::TokenStorage;
