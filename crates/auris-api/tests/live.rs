//! Живые тесты против настоящего API (гость, без токена).
//!
//! Помечены `#[ignore]`: запускаются явно
//! `cargo test -p auris-api --test live -- --ignored`.
//! Песочница может быть гео-заблокирована для контентных эндпоинтов
//! (451) — это тоже полезный сигнал, поэтому проверяем только
//! `/account/status`, который отвечает без региональных ограничений.

use auris_api::Client;

/// Гостевой `/account/status` должен РАЗВОРАЧИВАТЬСЯ из `result`:
/// до фикса поля молча подставлялись дефолтами (region = None),
/// после — приходит реальный гостевой аккаунт (region = 10000).
#[tokio::test]
#[ignore = "network"]
async fn guest_account_status_is_unwrapped() {
    let client = Client::new();
    let status = client.account_status().await.expect("guest status");
    let account = status.account.expect("account inside result");
    assert!(
        account.region.is_some(),
        "region должен прийти из result (гость: 10000), а не остаться None"
    );
    println!(
        "guest account: region={:?}, uid={}",
        account.region, account.uid
    );
}
