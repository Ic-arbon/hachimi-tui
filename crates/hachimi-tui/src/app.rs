use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use tokio::sync::mpsc;

use crate::api::client::HachimiClient;
use crate::api::endpoints::{RecentQuery, SongSearchQuery};
use crate::config::settings::{PlayMode, Settings};
use crate::model::auth::LoginReq;
use crate::model::queue::{MusicQueueItem, QueueState};
use crate::model::song::PublicSongDetail;
use crate::player::engine::{AudioSource, PlayerEngine, PlayerEvent};
use crate::ui::log_view::{LogLevel, LogStore};
use crate::ui::login::{LoginState, LoginStep};
use crate::ui::navigation::{NavNode, NavStack, SearchState};
use crate::ui::player_bar::PlayerBarState;

/// 异步消息，从后台任务发送到主循环
pub enum AppMessage {
    /// 终端事件（由持久后台线程读取）
    TermEvent(Event),
    /// 播放状态更新（UI 动画驱动）
    PlayerTick,
    /// 播放引擎事件
    PlayerStateChanged(PlayerEvent),
    /// 音频下载完成
    AudioFetched {
        title: String,
        artist: String,
        duration_secs: u32,
        data: Vec<u8>,
        cover_url: String,
    },
    /// 音频下载失败
    AudioFetchError(String),
    /// 封面图解码完成
    ImageFetched { url: String, img: image::DynamicImage },
    /// API 数据加载完成
    DataLoaded(DataPayload),
    /// 错误通知
    Error(String),
    /// Captcha 生成结果 (captcha_key, url)
    CaptchaGenerated(std::result::Result<(String, String), String>),
    /// 登录结果
    LoginResult(std::result::Result<crate::model::auth::LoginResp, String>),
}

/// 后台加载的数据
pub enum DataPayload {
    Songs(NavNode, Vec<PublicSongDetail>),
    Tags(Vec<String>),
}

/// 输入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Login,
}

