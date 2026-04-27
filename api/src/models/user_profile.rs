use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use shared::recommendation::UserMusicProfile;

#[cfg(feature = "server")]
use crate::db::DB;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct UserProfileRow {
    pub user_id: String,
    pub profile_json: String,
    pub top_artists_hash: Option<String>,
    pub last_report: Option<String>,
    pub updated_at: String,
}

#[cfg(feature = "server")]
impl UserProfileRow {
    pub async fn get(user_id: &str) -> Result<Option<UserMusicProfile>, String> {
        let row = sqlx::query_as::<_, Self>("SELECT * FROM user_profiles WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&*DB)
            .await
            .map_err(|e| e.to_string())?;
        match row {
            Some(r) => {
                let profile: UserMusicProfile =
                    serde_json::from_str(&r.profile_json).map_err(|e| e.to_string())?;
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    pub async fn upsert(user_id: &str, profile: &UserMusicProfile) -> Result<(), String> {
        let json = serde_json::to_string(profile).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO user_profiles (user_id, profile_json, top_artists_hash, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
               profile_json = excluded.profile_json,
               top_artists_hash = excluded.top_artists_hash,
               updated_at = excluded.updated_at",
        )
        .bind(user_id)
        .bind(&json)
        .bind(&profile.top_artists_hash)
        .bind(&now)
        .execute(&*DB)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_hash(user_id: &str) -> Result<Option<String>, String> {
        let row = sqlx::query_scalar::<_, Option<String>>(
            "SELECT top_artists_hash FROM user_profiles WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&*DB)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.flatten())
    }

    pub async fn update_report(user_id: &str, report_json: &str) -> Result<(), String> {
        sqlx::query("UPDATE user_profiles SET last_report = ? WHERE user_id = ?")
            .bind(report_json)
            .bind(user_id)
            .execute(&*DB)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_report(user_id: &str) -> Result<Option<String>, String> {
        let row = sqlx::query_scalar::<_, Option<String>>(
            "SELECT last_report FROM user_profiles WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&*DB)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.flatten())
    }
}
