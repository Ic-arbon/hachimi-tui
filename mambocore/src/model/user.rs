use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUserProfile {
    pub uid: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub gender: Option<i32>,
    pub is_banned: bool,
}

#[allow(dead_code)] // TODO: 用户搜索
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSearchResp {
    pub hits: Vec<PublicUserProfile>,
    pub query: String,
    pub processing_time_ms: i64,
    pub total_hits: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}
