use crate::model::auth::LoginReq;
use crate::ui::login::{LoginState, LoginStep};

use super::super::{App, AppMessage, InputMode};

impl App {
    // — 认证 —

    /// 第一步：校验输入 → 异步生成 captcha
    pub(crate) fn start_captcha(&mut self) {
        let email = self.login.email.trim().to_string();
        let password = self.login.password.clone();

        if email.is_empty() || password.is_empty() {
            self.login.error = Some(t!("app.email_password_required").to_string());
            return;
        }

        self.login.step = LoginStep::GeneratingCaptcha;
        self.login.error = None;

        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let result = client.generate_captcha().await;
            let _ = tx.send(AppMessage::CaptchaGenerated(
                result
                    .map(|resp| (resp.captcha_key, resp.url))
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// 第二步：captcha 已完成，提交登录
    pub(crate) fn submit_login(&mut self) {
        let Some(captcha_key) = self.login.captcha_key.clone() else {
            self.login.error = Some(t!("app.no_captcha_key").to_string());
            self.login.step = LoginStep::Input;
            return;
        };

        let email = self.login.email.trim().to_string();
        let password = self.login.password.clone();

        self.login.step = LoginStep::Submitting;
        self.login.error = None;

        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let req = LoginReq {
                email,
                password,
                code: None,
                device_info: "tokaitui".to_string(),
                captcha_key,
            };

            let result = client.login(&req).await;
            let _ = tx.send(AppMessage::LoginResult(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub(crate) fn logout(&mut self) {
        let _ = crate::config::auth_store::clear();

        let client = self.client.clone();
        tokio::spawn(async move {
            client.clear_auth().await;
        });

        self.username = None;
        self.cache.songs.clear();
        self.cache.loading.clear();
        self.cache.tags = None;
        self.cache.playlists = None;
        self.cache.queue_song_detail.clear();
        self.login = LoginState::new();
        self.ui.input_mode = InputMode::Login;
    }

    /// 恢复上次退出时的播放
    pub(crate) fn resume_playback(&mut self) {
        if let Some(song) = self.queue.current_song().cloned() {
            let pos = self.resume_position_ms;
            self.start_audio_fetch(song.id, &song.name, &song.artist);
            self.resume_position_ms = pos;
        }
    }
}
