//! Темы оформления.
//!
//! Дизайн-токены повторяют мокапы Win11 Fluent (тёмная тема по умолчанию).
//! Помимо пресетов поддерживаются пастельные тёплые, мятные и монохромные
//! схемы, а также полная кастомизация акцентного цвета.

use serde::{Deserialize, Serialize};

/// Цвет в формате `#RRGGBB`/`#AARRGGBB` (для сериализации в конфиг).
pub type Hex = String;

/// Полный набор дизайн-токенов темы.
///
/// Поля соответствуют CSS-переменным мокапов, поэтому UI в Slint
/// переносит их один-в-один.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Theme {
    /// Фон окна (Mica base).
    pub bg: Hex,
    /// Глубокий фон (плеер-полоса, шапки).
    pub bg_deep: Hex,
    /// Фон карточек.
    pub card: Hex,
    /// Заливка контролов (кнопки, чипы).
    pub fill: Hex,
    /// Заливка при наведении.
    pub fill_hover: Hex,
    /// Обводка контролов.
    pub stroke: Hex,
    /// Обводка «сильная» (нижняя грань полей ввода).
    pub stroke_strong: Hex,
    /// Основной текст.
    pub text: Hex,
    /// Вторичный текст.
    pub text_2: Hex,
    /// Третичный текст (подписи, время).
    pub text_3: Hex,
    /// Акцент (ссылки, прогресс, выделение).
    pub accent: Hex,
    /// Акцент «глубокий» (заливка основных кнопок).
    pub accent_deep: Hex,
    /// Текст на акценте (белый или чёрный — зависит от яркости).
    pub text_on_accent: Hex,
    /// Полупрозрачная акцентная заливка (выделение пунктов).
    pub accent_fill: Hex,
    /// Текст на акцентной заливке.
    pub text_on_accent_fill: Hex,
}

impl Default for Theme {
    fn default() -> Self {
        Self::fluent_dark()
    }
}

/// Идентификатор пресета.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePreset {
    /// Ключ пресета (`fluent-dark`).
    pub id: &'static str,
    /// Человекочитаемое название.
    pub title: &'static str,
}

/// Доступные пресеты (порядок = порядок в галерее настроек).
pub const PRESETS: &[ThemePreset] = &[
    ThemePreset {
        id: "fluent-dark",
        title: "Fluent · Тёмная",
    },
    ThemePreset {
        id: "fluent-light",
        title: "Fluent · Светлая",
    },
    ThemePreset {
        id: "pastel-warm",
        title: "Тёплая пастель",
    },
    ThemePreset {
        id: "pastel-mint",
        title: "Мятная пастель",
    },
    ThemePreset {
        id: "mono-dark",
        title: "Монохром · Тёмная",
    },
    ThemePreset {
        id: "mono-light",
        title: "Монохром · Светлая",
    },
];

impl Theme {
    /// Win11 Fluent, тёмная (как на мокапах).
    pub fn fluent_dark() -> Self {
        Self {
            bg: "#202020".into(),
            bg_deep: "#1B1B1B".into(),
            card: "#2A2A2A".into(),
            fill: "#0CFFFFFF".into(), // ARGB: alpha 0x0C ≈ 5% белого
            fill_hover: "#16FFFFFF".into(),
            stroke: "#13FFFFFF".into(),
            stroke_strong: "#1FFFFFFF".into(),
            text: "#FFFFFF".into(),
            text_2: "#C8FFFFFF".into(),
            text_3: "#8BFFFFFF".into(),
            accent: "#4CC2FF".into(),
            accent_deep: "#0067C0".into(),
            text_on_accent: "#FFFFFF".into(),
            accent_fill: "#244CC2FF".into(),
            text_on_accent_fill: "#C3E9FF".into(),
        }
    }

    /// Win11 Fluent, светлая.
    pub fn fluent_light() -> Self {
        Self {
            bg: "#F3F3F3".into(),
            bg_deep: "#EAEAEA".into(),
            card: "#FBFBFB".into(),
            fill: "#0D000000".into(),
            fill_hover: "#16000000".into(),
            stroke: "#12000000".into(),
            stroke_strong: "#1F000000".into(),
            text: "#1A1A1A".into(),
            text_2: "#BF000000".into(),
            text_3: "#85000000".into(),
            accent: "#0067C0".into(),
            accent_deep: "#0067C0".into(),
            text_on_accent: "#FFFFFF".into(),
            accent_fill: "#1A0067C0".into(),
            text_on_accent_fill: "#005EA6".into(),
        }
    }

