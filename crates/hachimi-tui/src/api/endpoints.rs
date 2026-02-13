use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::client::HachimiClient;
use crate::model::{
    auth::*,
    play_history::*,
    playlist::*,
    song::*,
    user::*,
};

// — 查询参数结构 —

#[derive(Serialize)]
pub struct RecentQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<DateTime<Utc>>,
    pub limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<bool>,
}

#[derive(Serialize)]
pub struct SongSearchQuery {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
}

#[derive(Serialize)]
pub struct UserSearchQuery {
    pub q: String,
    pub page: i32,
    pub size: i32,
}

#[derive(Serialize)]
pub struct PlaylistSearchQuery {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
}

#[derive(Serialize)]
pub struct PageByUserQuery {
    pub user_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}

#[derive(Serialize)]
pub struct HistoryCursorQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<DateTime<Utc>>,
    pub size: i32,
}

#[derive(Serialize)]
pub struct PageFavoritesQuery {
    pub page_index: i64,
    pub page_size: i64,
}

#[derive(Serialize)]
pub struct IdQuery {
    pub id: i64,
}

#[derive(Serialize)]
pub struct DisplayIdQuery {
    pub id: String,
}

#[derive(Serialize)]
pub struct UidQuery {
    pub uid: i64,
}

#[derive(Serialize)]
pub struct PlaylistIdBody {
    pub id: i64,
}

#[derive(Serialize)]
pub struct AddSongBody {
    pub playlist_id: i64,
    pub song_id: i64,
}

#[derive(Serialize)]
pub struct RemoveSongBody {
    pub playlist_id: i64,
    pub song_id: i64,
}

#[derive(Serialize)]
pub struct CreatePlaylistBody {
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
}

#[derive(Serialize)]
pub struct TouchBody {
    pub song_id: i64,
}

#[derive(Serialize)]
pub struct DeleteHistoryBody {
    pub history_id: i64,
}

#[derive(Serialize)]
pub struct FavoriteBody {
    pub playlist_id: i64,
}

#[derive(Serialize)]
pub struct CheckFavoriteQuery {
    pub playlist_id: i64,
}

impl HachimiClient {
    // — 认证 —

    pub async fn login(&self, req: &LoginReq) -> Result<LoginResp> {
        self.post("/auth/login/email", req).await
    }

    pub async fn refresh_token(&self, req: &RefreshTokenReq) -> Result<TokenPair> {
        self.post("/auth/refresh_token", req).await
    }

    pub async fn generate_captcha(&self) -> Result<GenerateCaptchaResp> {
        self.get("/auth/captcha/generate").await
    }

    pub async fn submit_captcha(&self, req: &SubmitCaptchaReq) -> Result<()> {
        self.post::<_, serde_json::Value>("/auth/captcha/submit", req)
            .await?;
        Ok(())
    }

    // — 歌曲 —

    pub async fn recent_songs(&self, query: &RecentQuery) -> Result<RecentResp> {
        self.get_with_query("/song/recent_v2", query).await
    }

    pub async fn recommend_songs(&self) -> Result<RecommendResp> {
        self.get("/song/recommend").await
    }

    pub async fn recommend_songs_anonymous(&self) -> Result<RecommendResp> {
        self.get("/song/recommend_anonymous").await
    }

    pub async fn hot_songs_weekly(&self) -> Result<HotResp> {
        self.get("/song/hot/weekly").await
    }

    pub async fn search_songs(&self, query: &SongSearchQuery) -> Result<SongSearchResp> {
        self.get_with_query("/song/search", query).await
    }

    pub async fn song_detail_by_id(&self, id: i64) -> Result<PublicSongDetail> {
        self.get_with_query("/song/detail_by_id", &IdQuery { id })
            .await
    }

    pub async fn song_detail(&self, display_id: &str) -> Result<PublicSongDetail> {
        self.get_with_query(
            "/song/detail",
            &DisplayIdQuery {
                id: display_id.to_string(),
            },
        )
        .await
    }

