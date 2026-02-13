use std::collections::{HashMap, HashSet};
use std::io;

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
use tokio::sync::mpsc;

use crate::api::client::HachimiClient;
use crate::api::endpoints::RecentQuery;
use crate::config::settings::Settings;
use crate::model::auth::LoginReq;
use crate::model::song::PublicSongDetail;
use crate::ui::login::{LoginState, LoginStep};
use crate::ui::navigation::{NavNode, NavStack, SearchState};
use crate::ui::player_bar::PlayerBarState;

/// 异步消息，从后台任务发送到主循环
pub enum AppMessage {
    /// 终端事件（由持久后台线程读取）
    TermEvent(Event),
    /// 播放状态更新
    PlayerTick,
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
    pub login: LoginState,
    pub show_help: bool,
    pub username: Option<String>,
    pub song_cache: HashMap<NavNode, Vec<PublicSongDetail>>,
    loading: HashSet<NavNode>,
    pub scroll_tick: u16,
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    msg_rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let settings = Settings::load()?;
        let client = HachimiClient::new(None)?;
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        // 加载已保存的认证信息
        let has_auth = if let Ok(Some(auth)) = crate::config::auth_store::load() {
            client.set_auth(auth).await;
            true
        } else {
            false
        };

        let volume = settings.player.volume;
        let input_mode = if has_auth {
            InputMode::Normal
        } else {
            InputMode::Login
        };

        Ok(Self {
            running: true,
            settings,
            client,
            nav: NavStack::new(),
            search: SearchState::new(),
            input_mode,
            player_expanded: false,
            player_bar: PlayerBarState {
                volume,
                ..Default::default()
            },
            login: LoginState::new(),
            show_help: false,
            username: None,
            song_cache: HashMap::new(),
            loading: HashSet::new(),
            scroll_tick: 0,
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

        let result = self.main_loop(&mut terminal).await;

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
                    KeyCode::Char('?') | KeyCode::Esc => self.show_help = false,
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
            (_, KeyCode::Char('L')) => {
                self.logout();
            }
            (_, KeyCode::Char(' ')) => {
                // TODO: 播放/暂停
            }
            (_, KeyCode::Char('n')) => {
                // TODO: 下一曲
            }
            (_, KeyCode::Char('N')) => {
                // TODO: 上一曲
            }
            (_, KeyCode::Char('+') | KeyCode::Char('=')) => {
                // TODO: 音量增大
            }
            (_, KeyCode::Char('-')) => {
                // TODO: 音量减小
            }
            (_, KeyCode::Char('>')) => {
                // TODO: 快进 5s
            }
            (_, KeyCode::Char('<')) => {
                // TODO: 快退 5s
            }
            (_, KeyCode::Char('s')) => {
                // TODO: 切换播放模式
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
                // TODO: 添加到队列
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
                // 取消 captcha，回到输入
                self.login.step = LoginStep::Input;
                self.login.captcha_key = None;
                self.login.error = None;
            }
            (_, KeyCode::Enter) => {
                // 用户在浏览器完成 captcha 后按 Enter 继续登录
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
            self.login.error = Some("Email and password required".to_string());
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
            self.login.error = Some("No captcha key".to_string());
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
        if self.loading.contains(node) || self.song_cache.contains_key(node) {
            return;
        }
        self.loading.insert(node.clone());
        let node_owned = node.clone();
        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
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
                    client.recommend_songs().await.map(|r| r.songs)
                }
                NavNode::WeeklyHot => {
                    client.hot_songs_weekly().await.map(|r| r.songs)
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
        }
    }

    async fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::TermEvent(ev) => {
                self.handle_event(ev);
            }
            AppMessage::PlayerTick => {
                self.scroll_tick = self.scroll_tick.wrapping_add(1);
                // TODO: 更新播放进度
            }
            AppMessage::DataLoaded(payload) => match payload {
                DataPayload::Songs(node, songs) => {
                    self.loading.remove(&node);
                    if !songs.is_empty() {
                        self.song_cache.insert(node, songs);
                    }
                }
            },
            AppMessage::Error(err) => {
                tracing::error!("{}", err);
            }
            AppMessage::CaptchaGenerated(result) => {
                match result {
                    Ok((captcha_key, url)) => {
                        self.login.captcha_key = Some(captcha_key);
                        self.login.step = LoginStep::WaitingCaptcha;
                        // 在浏览器中打开 captcha URL
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
                        // 登录失败回到 captcha 等待（可能 captcha 还有效）
                        // 若需要重新生成 captcha 则回到 Input
                        self.login.step = LoginStep::Input;
                        self.login.captcha_key = None;
                    }
                }
            }
        }
    }

    // — Miller Columns 导航方法 —

    fn current_list_len(&self) -> usize {
        let node = &self.nav.current().node;
        if node.has_static_children() {
            node.children().len()
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
    }

    fn nav_up(&mut self) {
        let sel = self.nav.current().selected;
        if sel > 0 {
            self.nav.current_mut().selected = sel - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
    }

    fn nav_drill_in(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
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
            }
        }
    }

    fn nav_drill_out(&mut self) {
        self.nav.pop();
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
    }

    fn nav_top(&mut self) {
        self.nav.current_mut().selected = 0;
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
    }

    fn nav_bottom(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.nav.current_mut().selected = len - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
    }

    // — 渲染 —

    fn render(&self, frame: &mut Frame) {
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
                } else {
                    self.render_miller(frame, chunks[1]);
                }
            }
        }

        self.render_player_bar(frame, chunks[2]);

        if self.show_help {
            crate::ui::help::render(frame, frame.area());
        }
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let status = if let Some(name) = &self.username {
            Span::styled(
                format!("  {name}"),
                crate::ui::theme::Theme::secondary(),
            )
        } else if self.client.is_authenticated_sync() {
            Span::styled(
                "  logged in",
                crate::ui::theme::Theme::secondary(),
            )
        } else {
            Span::styled(
                "  anonymous",
                crate::ui::theme::Theme::secondary(),
            )
        };

        let title_span = Span::styled("  HACHIMI", crate::ui::theme::Theme::title());
        let line = Line::from(vec![title_span, status]);
        let header = Paragraph::new(line);
        frame.render_widget(header, area);
    }

    fn render_miller(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::miller::render(
            frame,
            area,
            &self.nav,
            &self.song_cache,
            &self.loading,
            self.scroll_tick,
        );
    }

    fn render_player_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::player_bar::render(frame, area, &self.player_bar);
    }

    fn render_player_view(&self, _frame: &mut Frame, _area: ratatui::layout::Rect) {
        // TODO: 展开播放器视图
    }
}
