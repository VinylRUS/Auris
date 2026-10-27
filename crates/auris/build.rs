fn main() {
    slint_build::compile("ui/main.slint").expect("failed to compile Slint UI");

    // Ресурсы Windows: иконка + метаданные версии в auris.exe.
    // Влияют только на сборку под Windows.
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        res.set(
            "FileDescription",
            "Auris — неофициальный клиент Яндекс.Музыки",
        );
        res.set("ProductName", "Auris");
        res.set("LegalCopyright", "GPL-3.0-or-later");
        res.set("OriginalFilename", "auris.exe");
        // FILEVERSION/ProductVersion должны быть числовыми: 0.0.1-beta.1 → 0.0.1.1
        let v = env!("CARGO_PKG_VERSION");
        let numeric = version_to_numeric(v);
        res.set("FileVersion", &numeric);
        res.set("ProductVersion", &numeric);
        res.compile().expect("failed to embed Windows resources");
    }
}

/// `0.0.1-beta.1` → `0.0.1.1`; `0.2.0` → `0.2.0.0`.
#[cfg(target_os = "windows")]
fn version_to_numeric(v: &str) -> String {
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let mut parts: Vec<u32> = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let pre_num = pre
        .and_then(|p| p.rsplit('.').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    while parts.len() < 3 {
        parts.push(0);
    }
    format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], pre_num)
}
