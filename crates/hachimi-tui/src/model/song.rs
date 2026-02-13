use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicSongDetail {
    pub id: i64,
    pub display_id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub duration_seconds: i32,
    pub tags: Vec<TagItem>,
    pub lyrics: String,
    pub audio_url: String,
    pub cover_url: String,
    pub production_crew: Vec<SongProductionCrew>,
    pub creation_type: i32,
    pub origin_infos: Vec<CreationTypeInfo>,
    pub uploader_uid: i64,
    pub uploader_name: String,
    pub play_count: i64,
    pub like_count: i64,
    pub external_links: Vec<ExternalLink>,
    pub create_time: DateTime<Utc>,
    pub release_time: DateTime<Utc>,
    pub explicit: Option<bool>,
    pub gain: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSongItem {
    pub id: i64,
    pub display_id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub artist: String,
    pub duration_seconds: i32,
    pub play_count: i64,
    pub like_count: i64,
    pub cover_art_url: String,
    pub audio_url: String,
    pub uploader_uid: i64,
    pub uploader_name: String,
    pub explicit: Option<bool>,
    #[serde(default)]
    pub original_artists: Vec<String>,
    #[serde(default)]
    pub original_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagItem {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRecommendItem {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongProductionCrew {
    pub id: i64,
    pub role: String,
    pub uid: Option<i64>,
    pub person_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationTypeInfo {
    pub song_display_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub url: Option<String>,
    pub origin_type: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLink {
    pub platform: String,
    pub url: String,
}

// — API 请求/响应 —

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentResp {
    pub songs: Vec<PublicSongDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendResp {
    pub songs: Vec<PublicSongDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotResp {
    pub songs: Vec<PublicSongDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongSearchResp {
    pub hits: Vec<SearchSongItem>,
    pub query: String,
    pub processing_time_ms: i64,
    pub total_hits: Option<i32>,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRecommendResp {
    pub result: Vec<TagRecommendItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageByUserResp {
    pub songs: Vec<PublicSongDetail>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

impl PublicSongDetail {
    pub fn format_duration(&self) -> String {
        let mins = self.duration_seconds / 60;
        let secs = self.duration_seconds % 60;
        format!("{mins}:{secs:02}")
    }
}

impl SearchSongItem {
    pub fn format_duration(&self) -> String {
        let mins = self.duration_seconds / 60;
        let secs = self.duration_seconds % 60;
        format!("{mins}:{secs:02}")
    }

    pub fn into_song_detail(self) -> PublicSongDetail {
        PublicSongDetail {
            id: self.id,
            display_id: self.display_id,
            title: self.title,
            subtitle: self.subtitle,
            description: self.description,
            duration_seconds: self.duration_seconds,
            tags: vec![],
            lyrics: String::new(),
            audio_url: self.audio_url,
            cover_url: self.cover_art_url,
            production_crew: vec![],
            creation_type: 0,
            origin_infos: vec![],
            uploader_uid: self.uploader_uid,
            uploader_name: self.uploader_name,
            play_count: self.play_count,
            like_count: self.like_count,
            external_links: vec![],
            create_time: chrono::Utc::now(),
            release_time: chrono::Utc::now(),
            explicit: self.explicit,
            gain: None,
        }
    }
}