    /// Тёплая пастель: кремовый фон, терракотовый акцент.
    pub fn pastel_warm() -> Self {
        Self {
            bg: "#F6EDE3".into(),
            bg_deep: "#F1E4D6".into(),
            card: "#FDFAF5".into(),
            fill: "#0F8A6B4F".into(),
            fill_hover: "#178A6B4F".into(),
            stroke: "#1A8A6B4F".into(),
            stroke_strong: "#2A8A6B4F".into(),
            text: "#3E332B".into(),
            text_2: "#B84A3F35".into(),
            text_3: "#804A3F35".into(),
            accent: "#C2725A".into(),
            accent_deep: "#C2725A".into(),
            text_on_accent: "#FFF6EF".into(),
            accent_fill: "#26C2725A".into(),
            text_on_accent_fill: "#A05A3C".into(),
        }
    }

    /// Мятная пастель: холодный зелёно-голубой.
    pub fn pastel_mint() -> Self {
        Self {
            bg: "#ECF3F0".into(),
            bg_deep: "#E4EEE9".into(),
            card: "#FAFEFC".into(),
            fill: "#0F51796B".into(),
            fill_hover: "#1751796B".into(),
            stroke: "#1A51796B".into(),
            stroke_strong: "#2A51796B".into(),
            text: "#26332E".into(),
            text_2: "#B826332E".into(),
            text_3: "#8026332E".into(),
            accent: "#4E8D77".into(),
            accent_deep: "#4E8D77".into(),
            text_on_accent: "#F2FBF7".into(),
            accent_fill: "#264E8D77".into(),
            text_on_accent_fill: "#3A6B58".into(),
        }
    }

    /// Монохром тёмный: чистые градации серого, белый акцент.
    pub fn mono_dark() -> Self {
        Self {
            bg: "#0D0D0D".into(),
            bg_deep: "#090909".into(),
            card: "#161616".into(),
            fill: "#0FFFFFFF".into(),
            fill_hover: "#16FFFFFF".into(),
            stroke: "#14FFFFFF".into(),
            stroke_strong: "#23FFFFFF".into(),
            text: "#F2F2F2".into(),
            text_2: "#B4B4B4".into(),
            text_3: "#7A7A7A".into(),
            accent: "#E8E8E8".into(),
            accent_deep: "#E8E8E8".into(),
            text_on_accent: "#111111".into(),
            accent_fill: "#1FE8E8E8".into(),
            text_on_accent_fill: "#F2F2F2".into(),
        }
    }

    /// Монохром светлый: бумажно-белый, чёрный акцент.
    pub fn mono_light() -> Self {
        Self {
            bg: "#FAFAFA".into(),
            bg_deep: "#F0F0F0".into(),
            card: "#FFFFFF".into(),
            fill: "#0F000000".into(),
            fill_hover: "#16000000".into(),
            stroke: "#14000000".into(),
            stroke_strong: "#23000000".into(),
            text: "#171717".into(),
            text_2: "#BA171717".into(),
            text_3: "#7F171717".into(),
            accent: "#1A1A1A".into(),
            accent_deep: "#1A1A1A".into(),
            text_on_accent: "#FFFFFF".into(),
            accent_fill: "#141A1A1A".into(),
            text_on_accent_fill: "#1A1A1A".into(),
        }
    }

    /// Тема по ключу пресета (неизвестный ключ → тёмная Fluent).
    pub fn by_preset(id: &str) -> Self {
        match id {
            "fluent-light" => Self::fluent_light(),
            "pastel-warm" => Self::pastel_warm(),
            "pastel-mint" => Self::pastel_mint(),
            "mono-dark" => Self::mono_dark(),
            "mono-light" => Self::mono_light(),
            _ => Self::fluent_dark(),
        }
    }

