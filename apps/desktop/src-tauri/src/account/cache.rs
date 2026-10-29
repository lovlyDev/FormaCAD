use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::client::Profile;
use super::error::AccountResult;

/// Non-secret account data cached in the existing `settings` table so the
/// account panel and offline rights work without network. Tokens are not here.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountCache {
    pub server: String,
    pub profile: Option<Profile>,
    pub document: Option<String>,
    pub jwks: Vec<serde_json::Value>,
    pub synced_at: Option<i64>,
    /// Latest wall-clock time observed (clock rollback detection).
    pub max_seen: i64,
}

const KEY: &str = "account_cache";

pub async fn load(pool: &SqlitePool) -> AccountResult<Option<AccountCache>> {
    let json: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(KEY)
        .fetch_optional(pool)
        .await?;
    Ok(json.and_then(|s| serde_json::from_str(&s).ok()))
}

pub async fn save(pool: &SqlitePool, cache: &AccountCache) -> AccountResult<()> {
    let json = serde_json::to_string(cache).unwrap_or_default();
    sqlx::query("INSERT INTO settings(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(KEY)
        .bind(json)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear(pool: &SqlitePool) -> AccountResult<()> {
    sqlx::query("DELETE FROM settings WHERE key = ?")
        .bind(KEY)
        .execute(pool)
        .await?;
    Ok(())
}
