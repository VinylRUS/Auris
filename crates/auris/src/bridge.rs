//! Мост между ядром (auris-core) и Slint-темой.

use slint::ComponentHandle;

use crate::MainWindow;
use crate::Theme as UiTheme;
use auris_core::theme::Theme;

/// `#RRGGBB` / `#AARRGGBB` → цвет Slint.
pub fn color_from_hex(hex: &str) -> slint::Color {
    let h = hex.trim_start_matches('#');
    let (a, r, g, b) = match h.len() {
        6 => (
            0xFF,
            u8::from_str_radix(&h[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(0),
        ),
        8 => (
            u8::from_str_radix(&h[0..2], 16).unwrap_or(0xFF),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(0),
            u8::from_str_radix(&h[6..8], 16).unwrap_or(0),
        ),
        _ => (0xFF, 0, 0, 0),
    };
    slint::Color::from_argb_u8(a, r, g, b)
}

/// Применяет тему к глобалу Theme окна.
pub fn apply_theme(ui: &MainWindow, theme: &Theme) {
    let g = ui.global::<UiTheme>();
    g.set_bg(color_from_hex(&theme.bg));
    g.set_bg_deep(color_from_hex(&theme.bg_deep));
    g.set_card(color_from_hex(&theme.card));
    g.set_fill(color_from_hex(&theme.fill));
    g.set_fill_hover(color_from_hex(&theme.fill_hover));
    g.set_stroke(color_from_hex(&theme.stroke));
    g.set_stroke_strong(color_from_hex(&theme.stroke_strong));
    g.set_text(color_from_hex(&theme.text));
    g.set_text_2(color_from_hex(&theme.text_2));
    g.set_text_3(color_from_hex(&theme.text_3));
    g.set_accent(color_from_hex(&theme.accent));
    g.set_accent_deep(color_from_hex(&theme.accent_deep));
    g.set_text_on_accent(color_from_hex(&theme.text_on_accent));
    g.set_accent_fill(color_from_hex(&theme.accent_fill));
    g.set_text_on_accent_fill(color_from_hex(&theme.text_on_accent_fill));
    g.set_light(theme.is_light());
}

/// Тема из настроек (пресет + кастомный акцент).
pub fn theme_from_config(preset: &str, accent: &Option<String>) -> Theme {
    let mut t = Theme::by_preset(preset);
    if let Some(hex) = accent {
        if hex.starts_with('#') && hex.len() >= 7 {
            t.apply_accent(hex);
        }
    }
    t
}