pub struct App {
    pub running: bool,
    pub settings: Settings,
    pub client: HachimiClient,
    pub nav: NavStack,
    pub search: SearchState,
    pub input_mode: InputMode,
    pub player_expanded: bool,
    pub player_bar: PlayerBarState,
    pub player: PlayerEngine,
    pub volume: u8,
    pub is_muted: bool,
    pub queue: QueueState,
    pub login: LoginState,
    pub show_help: bool,
    pub show_logs: bool,
    pub logs: LogStore,
    pub username: Option<String>,
    pub song_cache: HashMap<NavNode, Vec<PublicSongDetail>>,
    pub tag_cache: Vec<String>,
    loading: HashSet<NavNode>,
    pub scroll_tick: u16,
    pub picker: Option<Picker>,
    pub image_cache: HashMap<String, StatefulProtocol>,
    image_loading: HashSet<String>,
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    msg_rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let settings = Settings::load()?;
        let client = HachimiClient::new(None)?;
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        // 加载已保存的认证信息，并检查 token 是否过期
        let (has_auth, saved_username) = if let Ok(Some(auth)) = crate::config::auth_store::load() {
            let name = auth.username.clone();
            client.set_auth(auth.clone()).await;
            client.ensure_valid_auth().await;
            let authenticated = client.is_authenticated().await;
            // 旧 auth 文件可能没有 username，从 JWT 提取 uid 后调 API 获取
            let name = if name.is_none() && authenticated {
                if let Some(uid) = crate::config::auth_store::extract_uid_from_token(&auth.access_token) {
                    match client.user_profile(uid).await {
                        Ok(profile) => {
                            let uname = profile.username.clone();
                            // 回存到 auth 文件
                            let mut updated = crate::config::auth_store::load()
                                .ok().flatten().unwrap_or(auth);
                            updated.username = Some(uname.clone());
                            let _ = crate::config::auth_store::save(&updated);
                            Some(uname)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                name
            };
            (authenticated, name)
        } else {
            (false, None)
        };

        crate::ui::i18n::set_lang(settings.display.language);

        let volume = settings.player.volume;
        let input_mode = if has_auth {
            InputMode::Normal
        } else {
            InputMode::Login
        };

        // 创建播放引擎
        let player = PlayerEngine::spawn()?;
        player.set_volume(volume as f32 / 100.0);

        // 加载或创建播放队列
        let queue = QueueState::load_persisted().unwrap_or_else(|_| QueueState::new());

        Ok(Self {
            running: true,
            settings,
            client,
            nav: NavStack::new(),
            search: SearchState::new(),
            input_mode,
            player_expanded: false,
            player_bar: PlayerBarState::default(),
            volume,
            is_muted: false,
            player,
            queue,
            login: LoginState::new(),
            show_help: false,
            show_logs: false,
            logs: LogStore::new(),
            username: saved_username,
            song_cache: HashMap::new(),
            tag_cache: Vec::new(),
            loading: HashSet::new(),
            scroll_tick: 0,
            picker: None,
            image_cache: HashMap::new(),
            image_loading: HashSet::new(),
            msg_tx,
            msg_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.picker = Some(
            Picker::from_query_stdio()
                .unwrap_or_else(|_| Picker::from_fontsize((8, 16)))
        );

        let result = self.main_loop(&mut terminal).await;

        // 退出时持久化队列
        let _ = self.queue.persist();

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        // 启动持久的事件读取线程，避免 select! + spawn_blocking 丢事件
        let event_tx = self.msg_tx.clone();
        std::thread::spawn(move || {
            loop {
                match event::read() {
                    Ok(ev) => {
                        if event_tx.send(AppMessage::TermEvent(ev)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // UI 定时 tick（驱动文字滚动动画、播放进度等）
        let tick_tx = self.msg_tx.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(300));
            loop {
                interval.tick().await;
                if tick_tx.send(AppMessage::PlayerTick).is_err() {
                    break;
                }
            }
        });

        // 监听播放引擎事件，转发为 AppMessage
        let player_tx = self.msg_tx.clone();
        let mut player_rx = self.player.subscribe();
        tokio::spawn(async move {
            while player_rx.changed().await.is_ok() {
                let event = player_rx.borrow().clone();
                if player_tx.send(AppMessage::PlayerStateChanged(event)).is_err() {
                    break;
                }
            }
        });

        while self.running {
            terminal.draw(|f| self.render(f))?;

            if let Some(msg) = self.msg_rx.recv().await {
                self.handle_message(msg).await;
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
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
                let vol = (self.volume as u16 + 5).min(100) as u8;
                self.volume = vol;
                self.player.set_volume(vol as f32 / 100.0);
            }
            (_, KeyCode::Char('-')) => {
                let vol = self.volume.saturating_sub(5);
                self.volume = vol;
                self.player.set_volume(vol as f32 / 100.0);
            }
            (_, KeyCode::Char('>')) => {
                if self.player_bar.has_song() {
                    let new_pos = (self.player_bar.current_secs + 5)
                        .min(self.player_bar.total_secs);
                    self.player.seek(Duration::from_secs(new_pos as u64));
                }
            }
            (_, KeyCode::Char('<')) => {
                if self.player_bar.has_song() {
                    let new_pos = self.player_bar.current_secs.saturating_sub(5);
                    self.player.seek(Duration::from_secs(new_pos as u64));
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
                self.player_expanded = !self.player_expanded;
            }
            (_, KeyCode::Char('/')) => {
                self.input_mode = InputMode::Search;
                self.search.is_editing = true;
            }

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

    /// 第一步：校验输入 → 异步生成 captcha
    fn start_captcha(&mut self) {
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
    fn submit_login(&mut self) {
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
                device_info: "hachimi-tui".to_string(),
                captcha_key,
            };

            let result = client.login(&req).await;
            let _ = tx.send(AppMessage::LoginResult(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    fn logout(&mut self) {
        let _ = crate::config::auth_store::clear();

        let client = self.client.clone();
        tokio::spawn(async move {
            client.clear_auth().await;
        });

        self.username = None;
        self.song_cache.clear();
        self.loading.clear();
        self.login = LoginState::new();
        self.input_mode = InputMode::Login;
    }

    fn load_node_data(&mut self, node: &NavNode) {
        // Categories 用 tag_cache 而非 song_cache
        if *node == NavNode::Categories {
            if self.loading.contains(node) || !self.tag_cache.is_empty() {
                return;
            }
        } else if self.loading.contains(node) || self.song_cache.contains_key(node) {
            return;
        }
        self.loading.insert(node.clone());
        let node_owned = node.clone();
        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            // Categories 走单独的 tag 加载流程
            if node_owned == NavNode::Categories {
                let result = if client.is_authenticated().await {
                    client.recommend_tags().await
                } else {
                    client.recommend_tags_anonymous().await
                };
                match result {
                    Ok(resp) => {
                        let names: Vec<String> = resp.result.into_iter().map(|t| t.name).collect();
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Tags(names)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Tags(vec![])));
                        let _ = tx.send(AppMessage::Error(e.to_string()));
                    }
                }
                return;
            }

            let result = match &node_owned {
                NavNode::LatestReleases => client
                    .recent_songs(&RecentQuery {
                        cursor: None,
                        limit: 30,
                        after: None,
                    })
                    .await
                    .map(|r| r.songs),
                NavNode::DailyRecommend => {
                    let resp = if client.is_authenticated().await {
                        client.recommend_songs().await
                    } else {
                        client.recommend_songs_anonymous().await
                    };
                    resp.map(|r| r.songs)
                }
                NavNode::WeeklyHot => {
                    client.hot_songs_weekly().await.map(|r| r.songs)
                }
                NavNode::Tag { name } => {
                    client
                        .search_songs(&SongSearchQuery {
                            q: String::new(),
                            limit: Some(30),
                            offset: None,
                            filter: Some(format!("tags = \"{}\"", name)),
                            sort_by: Some("release_time_desc".to_string()),
                        })
                        .await
                        .map(|r| r.hits.into_iter().map(|s| s.into_song_detail()).collect())
                }
                _ => return,
            };

            match result {
                Ok(songs) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        node_owned, songs,
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        node_owned,
                        vec![],
                    )));
                    let _ = tx.send(AppMessage::Error(e.to_string()));
                }
            }
        });
    }

    fn maybe_load_preview_data(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
        if node.has_static_children() {
            let children = node.children();
            if let Some(child) = children.get(sel) {
                if child.needs_dynamic_data() {
                    let child = child.clone();
                    self.load_node_data(&child);
                }
            }
        } else if node == NavNode::Categories {
            // 加载选中标签的歌曲预览
            if let Some(tag_name) = self.tag_cache.get(sel).cloned() {
                let tag_node = NavNode::Tag { name: tag_name };
                self.load_node_data(&tag_node);
            }
        }
    }

    async fn handle_message(&mut self, msg: AppMessage) {
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
                        self.player_bar.is_playing = true;
                        self.player_bar.is_loading = false;
                    }
                    PlayerEvent::Paused => {
                        self.player_bar.is_playing = false;
                    }
                    PlayerEvent::Stopped => {
                        self.player_bar.is_playing = false;
                        self.player_bar.title.clear();
                        self.player_bar.artist.clear();
                        self.player_bar.current_secs = 0;
                        self.player_bar.total_secs = 0;
                    }
                    PlayerEvent::Progress { position_secs, duration_secs } => {
                        self.player_bar.current_secs = position_secs;
                        self.player_bar.total_secs = duration_secs;
                    }
                    PlayerEvent::TrackEnded => {
                        self.play_next();
                    }
                    PlayerEvent::Error(msg) => {
                        self.player_bar.is_loading = false;
                        self.logs.push(LogLevel::Error, msg);
                    }
                    PlayerEvent::Loading => {
                        self.player_bar.is_loading = true;
                    }
                }
            }
            AppMessage::AudioFetched { title, artist, duration_secs, data, cover_url } => {
                self.player_bar.title = title;
                self.player_bar.artist = artist;
                self.player_bar.total_secs = duration_secs;
                self.player_bar.current_secs = 0;
                self.player_bar.is_loading = false;
                self.player_bar.cover_url = cover_url.clone();
                self.player.play(AudioSource::Buffered(data), duration_secs);
                self.start_image_fetch(&cover_url);
            }
            AppMessage::AudioFetchError(err) => {
                self.player_bar.is_loading = false;
                self.logs.push(LogLevel::Error, err);
            }
            AppMessage::DataLoaded(payload) => match payload {
                DataPayload::Songs(node, songs) => {
                    self.loading.remove(&node);
                    if !songs.is_empty() {
                        self.song_cache.insert(node, songs);
                    }
                }
                DataPayload::Tags(tags) => {
                    self.loading.remove(&NavNode::Categories);
                    self.tag_cache = tags;
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
            AppMessage::ImageFetched { url, img } => {
                self.image_loading.remove(&url);
                if let Some(picker) = &mut self.picker {
                    let protocol = picker.new_resize_protocol(img);
                    self.image_cache.insert(url, protocol);
                }
            }
        }
    }

    // — 播放控制方法 —

    fn toggle_play_pause(&mut self) {
        if self.player_bar.is_playing {
            self.player.pause();
        } else if self.player_bar.has_song() {
            self.player.resume();
        } else if let Some(song) = self.queue.current_song().cloned() {
            self.start_audio_fetch(song.id, &song.name, &song.artist, song.duration_secs as u32);
        }
    }

    fn play_next(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.next_with_mode(&mode).cloned() {
            self.start_audio_fetch(item.id, &item.name, &item.artist, item.duration_secs as u32);
        }
    }

    fn play_prev(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.prev_with_mode(&mode).cloned() {
            self.start_audio_fetch(item.id, &item.name, &item.artist, item.duration_secs as u32);
        }
    }

    /// 获取当前 Miller Columns 选中的歌曲
    fn selected_song(&self) -> Option<&PublicSongDetail> {
        let node = &self.nav.current().node;
        let sel = self.nav.current().selected;
        if !node.has_static_children() {
            self.song_cache.get(node).and_then(|songs| songs.get(sel))
        } else {
            None
        }
    }

    fn song_to_queue_item(song: &PublicSongDetail) -> MusicQueueItem {
        MusicQueueItem {
            id: song.id,
            display_id: song.display_id.clone(),
            name: song.title.clone(),
            artist: song.uploader_name.clone(),
            duration_secs: song.duration_seconds,
            cover_url: song.cover_url.clone(),
            explicit: song.explicit,
            audio_url: song.audio_url.clone(),
            gain: song.gain,
        }
    }

    fn add_selected_to_queue(&mut self) {
        if let Some(song) = self.selected_song().cloned() {
            let item = Self::song_to_queue_item(&song);
            self.queue.add(item);
        }
    }

    /// 异步获取歌曲详情 → 下载音频 → 发送 AudioFetched
    fn start_audio_fetch(&mut self, song_id: i64, title: &str, artist: &str, duration_secs: u32) {
        self.player_bar.is_loading = true;
        self.player_bar.title = title.to_string();
        self.player_bar.artist = artist.to_string();

        let tx = self.msg_tx.clone();
        let client = self.client.clone();
        let title = title.to_string();
        let artist = artist.to_string();

        tokio::spawn(async move {
            // 第一步：获取歌曲详情拿到 audio_url
            let detail = match client.song_detail_by_id(song_id).await {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(AppMessage::AudioFetchError(
                        format!("获取歌曲详情失败: {e}"),
                    ));
                    return;
                }
            };

            let cover_url = detail.cover_url.clone();
            let audio_url = &detail.audio_url;
            if audio_url.is_empty() {
                let _ = tx.send(AppMessage::AudioFetchError(
                    "歌曲无音频地址".to_string(),
                ));
                return;
            }

            // 第二步：下载音频数据
            match client.get_audio_stream(audio_url).await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx.send(AppMessage::AudioFetchError(
                            format!("音频请求返回 {status}: {body}"),
                        ));
                        return;
                    }

                    match resp.bytes().await {
                        Ok(bytes) => {
                            if bytes.is_empty() {
                                let _ = tx.send(AppMessage::AudioFetchError(
                                    "音频数据为空".to_string(),
                                ));
                                return;
                            }
                            let _ = tx.send(AppMessage::AudioFetched {
                                title,
                                artist,
                                duration_secs,
                                data: bytes.to_vec(),
                                cover_url,
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::AudioFetchError(
                                format!("下载音频失败: {e}"),
                            ));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::AudioFetchError(
                        format!("请求音频失败: {e}"),
                    ));
                }
            }
        });
    }

    // — Miller Columns 导航方法 —

    fn current_list_len(&self) -> usize {
        let node = &self.nav.current().node;
        if *node == NavNode::Settings {
            crate::ui::settings_view::ITEMS_COUNT
        } else if node.has_static_children() {
            node.children().len()
        } else if *node == NavNode::Categories {
            self.tag_cache.len()
        } else if let Some(songs) = self.song_cache.get(node) {
            songs.len()
        } else {
            0
        }
    }

    fn nav_down(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            let sel = self.nav.current().selected;
            if sel + 1 < len {
                self.nav.current_mut().selected = sel + 1;
                self.scroll_tick = 0;
            }
        }
        self.maybe_load_preview_data();
        self.maybe_load_cover_image();
    }