    pub async fn recommend_tags(&self) -> Result<TagRecommendResp> {
        self.get("/song/tag/recommend").await
    }

    pub async fn songs_by_user(&self, query: &PageByUserQuery) -> Result<PageByUserResp> {
        self.get_with_query("/song/page_by_user", query).await
    }

    // — 歌单 —

    pub async fn my_playlists(&self) -> Result<PlaylistListResp> {
        self.get("/playlist/list").await
    }

    pub async fn playlist_detail_private(&self, id: i64) -> Result<PlaylistDetailResp> {
        self.get_with_query("/playlist/detail_private", &IdQuery { id })
            .await
    }

    pub async fn playlist_detail(&self, id: i64) -> Result<PlaylistDetailResp> {
        self.get_with_query("/playlist/detail", &IdQuery { id })
            .await
    }

    pub async fn create_playlist(&self, body: &CreatePlaylistBody) -> Result<CreatePlaylistResp> {
        self.post("/playlist/create", body).await
    }

    pub async fn delete_playlist(&self, id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>("/playlist/delete", &PlaylistIdBody { id })
            .await?;
        Ok(())
    }

    pub async fn add_song_to_playlist(&self, playlist_id: i64, song_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/playlist/add_song",
            &AddSongBody {
                playlist_id,
                song_id,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn remove_song_from_playlist(&self, playlist_id: i64, song_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/playlist/remove_song",
            &RemoveSongBody {
                playlist_id,
                song_id,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn search_playlists(
        &self,
        query: &PlaylistSearchQuery,
    ) -> Result<PlaylistSearchResp> {
        self.get_with_query("/playlist/search", query).await
    }

    pub async fn playlists_containing(&self, song_id: i64) -> Result<ListContainingResp> {
        self.get_with_query(
            "/playlist/list_containing",
            &IdQuery { id: song_id },
        )
        .await
    }

    pub async fn favorite_playlists(
        &self,
        query: &PageFavoritesQuery,
    ) -> Result<PageFavoritesResp> {
        self.get_with_query("/playlist/favorite/page", query).await
    }

    pub async fn add_favorite(&self, playlist_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/playlist/favorite/add",
            &FavoriteBody { playlist_id },
        )
        .await?;
        Ok(())
    }

    pub async fn remove_favorite(&self, playlist_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/playlist/favorite/remove",
            &FavoriteBody { playlist_id },
        )
        .await?;
        Ok(())
    }

    pub async fn check_favorite(&self, playlist_id: i64) -> Result<CheckFavoriteResp> {
        self.get_with_query(
            "/playlist/favorite/check",
            &CheckFavoriteQuery { playlist_id },
        )
        .await
    }

    // — 用户 —

    pub async fn user_profile(&self, uid: i64) -> Result<PublicUserProfile> {
        self.get_with_query("/user/profile", &UidQuery { uid })
            .await
    }

    pub async fn search_users(&self, query: &UserSearchQuery) -> Result<UserSearchResp> {
        self.get_with_query("/user/search", query).await
    }

    // — 播放历史 —

    pub async fn play_history(&self, query: &HistoryCursorQuery) -> Result<CursorResp> {
        self.get_with_query("/play_history/cursor", query).await
    }

    pub async fn touch_play_history(&self, song_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/play_history/touch",
            &TouchBody { song_id },
        )
        .await?;
        Ok(())
    }

    pub async fn touch_play_history_anonymous(&self, song_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/play_history/touch_anonymous",
            &TouchBody { song_id },
        )
        .await?;
        Ok(())
    }

    pub async fn delete_play_history(&self, history_id: i64) -> Result<()> {
        self.post::<_, serde_json::Value>(
            "/play_history/delete",
            &DeleteHistoryBody { history_id },
        )
        .await?;
        Ok(())
    }
}
