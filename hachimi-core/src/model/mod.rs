pub mod auth;
pub mod playlist;
pub mod song;
pub mod user;

use serde::Deserialize;

/// API 通用响应包装
#[derive(Debug, Clone, Deserialize)]
pub struct WebResp<T> {
    pub ok: bool,
    pub data: serde_json::Value,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommonError {
    pub code: String,
    pub msg: String,
}

impl<T: serde::de::DeserializeOwned> WebResp<T> {
    pub fn into_result(self) -> Result<T, CommonError> {
        if self.ok {
            serde_json::from_value(self.data).map_err(|e| CommonError {
                code: "parse_error".to_string(),
                msg: e.to_string(),
            })
        } else {
            let err: CommonError = serde_json::from_value(self.data).unwrap_or(CommonError {
                code: "unknown".to_string(),
                msg: "unknown error".to_string(),
            });
            Err(err)
        }
    }
}

impl std::fmt::Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.msg)
    }
}

impl std::error::Error for CommonError {}

/// 播放历史
pub mod play_history {
    use chrono::{DateTime, Utc};
    use serde::Deserialize;

    use super::song::PublicSongDetail;

    #[derive(Debug, Clone, Deserialize)]
    #[allow(dead_code)] // TODO: 播放历史详情
    pub struct PlayHistoryItem {
        pub id: i64,
        pub song_info: PublicSongDetail,
        pub play_time: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CursorResp {
        pub list: Vec<PlayHistoryItem>,
    }
}
