use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::config::settings::PlayMode;
use crate::player::engine::{AudioSource, PlayerEvent};
use crate::ui::log_view::LogLevel;
use crate::ui::login::LoginStep;
use crate::ui::navigation::NavNode;

use super::{App, AppMessage, DataPayload, InputMode};

const VOLUME_STEP: u8 = 5;
const MAX_VOLUME: u8 = 100;
const SEEK_STEP_SECS: u32 = 5;

impl App {
    pub(crate) fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            // 帮助浮层打开时，只响应关闭操作
            if self.show_help {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc => {
                        self.show_help = false;
                        self.help_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.help_scroll = self.help_scroll.saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.help_scroll = self.help_scroll.saturating_sub(1);
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
                    KeyCode::Char('h') | KeyCode::Left => self.logs.scroll_left(),
                    KeyCode::Char('l') | KeyCode::Right => self.logs.scroll_right(),
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

    fn adjust_volume(&mut self, delta: i16) {
        let vol = (self.player.volume as i16 + delta).clamp(0, MAX_VOLUME as i16) as u8;
        self.player.volume = vol;
        self.player.engine.set_volume(vol as f32 / MAX_VOLUME as f32);
    }

    fn seek_relative(&mut self, delta_secs: i32) {
        if self.player.bar.has_song() {
            let new_pos = (self.player.bar.current_secs as i64 + delta_secs as i64)
                .clamp(0, self.player.bar.total_secs as i64) as u32;
            self.player.engine.seek(Duration::from_secs(new_pos as u64));
        }
    }

    /// 处理 expanded 和 normal 共享的全局键绑定，返回 true 表示已处理
    fn handle_global_key(&mut self, key: KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.running = false;
            }
            (_, KeyCode::Char('?')) => self.show_help = true,
            (_, KeyCode::Char('!')) => {
                self.show_logs = true;
                self.logs.mark_read();
            }
            (_, KeyCode::Char('L')) => self.logout(),
            (_, KeyCode::Char(' ')) => self.toggle_play_pause(),
            (_, KeyCode::Char('n')) => self.play_next(),
            (_, KeyCode::Char('N')) => self.play_prev(),
            (_, KeyCode::Char('+') | KeyCode::Char('=')) => self.adjust_volume(VOLUME_STEP as i16),
            (_, KeyCode::Char('-')) => self.adjust_volume(-(VOLUME_STEP as i16)),
            (_, KeyCode::Char('>')) => self.seek_relative(SEEK_STEP_SECS as i32),
            (_, KeyCode::Char('<')) => self.seek_relative(-(SEEK_STEP_SECS as i32)),
            (_, KeyCode::Char('s')) => {
                self.settings.player.default_play_mode = match self.settings.player.default_play_mode {
                    PlayMode::Sequential => PlayMode::Shuffle,
                    PlayMode::Shuffle => PlayMode::RepeatOne,
                    PlayMode::RepeatOne => PlayMode::Sequential,
                };
            }
            _ => return false,
        }
        true
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.handle_global_key(key) {
            return;
        }

        if self.player.expanded {
            // 展开页专属键
            match (key.modifiers, key.code) {
                (_, KeyCode::Char('i')) => self.player.expanded = false,
                (_, KeyCode::Char('j') | KeyCode::Down) => self.nav_down(),
                (_, KeyCode::Char('k') | KeyCode::Up) => self.nav_up(),
                (_, KeyCode::Char('g')) => self.nav_top(),
                (_, KeyCode::Char('G')) => self.nav_bottom(),
                (_, KeyCode::Char('h') | KeyCode::Left) => self.player.expanded = false,
                (_, KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter) => {
                    self.play_expanded_song();
                }
                _ => {}
            }
            return;
        }

        // Normal 模式专属键
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('i')) => {
                self.player.expanded = true;
                self.player.follow_playback = self.player.current_detail.is_some();
            }
            (_, KeyCode::Char('/')) => {
                self.search.clear();
                self.input_mode = InputMode::Search;
            }
            (_, KeyCode::Tab) => {
                if self.nav.contains(&NavNode::SearchResults) {
                    self.search.search_type = self.search.search_type.next();
                    self.nav.current_mut().selected = 0;
                }
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => self.nav_down(),
            (_, KeyCode::Char('k') | KeyCode::Up) => self.nav_up(),
            (_, KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter) => self.nav_drill_in(),
            (_, KeyCode::Char('h') | KeyCode::Left) => self.nav_drill_out(),
            (_, KeyCode::Char('g')) => self.nav_top(),
            (_, KeyCode::Char('G')) => self.nav_bottom(),
            (_, KeyCode::Char('a')) => self.add_selected_to_queue(),
            (_, KeyCode::Char('d')) => self.remove_from_queue(),
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
            }
            (_, KeyCode::Enter) => {
                self.input_mode = InputMode::Normal;
                if !self.search.query.trim().is_empty() {
                    self.execute_search();
                    if !self.nav.pop_to(&NavNode::SearchResults) {
                        self.nav.push(NavNode::SearchResults);
                    }
                }
            }
            (_, KeyCode::Tab) => {
                self.search.search_type = self.search.search_type.next();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.search.sort = self.search.sort.next();
            }
            (_, KeyCode::Left) => {
                if self.search.cursor_pos > 0 {
                    self.search.cursor_pos -= 1;
                }
            }
            (_, KeyCode::Right) => {
                if self.search.cursor_pos < self.search.query.chars().count() {
                    self.search.cursor_pos += 1;
                }
            }
            (_, KeyCode::Backspace) => {
                if self.search.cursor_pos > 0 {
                    self.search.cursor_pos -= 1;
                    let byte_idx = self.search.query.char_indices()
                        .nth(self.search.cursor_pos).map(|(i, _)| i).unwrap_or(self.search.query.len());
                    self.search.query.remove(byte_idx);
                }
            }
            (_, KeyCode::Char(c)) => {
                let byte_idx = self.search.query.char_indices()
                    .nth(self.search.cursor_pos).map(|(i, _)| i).unwrap_or(self.search.query.len());
                self.search.query.insert(byte_idx, c);
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
                        self.player.parsed_lyrics = crate::ui::lyrics::ParsedLyrics::Empty;
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
                let gain = if self.settings.player.replay_gain {
                    detail.gain
                } else {
                    None
                };
                let cover_url = detail.cover_url.clone();
                self.player.parsed_lyrics = crate::ui::lyrics::parse(&detail.lyrics);
                self.player.current_detail = Some(detail);
                self.player.engine.play(AudioSource::Buffered(data), duration_secs, gain);
                if let Some(pos_ms) = self.resume_position_ms.take() {
                    self.player.engine.seek(std::time::Duration::from_millis(pos_ms));
                    self.player.bar.current_secs = (pos_ms / 1000) as u32;
                }
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
                    self.after_nav_move();
                }
                DataPayload::Tags(tags) => {
                    self.cache.loading.remove(&NavNode::Categories);
                    self.cache.tags = Some(tags);
                    self.after_nav_move();
                }
                DataPayload::Playlists(playlists) => {
                    self.cache.loading.remove(&NavNode::MyPlaylists);
                    self.cache.playlists = Some(playlists);
                    self.after_nav_move();
                }
                DataPayload::SearchUsers(users) => {
                    self.cache.loading.remove(&NavNode::SearchResults);
                    self.cache.search_users = users;
                    self.after_nav_move();
                }
                DataPayload::SearchPlaylists(playlists) => {
                    self.cache.loading.remove(&NavNode::SearchResults);
                    self.cache.search_playlists = playlists;
                    self.after_nav_move();
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
                        self.resume_playback();
                    }
                    Err(e) => {
                        self.login.error = Some(e);
                        self.login.step = LoginStep::Input;
                        self.login.captcha_key = None;
                    }
                }
            }
            AppMessage::ImageFetched { url, protocol, raw_bytes } => {
                self.cache.images_loading.remove(&url);
                self.cache.image_bytes.insert(url.clone(), raw_bytes);
                self.cache.images.insert(url.clone(), protocol);
                self.cache.image_order.push(url);
                self.cache.evict_images_if_needed();
            }
            AppMessage::DebouncedCoverLoad(url) => {
                self.start_image_fetch(&url);
            }
            AppMessage::SongDetailFetched { node, index, detail } => {
                self.cache.detail_loading.remove(&detail.id);
                if node == NavNode::Queue {
                    self.cache.queue_song_detail.insert(detail.id, detail);
                } else if let Some(songs) = self.cache.songs.get_mut(&node) {
                    if index < songs.len() && songs[index].id == detail.id {
                        songs[index] = detail;
                    }
                }
            }
        }
    }
}
