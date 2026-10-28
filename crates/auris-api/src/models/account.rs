//! Аккаунт и статус подписки.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    ///
    /// Живой API отдаёт их то строками (`["landing-play", …]` —
    /// гостевой аккаунт), то объектами `{until, value}` —
    /// десериализатор принимает обе формы.
    #[serde(default, deserialize_with = "permission_defaults")]
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

/// `permissions.default`: строки или объекты `{until, value}`.
fn permission_defaults<'de, D>(d: D) -> std::result::Result<Vec<PermissionDefault>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<Vec<Value>>::deserialize(d)?.unwrap_or_default();
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        match v {
            Value::String(s) => out.push(PermissionDefault {
                until: None,
                value: Some(s),
            }),
            Value::Object(_) => {
                if let Ok(p) = serde_json::from_value::<PermissionDefault>(v) {
                    out.push(p);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}
