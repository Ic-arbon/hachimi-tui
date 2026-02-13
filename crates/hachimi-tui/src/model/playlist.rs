use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::user::PublicUserProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
    pub description: Option<String>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub is_public: bool,
    pub songs_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistMetadata {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub user_avatar_url: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub songs_count: i64,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSongItem {
    pub song_id: i64,
    pub song_display_id: String,
    pub title: String,
    pub subtitle: String,
    pub cover_url: String,
    pub uploader_name: String,
    pub uploader_uid: i64,
    pub duration_seconds: i32,
    pub order_index: i32,
    pub add_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritePlaylistItem {
    pub metadata: PlaylistMetadata,
    pub order_index: i32,
    pub add_time: DateTime<Utc>,
}

// — API 响应 —

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistListResp {
    pub playlists: Vec<PlaylistItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetailResp {
    pub playlist_info: PlaylistItem,
    pub songs: Vec<PlaylistSongItem>,
    pub creator_profile: Option<PublicUserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSearchResp {
    pub hits: Vec<PlaylistMetadata>,
    pub query: String,
    pub processing_time_ms: i64,
    pub total_hits: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListContainingResp {
    pub playlist_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlaylistResp {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFavoritesResp {
    pub data: Vec<FavoritePlaylistItem>,
    pub page_index: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFavoriteResp {
    pub playlist_id: i64,
    pub is_favorite: bool,
    pub add_time: Option<DateTime<Utc>>,
}

impl PlaylistSongItem {
    pub fn format_duration(&self) -> String {
        let mins = self.duration_seconds / 60;
        let secs = self.duration_seconds % 60;
        format!("{mins}:{secs:02}")
    }
}
