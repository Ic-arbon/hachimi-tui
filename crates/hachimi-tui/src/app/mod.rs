mod actions;
mod event;
mod render;

use std::collections::{HashMap, HashSet};
use std::io;

use anyhow::Result;
use crossterm::{
    event::Event,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use tokio::sync::mpsc;

use crate::api::client::HachimiClient;
use crate::config::settings::Settings;
use crate::model::queue::QueueState;
use crate::model::song::PublicSongDetail;
use crate::player::engine::{PlayerEngine, PlayerEvent};
use crate::ui::log_view::LogStore;
use crate::ui::login::LoginState;
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
        detail: PublicSongDetail,
        data: Vec<u8>,
    },
    /// 音频下载失败
    AudioFetchError(String),
    /// 封面图处理完成（已 resize + 生成协议数据）
    ImageFetched { url: String, protocol: StatefulProtocol },
    /// API 数据加载完成
    DataLoaded(DataPayload),
    /// 错误通知
    Error(String),
    /// Captcha 生成结果 (captcha_key, url)
    CaptchaGenerated(std::result::Result<(String, String), String>),
    /// 登录结果
    LoginResult(std::result::Result<crate::model::auth::LoginResp, String>),
    /// 歌曲详情补全（搜索结果→完整详情）
    SongDetailFetched {
        node: NavNode,
        index: usize,
        detail: PublicSongDetail,
    },
    /// 防抖后触发封面加载
    DebouncedCoverLoad(String),
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

pub struct PlayerState {
    pub engine: PlayerEngine,
    pub bar: PlayerBarState,
    pub expanded: bool,
    pub volume: u8,
    pub is_muted: bool,
    /// 当前播放歌曲的完整详情（用于歌词等展示）
    pub current_detail: Option<PublicSongDetail>,
}

pub struct DataCache {
    pub songs: HashMap<NavNode, Vec<PublicSongDetail>>,
    pub tags: Vec<String>,
    pub loading: HashSet<NavNode>,
    pub images: HashMap<String, StatefulProtocol>,
    pub(crate) images_loading: HashSet<String>,
    pub picker: Option<Picker>,
    /// 正在补全详情的歌曲 ID
    pub(crate) detail_loading: HashSet<i64>,
    /// 最近一次渲染时图片 widget 的区域，用于后台预编码
    pub(crate) last_image_rect: ratatui::layout::Rect,
}

pub struct App {
    pub running: bool,
    pub settings: Settings,
    pub client: HachimiClient,
    pub player: PlayerState,
    pub queue: QueueState,
    pub cache: DataCache,
    pub nav: NavStack,
    pub search: SearchState,
    pub input_mode: InputMode,
    pub login: LoginState,
    pub show_help: bool,
    pub show_logs: bool,
    pub logs: LogStore,
    pub username: Option<String>,
    pub scroll_tick: u16,
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    msg_rx: mpsc::UnboundedReceiver<AppMessage>,
    pub(crate) cover_debounce: Option<tokio::task::JoinHandle<()>>,
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
        let engine = PlayerEngine::spawn()?;
        engine.set_volume(volume as f32 / 100.0);

        // 加载或创建播放队列
        let queue = QueueState::load_persisted().unwrap_or_else(|_| QueueState::new());

        Ok(Self {
            running: true,
            settings,
            client,
            nav: NavStack::new(),
            search: SearchState::new(),
            input_mode,
            player: PlayerState {
                engine,
                bar: PlayerBarState::default(),
                expanded: false,
                volume,
                is_muted: false,
                current_detail: None,
            },
            queue,
            cache: DataCache {
                songs: HashMap::new(),
                tags: Vec::new(),
                loading: HashSet::new(),
                images: HashMap::new(),
                images_loading: HashSet::new(),
                picker: None,
                detail_loading: HashSet::new(),
                last_image_rect: ratatui::layout::Rect::default(),
            },
            login: LoginState::new(),
            show_help: false,
            show_logs: false,
            logs: LogStore::new(),
            username: saved_username,
            scroll_tick: 0,
            msg_tx,
            msg_rx,
            cover_debounce: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.cache.picker = Some(
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
                match crossterm::event::read() {
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
        let mut player_rx = self.player.engine.take_event_receiver();
        tokio::spawn(async move {
            while let Some(event) = player_rx.recv().await {
                if player_tx.send(AppMessage::PlayerStateChanged(event)).is_err() {
                    break;
                }
            }
        });

        while self.running {
            terminal.draw(|f| self.render(f))?;

            // 等待至少一条消息
            if let Some(msg) = self.msg_rx.recv().await {
                self.handle_message(msg).await;
            }
            // 批量处理所有已积压的消息，避免每条消息都触发一次 draw
            while let Ok(msg) = self.msg_rx.try_recv() {
                self.handle_message(msg).await;
            }
        }
        Ok(())
    }
}