    fn nav_up(&mut self) {
        let sel = self.nav.current().selected;
        if sel > 0 {
            self.nav.current_mut().selected = sel - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
        self.maybe_load_cover_image();
    }

    fn nav_drill_in(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
        if node == NavNode::Settings {
            crate::ui::settings_view::cycle_setting(&mut self.settings, sel);
            // 播放模式已直接修改 settings，无需同步
            let _ = self.settings.save();
            return;
        }
        if node.has_static_children() {
            let children = node.children();
            if sel < children.len() {
                let child = children[sel].clone();
                if child.needs_dynamic_data() {
                    self.load_node_data(&child);
                }
                self.nav.push(child);
                self.scroll_tick = 0;
                self.maybe_load_preview_data();
                self.maybe_load_cover_image();
            }
        } else if node == NavNode::Categories {
            // 进入选中的标签
            if let Some(tag_name) = self.tag_cache.get(sel).cloned() {
                let tag_node = NavNode::Tag { name: tag_name };
                self.load_node_data(&tag_node);
                self.nav.push(tag_node);
                self.scroll_tick = 0;
                self.maybe_load_preview_data();
                self.maybe_load_cover_image();
            }
        } else {
            // 当前节点是歌曲列表，按 Enter 播放选中歌曲
            if let Some(songs) = self.song_cache.get(&node).cloned() {
                if sel < songs.len() {
                    // 替换队列为当前列表所有歌曲
                    self.queue.clear();
                    for song in &songs {
                        self.queue.add(Self::song_to_queue_item(song));
                    }
                    self.queue.current_index = Some(sel);

                    // 播放选中歌曲
                    let song = &songs[sel];
                    self.start_audio_fetch(
                        song.id, &song.title, &song.uploader_name,
                        song.duration_seconds as u32,
                    );
                }
            }
        }
    }

    fn nav_drill_out(&mut self) {
        self.nav.pop();
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
        self.maybe_load_cover_image();
    }

    fn nav_top(&mut self) {
        self.nav.current_mut().selected = 0;
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
        self.maybe_load_cover_image();
    }

    fn nav_bottom(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.nav.current_mut().selected = len - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
        self.maybe_load_cover_image();
    }

    // — 图片加载 —

    fn start_image_fetch(&mut self, url: &str) {
        if url.is_empty() || self.image_cache.contains_key(url) || self.image_loading.contains(url) {
            return;
        }
        self.image_loading.insert(url.to_string());
        let tx = self.msg_tx.clone();
        let client = self.client.clone();
        let url = url.to_string();

        tokio::spawn(async move {
            let resp = match client.get_audio_stream(&url).await {
                Ok(r) if r.status().is_success() => r,
                _ => return,
            };
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                _ => return,
            };
            // 图片解码在 blocking 线程池执行，避免阻塞主循环
            let data = bytes.to_vec();
            let decoded = tokio::task::spawn_blocking(move || {
                image::load_from_memory(&data).ok()
            })
            .await;
            if let Ok(Some(img)) = decoded {
                let _ = tx.send(AppMessage::ImageFetched { url, img });
            }
        });
    }

