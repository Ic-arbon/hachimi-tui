use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 本地持久化的认证数据（access_token + refresh_token + 过期时间戳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    #[serde(default)]
    pub username: Option<String>,
}

impl AuthData {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
    pub code: Option<String>,
    pub device_info: String,
    pub captcha_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResp {
    #[allow(dead_code)] // TODO: 用户 UID
    pub uid: i64,
    pub username: String,
    pub token: TokenPair,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshTokenReq {
    pub refresh_token: String,
    pub device_info: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateCaptchaResp {
    pub captcha_key: String,
    pub url: String,
}

#[allow(dead_code)] // TODO: 验证码提交
#[derive(Debug, Clone, Serialize)]
pub struct SubmitCaptchaReq {
    pub captcha_key: String,
    pub token: String,
}
