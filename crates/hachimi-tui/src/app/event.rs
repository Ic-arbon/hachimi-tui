use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::config::settings::PlayMode;
use crate::player::engine::{AudioSource, PlayerEvent};
use crate::ui::log_view::LogLevel;
use crate::ui::login::LoginStep;
use crate::ui::navigation::NavNode;

use super::{App, AppMessage, DataPayload, InputMode};

impl App {
    pub(crate) fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            // 帮助浮层打开时，只响应关闭操作
            if self.show_help {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc => {
                        self.show_help = false;
                    }
                    _ => {}
                }
                return;
            }

            // 日志浮层打开时，只响应滚动和关闭
            if self.show_logs {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('!') | KeyCode::Esc => {
                        self.show_logs = false;
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.logs.scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => self.logs.scroll_up(),
                    _ => {}
                }
                return;
            }

            match self.input_mode {
                InputMode::Normal => self.handle_normal_key(key),
                InputMode::Search => self.handle_search_key(key),
                InputMode::Login => self.handle_login_key(key),
            }
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // 全局
            (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.running = false;
            }
            (_, KeyCode::Char('?')) => {
                self.show_help = true;
            }
            (_, KeyCode::Char('!')) => {
                self.show_logs = true;
                self.logs.mark_read();
            }
            (_, KeyCode::Char('L')) => {
                self.logout();
            }
            (_, KeyCode::Char(' ')) => {
                self.toggle_play_pause();
            }
            (_, KeyCode::Char('n')) => {
                self.play_next();
            }
            (_, KeyCode::Char('N')) => {
                self.play_prev();
            }
            (_, KeyCode::Char('+') | KeyCode::Char('=')) => {
                let vol = (self.player.volume as u16 + 5).min(100) as u8;
                self.player.volume = vol;
                self.player.engine.set_volume(vol as f32 / 100.0);
            }
            (_, KeyCode::Char('-')) => {
                let vol = self.player.volume.saturating_sub(5);
                self.player.volume = vol;
                self.player.engine.set_volume(vol as f32 / 100.0);
            }
            (_, KeyCode::Char('>')) => {
                if self.player.bar.has_song() {
                    let new_pos = (self.player.bar.current_secs + 5)
                        .min(self.player.bar.total_secs);
                    self.player.engine.seek(Duration::from_secs(new_pos as u64));
                }
            }
            (_, KeyCode::Char('<')) => {
                if self.player.bar.has_song() {
                    let new_pos = self.player.bar.current_secs.saturating_sub(5);
                    self.player.engine.seek(Duration::from_secs(new_pos as u64));
                }
            }
            (_, KeyCode::Char('s')) => {
                self.settings.player.default_play_mode = match self.settings.player.default_play_mode {
                    PlayMode::Sequential => PlayMode::Shuffle,
                    PlayMode::Shuffle => PlayMode::RepeatOne,
                    PlayMode::RepeatOne => PlayMode::Sequential,
                };
            }
            (_, KeyCode::Char('v')) => {
                self.player.expanded = !self.player.expanded;
            }
            // TODO: 搜索功能尚未完成，暂时禁用
            // (_, KeyCode::Char('/')) => {
            //     self.input_mode = InputMode::Search;
            //     self.search.is_editing = true;
            // }

            // Miller Columns 导航
            (_, KeyCode::Char('j') | KeyCode::Down) => self.nav_down(),
            (_, KeyCode::Char('k') | KeyCode::Up) => self.nav_up(),
            (_, KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter) => self.nav_drill_in(),
            (_, KeyCode::Char('h') | KeyCode::Left) => self.nav_drill_out(),
            (_, KeyCode::Char('g')) => self.nav_top(),
            (_, KeyCode::Char('G')) => self.nav_bottom(),

            (_, KeyCode::Char('a')) => {
                self.add_selected_to_queue();
            }
            (_, KeyCode::Char('o')) => {
                if let Some(song) = self.selected_song().cloned() {
                    if let Some(link) = song.external_links.first() {
                        let _ = open::that(&link.url);
                    }
                }
            }
            (_, KeyCode::Char('p')) => {
                // TODO: 添加到歌单
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.input_mode = InputMode::Normal;
                self.search.is_editing = false;
            }
            (_, KeyCode::Enter) => {
                // TODO: 执行搜索
                self.search.is_editing = false;
                self.input_mode = InputMode::Normal;
            }
            (_, KeyCode::Tab) => {
                self.search.search_type = self.search.search_type.next();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.search.sort = self.search.sort.next();
            }
            (_, KeyCode::Backspace) => {
                if self.search.cursor_pos > 0 {
                    self.search.cursor_pos -= 1;
                    self.search.query.remove(self.search.cursor_pos);
                }
            }
            (_, KeyCode::Char(c)) => {
                self.search.query.insert(self.search.cursor_pos, c);
                self.search.cursor_pos += 1;
            }
            _ => {}
        }
    }

    fn handle_login_key(&mut self, key: KeyEvent) {
        if self.login.is_busy() {
            return;
        }

        match self.login.step {
            LoginStep::Input => self.handle_login_input_key(key),
            LoginStep::WaitingCaptcha => self.handle_login_captcha_key(key),
            _ => {}
        }
    }

    fn handle_login_input_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.running = false;
            }
            (_, KeyCode::Tab) => {
                self.login.toggle_field();
            }
            (_, KeyCode::Enter) => {
                self.start_captcha();
            }
            (_, KeyCode::Backspace) => {
                let (text, cursor) = self.login.current_input();
                if *cursor > 0 {
                    *cursor -= 1;
                    text.remove(*cursor);
                }
            }
            (_, KeyCode::Char(c)) => {
                let (text, cursor) = self.login.current_input();
                text.insert(*cursor, c);
                *cursor += 1;
            }
            _ => {}
        }
    }

    fn handle_login_captcha_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.login.step = LoginStep::Input;
                self.login.captcha_key = None;
                self.login.error = None;
            }
            (_, KeyCode::Enter) => {
                self.submit_login();
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::TermEvent(ev) => {
                self.handle_event(ev);
            }
            AppMessage::PlayerTick => {
                self.scroll_tick = self.scroll_tick.wrapping_add(1);
            }
            AppMessage::PlayerStateChanged(event) => {
                match event {
                    PlayerEvent::Playing => {
                        self.player.bar.is_playing = true;
                        self.player.bar.is_loading = false;
                    }
                    PlayerEvent::Paused => {
                        self.player.bar.is_playing = false;
                    }
                    PlayerEvent::Stopped => {
                        self.player.bar.is_playing = false;
                        self.player.bar.title.clear();
                        self.player.bar.artist.clear();
                        self.player.bar.current_secs = 0;
                        self.player.bar.total_secs = 0;
                    }
                    PlayerEvent::Progress { position_secs, duration_secs } => {
                        self.player.bar.current_secs = position_secs;
                        self.player.bar.total_secs = duration_secs;
                    }
                    PlayerEvent::TrackEnded => {
                        self.play_next();
                    }
                    PlayerEvent::Error(msg) => {
                        self.player.bar.is_loading = false;
                        self.logs.push(LogLevel::Error, msg);
                    }
                    PlayerEvent::Loading => {
                        self.player.bar.is_loading = true;
                    }
                }
            }
            AppMessage::AudioFetched { detail, data } => {
                self.player.bar.title = detail.title.clone();
                self.player.bar.artist = detail.uploader_name.clone();
                self.player.bar.total_secs = detail.duration_seconds as u32;
                self.player.bar.current_secs = 0;
                self.player.bar.is_loading = false;
                self.player.bar.cover_url = detail.cover_url.clone();
                let duration_secs = detail.duration_seconds as u32;
                let cover_url = detail.cover_url.clone();
                self.player.current_detail = Some(detail);
                self.player.engine.play(AudioSource::Buffered(data), duration_secs);
                self.start_image_fetch(&cover_url);
            }
            AppMessage::AudioFetchError(err) => {
                self.player.bar.is_loading = false;
                self.logs.push(LogLevel::Error, err);
            }
            AppMessage::DataLoaded(payload) => match payload {
                DataPayload::Songs(node, songs) => {
                    self.cache.loading.remove(&node);
                    if !songs.is_empty() {
                        self.cache.songs.insert(node, songs);
                    }
                }
                DataPayload::Tags(tags) => {
                    self.cache.loading.remove(&NavNode::Categories);
                    self.cache.tags = tags;
                }
            },
            AppMessage::Error(err) => {
                self.logs.push(LogLevel::Error, err);
            }
            AppMessage::CaptchaGenerated(result) => {
                match result {
                    Ok((captcha_key, url)) => {
                        self.login.captcha_key = Some(captcha_key);
                        self.login.step = LoginStep::WaitingCaptcha;
                        let _ = open::that(&url);
                    }
                    Err(e) => {
                        self.login.error = Some(e);
                        self.login.step = LoginStep::Input;
                    }
                }
            }
            AppMessage::LoginResult(result) => {
                match result {
                    Ok(resp) => {
                        let auth = crate::config::auth_store::AuthData {
                            access_token: resp.token.access_token.clone(),
                            refresh_token: resp.token.refresh_token.clone(),
                            expires_at: resp.token.expires_in.timestamp(),
                            username: Some(resp.username.clone()),
                        };
                        let _ = crate::config::auth_store::save(&auth);
                        self.client.set_auth(auth).await;
                        self.username = Some(resp.username);
                        self.login.step = LoginStep::Input;
                        self.login.captcha_key = None;
                        self.input_mode = InputMode::Normal;
                    }
                    Err(e) => {
                        self.login.error = Some(e);
                        self.login.step = LoginStep::Input;
                        self.login.captcha_key = None;
                    }
                }
            }
            AppMessage::ImageFetched { url, protocol } => {
                self.cache.images_loading.remove(&url);
                self.cache.images.insert(url, protocol);
            }
            AppMessage::DebouncedCoverLoad(url) => {
                self.start_image_fetch(&url);
            }
            AppMessage::SongDetailFetched { node, index, detail } => {
                self.cache.detail_loading.remove(&detail.id);
                if let Some(songs) = self.cache.songs.get_mut(&node) {
                    if index < songs.len() && songs[index].id == detail.id {
                        songs[index] = detail;
                    }
                }
            }
        }
    }
}
