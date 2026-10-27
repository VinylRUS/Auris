//! Генерация QR-кода для страницы подтверждения входа.

use std::path::Path;

/// Пишет PNG с QR-кодом `url` в `path`.
///
/// Тёмные модули — графит `#1B1B1B`, светлые — белый: читается
/// и в тёмной, и в светлой теме окна.
pub fn write_qr_png(url: &str, path: &Path) -> anyhow::Result<()> {
    let code = qrcode::QrCode::new(url)?;
    let image = code
        .render::<image::Rgba<u8>>()
        .quiet_zone(true)
        .min_dimensions(340, 340)
        .dark_color(image::Rgba([27, 27, 27, 255]))
        .light_color(image::Rgba([255, 255, 255, 255]))
        .build();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    image.save(path)?;
    Ok(())
}