    /// Применяет пользовательский акцент: обновляет `accent`,
    /// `accent_deep`, `accent_fill` и согласует контраст текста.
    pub fn apply_accent(&mut self, hex: &str) {
        let Some(rgb) = parse_hex(hex) else { return };
        let (r, g, b) = rgb;

        self.accent = format!("#{r:02X}{g:02X}{b:02X}");

        // Кнопка: акцент чуть темнее (или сам акцент, если он тёмный).
        let (dr, dg, db) = darken(r, g, b, 0.82);
        self.accent_deep = format!("#{dr:02X}{dg:02X}{db:02X}");

        // Заливка выделения: 15% акцента.
        self.accent_fill = format!("#26{r:02X}{g:02X}{b:02X}");

        // Текст на кнопке контрастен её БАЗЕ (accent-deep — именно он
        // фон кнопок в покое), а не «чистому» акценту. Раньше текст
        // считался по яркости акцента: для светлого акцента получали
        // тёмный текст, который терялся на светлом accent-deep.
        // Порог 0.4: белый текст читается только на действительно
        // тёмной базе (жёлтые/золотые даже затемнённые остаются светлыми
        // — на них нужен тёмный текст, ~9:1 против ~2:1 у белого).
        let lum = luminance(dr, dg, db);
        self.text_on_accent = if lum < 0.4 {
            "#FFFFFF".into()
        } else {
            "#151515".into()
        };

        // Текст на 15%-заливке лежит почти на фоне темы: в светлых
        // темах читается только затемнённый акцент, в тёмных — сам акцент.
        self.text_on_accent_fill = if self.is_light() {
            self.accent_deep.clone()
        } else {
            self.accent.clone()
        };
    }

    /// Тема — светлая? (для картинок-превью и панелей).
    pub fn is_light(&self) -> bool {
        parse_hex(&self.bg)
            .map(|(r, g, b)| luminance(r, g, b) > 0.5)
            .unwrap_or(false)
    }
}

/// `#RRGGBB` → `(r, g, b)`.
fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Относительная яркимость [0..1] (sRGB, упрощённая).
fn luminance(r: u8, g: u8, b: u8) -> f64 {
    let srgb = |v: u8| (f64::from(v) / 255.0).powf(2.2);
    0.2126 * srgb(r) + 0.7152 * srgb(g) + 0.0722 * srgb(b)
}

/// Пропорциональное затемнение.
fn darken(r: u8, g: u8, b: u8, k: f64) -> (u8, u8, u8) {
    (
        (f64::from(r) * k).round() as u8,
        (f64::from(g) * k).round() as u8,
        (f64::from(b) * k).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_distinct() {
        let ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 6);
        assert_ne!(Theme::fluent_dark(), Theme::mono_light());
        assert_ne!(Theme::pastel_warm(), Theme::pastel_mint());
    }

    #[test]
    fn accent_override_updates_contrast() {
        let mut t = Theme::mono_dark();
        t.apply_accent("#FFFFFF");
        assert_eq!(t.accent, "#FFFFFF");
        assert_eq!(t.text_on_accent, "#151515", "на белом акценте текст тёмный");

        t.apply_accent("#0067C0");
        assert_eq!(t.text_on_accent, "#FFFFFF", "на синем акценте текст белый");
    }

    /// Текст на кнопке должен контрастировать с её базой (accent-deep),
    /// а не с «чистым» акцентом: регрессия светлых акцентов.
    #[test]
    fn text_on_accent_contrasts_button_base() {
        // Светлый акцент в тёмной теме: кнопка = darken(#FFD700) = #D1B000,
        // на ней нужен тёмный текст (белый давал бы ~2:1).
        let mut t = Theme::fluent_dark();
        t.apply_accent("#FFD700");
        assert_eq!(t.accent_deep, "#D1B000");
        assert_eq!(
            t.text_on_accent, "#151515",
            "тёмный текст на светлой базе кнопки"
        );

        // Средний акцент: затемнённая база #3EA0D1 достаточно тёмная
        // для белого текста.
        t.apply_accent("#4CC2FF");
        assert_eq!(t.text_on_accent, "#FFFFFF");

        // Тот же жёлтый акцент в светлой теме: заливка почти сливается
        // с фоном, текст на ней — затемнённый акцент.
        let mut l = Theme::fluent_light();
        l.apply_accent("#FFD700");
        assert_eq!(l.text_on_accent_fill, "#D1B000");
        // В тёмной — сам акцент (светлый на тёмном фоне).
        t.apply_accent("#FFD700");
        assert_eq!(t.text_on_accent_fill, "#FFD700");
    }

    #[test]
    fn light_detection() {
        assert!(Theme::pastel_warm().is_light());
        assert!(!Theme::fluent_dark().is_light());
    }
}