    fn maybe_load_cover_image(&mut self) {
        if let Some(song) = self.selected_song().cloned() {
            if !song.cover_url.is_empty() {
                let url = song.cover_url.clone();
                self.start_image_fetch(&url);
            }
        }
    }

    // — 渲染 —

    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);

        match self.input_mode {
            InputMode::Login => {
                crate::ui::login::render(frame, chunks[1], &self.login);
            }
            _ => {
                if self.player_expanded {
                    self.render_player_view(frame, chunks[1]);
                } else if self.nav.current().node == NavNode::Settings {
                    self.render_settings(frame, chunks[1]);
                } else {
                    self.render_miller(frame, chunks[1]);
                }
            }
        }

        self.render_player_bar(frame, chunks[2]);

        if self.show_logs {
            crate::ui::log_view::render(frame, frame.area(), &self.logs);
        }

        if self.show_help {
            crate::ui::help::render(frame, frame.area());
        }
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::Alignment;
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;
        use unicode_width::UnicodeWidthStr;

        let status = if let Some(name) = &self.username {
            Span::styled(
                format!("  {name}"),
                crate::ui::theme::Theme::secondary(),
            )
        } else if self.client.is_authenticated_sync() {
            Span::styled(
                format!("  {}", t!("app.logged_in")),
                crate::ui::theme::Theme::secondary(),
            )
        } else {
            Span::styled(
                format!("  {}", t!("app.anonymous")),
                crate::ui::theme::Theme::secondary(),
            )
        };

        let title_span = Span::styled("  HACHIMI", crate::ui::theme::Theme::title());

        // 右侧色块段
        let mode_str = match self.settings.player.default_play_mode {
            PlayMode::Sequential => " [>] ",
            PlayMode::Shuffle => " [x] ",
            PlayMode::RepeatOne => " [1] ",
        };
        let vol_str = if self.is_muted {
            " vol -- ".to_string()
        } else {
            format!(" vol {}% ", self.volume)
        };
        let now = chrono::Local::now();
        let time_str = now.format(" %H:%M ").to_string();

        let block_bg = Style::default().fg(Color::Black).bg(Color::DarkGray);
        let block_accent = Style::default().fg(Color::Black).bg(Color::Cyan);

        let mut right_spans: Vec<Span> = Vec::new();

        if self.logs.unread_count > 0 {
            right_spans.push(Span::styled(
                format!(" ! {} ", self.logs.unread_count),
                Style::default().fg(Color::White).bg(Color::Red),
            ));
        }
        right_spans.push(Span::styled(mode_str, block_bg));
        right_spans.push(Span::styled(vol_str, block_accent));
        right_spans.push(Span::styled(time_str.clone(), block_bg));

        let right_width: u16 = right_spans
            .iter()
            .map(|s| s.content.width() as u16)
            .sum();

        // 左侧
        let left = Line::from(vec![title_span, status]);
        let left_p = Paragraph::new(left);

        let right_p = Paragraph::new(Line::from(right_spans))
            .alignment(Alignment::Right);

        use ratatui::layout::{Constraint as C, Direction as D, Layout as L};
        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([C::Min(1), C::Length(right_width)])
            .split(area);

        frame.render_widget(left_p, cols[0]);
        frame.render_widget(right_p, cols[1]);
    }

    fn render_miller(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::miller::render(
            frame,
            area,
            &self.nav,
            &self.song_cache,
            &self.tag_cache,
            &self.loading,
            self.scroll_tick,
            &self.settings,
            &mut self.image_cache,
        );
    }

    fn render_player_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::player_bar::render(frame, area, &self.player_bar);
    }

    fn render_settings(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint as C, Direction as D, Layout as L};
        use ratatui::style::{Modifier, Style};
        use ratatui::widgets::{List, ListItem};

        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([
                C::Percentage(15),
                C::Percentage(45),
                C::Percentage(40),
            ])
            .split(area);

        // Left: Root's children as parent column
        if let Some(parent) = self.nav.parent() {
            let children = parent.node.children();
            let items: Vec<ListItem> = children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let style = if i == parent.selected {
                        crate::ui::theme::Theme::secondary().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" {}", child.display_name())).style(style)
                })
                .collect();
            let list = List::new(items);
            frame.render_widget(list, cols[0]);
        }

        // Center: settings items
        let selected = self.nav.current().selected;
        crate::ui::settings_view::render_list(frame, cols[1], &self.settings, selected);

        // Right: hint
        crate::ui::settings_view::render_hint(frame, cols[2]);
    }

    fn render_player_view(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::player_view::render(
            frame,
            area,
            &self.player_bar,
            &mut self.image_cache,
        );
    }
}
