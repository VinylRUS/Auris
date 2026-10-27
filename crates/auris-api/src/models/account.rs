//! Аккаунт и статус подписки.

use serde::{Deserialize, Serialize};

use super::flex;

/// Данные аккаунта Яндекс ID.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Идентификатор пользователя.
    #[serde(default, deserialize_with = "flex::i64")]
    pub uid: i64,
    /// Логин (обычно e-mail).
    #[serde(default)]
    pub login: String,
    /// Полное имя.
    #[serde(default)]
    pub full_name: Option<String>,
    /// Отображаемое имя.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Регион.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub region: Option<i64>,
}

/// Подписка (Плюс/Музыка).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Можно ли начать триал.
    #[serde(default)]
    pub can_start_trial: Option<bool>,
    /// Unix-timestamp окончания подписки.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub until: Option<i64>,
    /// Активна ли подписка сейчас.
    #[serde(default)]
    pub active: Option<bool>,
}

/// Права доступа (используются API как индикатор подписки).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// До какого unix-timestamp действуют права.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub until: Option<i64>,
    /// Активные права (например `library`, `settings`).
    #[serde(default)]
    pub values: Vec<String>,
    /// Права по умолчанию с их сроками.
    #[serde(default)]
    pub default: Vec<PermissionDefault>,
}

/// Элемент списка `permissions.default`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDefault {
    /// До какого unix-timestamp действует право.
    #[serde(default, deserialize_with = "flex::opt_i64")]
    pub until: Option<i64>,
    /// Название права.
    #[serde(default)]
    pub value: Option<String>,
}

/// Ответ `/account/status`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Аккаунт.
    #[serde(default)]
    pub account: Option<Account>,
    /// Подписка.
    #[serde(default)]
    pub subscription: Option<Subscription>,
    /// Права.
    #[serde(default)]
    pub permissions: Option<Permissions>,
}

impl Status {
    /// Есть ли у пользователя действующая подписка (Плюс/Музыка).
    ///
    /// Ориентируется на `subscription.until` / `permissions.until`;
    /// без подписки возвращается `false` — скачивание и полное
    /// прослушивание будет недоступно (так решает сам API).
    pub fn has_subscription(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let sub = self
            .subscription
            .as_ref()
            .and_then(|s| s.until)
            .unwrap_or(0);
        let perm = self.permissions.as_ref().and_then(|p| p.until).unwrap_or(0);

        let until = sub.max(perm);
        until == 0 || until > now
    }

    /// Отображаемое имя (для сайдбара).
    pub fn display_name(&self) -> String {
        self.account
            .as_ref()
            .and_then(|a| {
                a.display_name
                    .clone()
                    .or_else(|| a.full_name.clone())
                    .or_else(|| Some(a.login.clone()))
            })
            .unwrap_or_else(|| "Слушатель".into())
    }

    /// UID аккаунта.
    pub fn uid(&self) -> i64 {
        self.account.as_ref().map(|a| a.uid).unwrap_or(0)
    }
}
