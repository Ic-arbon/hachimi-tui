use std::sync::Arc;

use anyhow::{Result, bail};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use tokio::sync::RwLock;

use crate::config::auth_store::AuthData;
use crate::model::{CommonError, WebResp};

const DEFAULT_BASE_URL: &str = "https://api.hachimi.world";

#[derive(Clone)]
pub struct HachimiClient {
    http: reqwest::Client,
    base_url: String,
    auth: Arc<RwLock<Option<AuthData>>>,
}

impl HachimiClient {
    pub fn new(base_url: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static("https://hachimi.world/"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("hachimi-tui/0.1.0"),
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

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn auth_header(&self) -> Option<String> {
        let guard = self.auth.read().await;
        guard
            .as_ref()
            .map(|a| format!("Bearer {}", a.access_token))
    }

    /// GET 请求（无参数）
    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path);
        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }

        let resp = req.send().await?;
        let web: WebResp<T> = resp.json().await?;
        web.into_result()
            .map_err(|e| anyhow::anyhow!("{}", e))
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

        let resp = req.send().await?;
        let web: WebResp<T> = resp.json().await?;
        web.into_result()
            .map_err(|e| anyhow::anyhow!("{}", e))
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

        let resp = req.send().await?;
        let web: WebResp<T> = resp.json().await?;
        web.into_result()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 获取音频流（用于流式播放）
    pub async fn get_audio_stream(&self, url: &str) -> Result<reqwest::Response> {
        let mut req = self.http.get(url);
        if let Some(auth) = self.auth_header().await {
            req = req.header(AUTHORIZATION, auth);
        }
        Ok(req.send().await?)
    }
}
