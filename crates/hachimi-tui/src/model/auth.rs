use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
