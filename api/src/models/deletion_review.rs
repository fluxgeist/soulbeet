use serde::{Deserialize, Serialize};
use shared::navidrome::DeletionReview;

#[cfg(feature = "server")]
use crate::db::DB;
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct DeletionReviewRow {
    pub id: String,
    pub song_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub path: Option<String>,
    pub rating: Option<i32>,
    pub action: String,
    pub user_id: String,
    pub created_at: String,
}

impl From<DeletionReviewRow> for DeletionReview {
    fn from(row: DeletionReviewRow) -> Self {
        DeletionReview {
            id: row.id,
            song_id: row.song_id,
            title: row.title,
            artist: row.artist,
            album: row.album,
            path: row.path,
            rating: row.rating.map(|r| r as u8),
            created_at: row.created_at,
        }
    }
}

#[cfg(feature = "server")]
impl DeletionReviewRow {
    pub async fn upsert(
        song_id: &str,
        title: &str,
        artist: &str,
        album: &str,
        path: Option<&str>,
        rating: Option<u8>,
        user_id: &str,
    ) -> Result<(), String> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO deletion_reviews (id, song_id, title, artist, album, path, rating, action, user_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'Deleted', ?)
             ON CONFLICT(song_id) DO UPDATE SET
               rating = excluded.rating,
               title = excluded.title,
               artist = excluded.artist,
               album = excluded.album,
               path = excluded.path,
               action = 'Deleted'"
        )
        .bind(&id)
        .bind(song_id)
        .bind(title)
        .bind(artist)
        .bind(album)
        .bind(path)
        .bind(rating.map(|r| r as i32))
        .bind(user_id)
        .execute(&*DB)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete(id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM deletion_reviews WHERE id = ?")
            .bind(id)
            .execute(&*DB)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_history(user_id: &str, limit: u32) -> Result<Vec<DeletionReview>, String> {
        let rows = sqlx::query_as::<_, DeletionReviewRow>(
            "SELECT * FROM deletion_reviews WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&*DB)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
