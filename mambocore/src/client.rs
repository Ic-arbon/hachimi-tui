use std::sync::Arc;

use anyhow::{Result, bail};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use tokio::sync::RwLock;

use crate::model::WebResp;
use crate::model::auth::{AuthData, RefreshTokenReq};

const DEFAULT_BASE_URL: &str = "https://api.hachimi.world";

/// 认证状态变更事件，调用方据此持久化或清除本地存储
pub enum AuthEvent {
    /// token 已刷新，调用方应持久化新数据
    Refreshed(AuthData),
    /// token 失效，调用方应清除本地存储
    Cleared,
}

#[derive(Clone)]
pub struct MamboClient {
    http: reqwest::Client,
    base_url: String,
    auth: Arc<RwLock<Option<AuthData>>>,
}

impl MamboClient {
    pub fn new(base_url: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static("https://hachimi.world/"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("tokaitui/0.1.0"),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .gzip(true)
            .build()?;

        Ok(Self {
            http,
            base_url: base_url.unwrap_or(DEFAULT_BASE_URL).to_string(),
            auth: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn set_auth(&self, data: AuthData) {
        *self.auth.write().await = Some(data);
    }

    pub async fn clear_auth(&self) {
        *self.auth.write().await = None;
    }

    pub async fn is_authenticated(&self) -> bool {
        self.auth.read().await.is_some()
    }

    /// 同步检查是否已认证（用于渲染）
    pub fn is_authenticated_sync(&self) -> bool {
        self.auth.try_read().map_or(false, |g| g.is_some())
    }

    /// 检查 token 是否过期，过期则尝试刷新，刷新失败则清除认证。
    /// 返回 `Some(AuthEvent)` 表示状态发生了变更，调用方应据此持久化。
    pub async fn ensure_valid_auth(&self) -> Option<AuthEvent> {
        let (expired, refresh_token) = {
            let guard = self.auth.read().await;
            match guard.as_ref() {
                Some(auth) if auth.is_expired() => (true, auth.refresh_token.clone()),
                _ => return None,
            }
        };
        if !expired {
            return None;
        }
        // 尝试刷新 token
        let result = self
            .refresh_token(&RefreshTokenReq {
                refresh_token,
                device_info: "tokaitui".to_string(),
            })
            .await;
        match result {
            Ok(pair) => {
                let old_username = {
                    let guard = self.auth.read().await;
                    guard.as_ref().and_then(|a| a.username.clone())
                };
                let auth = AuthData {
                    access_token: pair.access_token,
                    refresh_token: pair.refresh_token,
                    expires_at: pair.expires_in.timestamp(),
                    username: old_username,
                };
                self.set_auth(auth.clone()).await;
                Some(AuthEvent::Refreshed(auth))
            }
            Err(_) => {
                // 刷新失败，清除认证以降级到匿名模式
                self.clear_auth().await;
                Some(AuthEvent::Cleared)
            }
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn auth_header(&self) -> Option<String> {
        let guard = self.auth.read().await;
        guard
            .as_ref()
            .map(|a| format!("Bearer {}", a.access_token))
    }

    /// 解析 JSON 响应，出错时附带路径和原始 body 片段
    fn parse_response<T: serde::de::DeserializeOwned>(
        path: &str,
        text: &str,
    ) -> Result<T> {
        // 先尝试解析为标准 WebResp
        match serde_json::from_str::<WebResp<T>>(text) {
            Ok(web) => web
                .into_result()
                .map_err(|e| anyhow::anyhow!("[{}] {}", path, e)),
            Err(_) => {
                // 非 WebResp 格式，尝试提取 error 字段（如 {"error":"Invalid token"}）
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
                        bail!("[{}] {}", path, err);
                    }
                }
                bail!("[{}] unexpected response: {}", path, &text[..text.len().min(200)]);
            }
        }
    }

    /// GET 请求（无参数）
    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path);
        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }

        let text = req.send().await?.text().await?;
        Self::parse_response(path, &text)
    }

    /// GET 请求（带查询参数）
    pub async fn get_with_query<Q: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let url = self.url(path);
        let mut req = self.http.get(&url).query(query);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }

        let text = req.send().await?.text().await?;
        Self::parse_response(path, &text)
    }

    /// POST 请求（JSON body）
    pub async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(path);
        let mut req = self.http.post(&url).json(body);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }

        let text = req.send().await?.text().await?;
        Self::parse_response(path, &text)
    }

    /// 获取音频流（用于流式播放）
    /// url 可以是完整 URL 或相对路径
    pub async fn get_audio_stream(&self, url: &str) -> Result<reqwest::Response> {
        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            self.url(url)
        };
        let mut req = self.http.get(&full_url);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }
        Ok(req.send().await?)
    }
}
