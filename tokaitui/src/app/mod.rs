mod actions;
mod event;
mod render;

const UI_TICK_MS: u64 = 300;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use crossterm::event::Event;
use tokio::sync::mpsc;

use mambocore::MamboClient;
use crate::config::settings::Settings;
use crate::model::playlist::{PlaylistItem, PlaylistMetadata};
use crate::model::queue::QueueState;
use crate::model::song::PublicSongDetail;
use crate::model::user::PublicUserProfile;
use crate::player::engine::{PlayerEngine, PlayerEvent};
use crate::ui::log_view::LogStore;
use crate::ui::login::LoginState;
use crate::ui::lyrics::ParsedLyrics;
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
    /// 封面图片已上传到终端内存
    CoverReady {
        url: String,
        id: u32,
        upload_seq: Vec<u8>,
    },
}

/// 后台加载的数据
pub enum DataPayload {
    Songs(NavNode, Vec<PublicSongDetail>),
    Tags(Vec<String>),
    Playlists(Vec<PlaylistItem>),
    SearchUsers(Vec<PublicUserProfile>),
    SearchPlaylists(Vec<PlaylistMetadata>),
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
    /// 解析后的歌词（用于时间同步滚动）
    pub parsed_lyrics: ParsedLyrics,
    /// 展开页是否跟随播放状态（按 n/N 切歌后跟随，j/k 浏览后取消）
    pub follow_playback: bool,
}

/// 已上传到终端的封面条目
struct CoverEntry {
    id: u32,
    upload_seq: Vec<u8>,
}

/// 封面图片缓存：统一管理 URL→ID 映射、上传序列、加载状态
pub struct CoverCache {
    /// 完整条目（含上传序列，供缩放后重传）
    entries: HashMap<String, CoverEntry>,
    /// URL → image ID，借给渲染层（与 entries 保持同步）
    ids: HashMap<String, u32>,
    /// 正在下载的 URL
    loading: HashSet<String>,
    next_id: u32,
}

impl CoverCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ids: HashMap::new(),
            loading: HashSet::new(),
            next_id: 1,
        }
    }

    /// 封面是否已上传就绪
    pub fn is_ready(&self, url: &str) -> bool {
        self.entries.contains_key(url)
    }

    /// 封面是否正在下载
    pub fn is_loading(&self, url: &str) -> bool {
        self.loading.contains(url)
    }

    /// 已缓存封面数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 分配新 image ID
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 标记 URL 正在下载
    pub fn mark_loading(&mut self, url: String) {
        self.loading.insert(url);
    }

    /// 封面就绪，记录 ID 和上传序列
    pub fn mark_loaded(&mut self, url: String, id: u32, upload_seq: Vec<u8>) {
        self.loading.remove(&url);
        self.ids.insert(url.clone(), id);
        self.entries.insert(url, CoverEntry { id, upload_seq });
    }

    /// 淘汰任意一条旧记录，返回 (url, image_id)
    pub fn evict_one(&mut self) -> Option<(String, u32)> {
        let url = self.entries.keys().next()?.to_owned();
        let entry = self.entries.remove(&url)?;
        self.ids.remove(&url);
        Some((url, entry.id))
    }

    /// 所有已上传序列（供终端缩放后重传）
    pub fn all_upload_seqs(&self) -> impl Iterator<Item = &[u8]> {
        self.entries.values().map(|e| e.upload_seq.as_slice())
    }

    /// URL → image ID 映射，供渲染层借用
    pub fn id_map(&self) -> &HashMap<String, u32> {
        &self.ids
    }
}

pub struct DataCache {
    pub songs: HashMap<NavNode, Vec<PublicSongDetail>>,
    pub tags: Option<Vec<String>>,
    pub playlists: Option<Vec<PlaylistItem>>,
    pub search_users: Vec<PublicUserProfile>,
    pub search_playlists: Vec<PlaylistMetadata>,
    pub loading: HashSet<NavNode>,
    /// 正在补全详情的歌曲 ID
    pub(crate) detail_loading: HashSet<i64>,
    /// 队列项的完整歌曲详情缓存（按歌曲 ID）
    pub(crate) queue_song_detail: HashMap<i64, PublicSongDetail>,
    pub covers: CoverCache,
}

pub struct App {
    pub running: bool,
    pub settings: Settings,
    pub client: MamboClient,
    pub player: PlayerState,
    pub queue: QueueState,
    pub cache: DataCache,
    pub nav: NavStack,
    pub search: SearchState,
    pub input_mode: InputMode,
    pub login: LoginState,
    pub show_help: bool,
    pub help_scroll: u16,
    pub show_logs: bool,
    pub logs: LogStore,
    pub username: Option<String>,
    pub scroll_tick: u16,
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    msg_rx: mpsc::UnboundedReceiver<AppMessage>,
    /// 启动时待恢复的播放进度（毫秒），seek 后清零
    pub(crate) resume_position_ms: Option<u64>,
    /// 终端是否支持 Kitty 图形协议
    pub kitty_supported: bool,
    /// 待加载的封面（URL, 开始等待时刻），用于防抖
    pub pending_cover_load: Option<(String, std::time::Instant)>,
    /// 上帧已放置的封面 image ID，用于下帧清除不再显示的封面
    pub active_cover_ids: Vec<u32>,
    /// 终端缩放后需要在 draw() 之后重新上传 image data
    pub needs_cover_reupload: bool,
}

impl App {
    pub async fn new() -> Result<Self> {
        let settings = Settings::load()?;
        let client = MamboClient::new(None)?;
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        // 加载已保存的认证信息，并检查 token 是否过期
        let (has_auth, saved_username) = if let Ok(Some(auth)) = crate::config::auth_store::load() {
            let name = auth.username.clone();
            client.set_auth(auth.clone()).await;
            if let Some(event) = client.ensure_valid_auth().await {
                match event {
                    mambocore::AuthEvent::Refreshed(data) => {
                        let _ = crate::config::auth_store::save(&data);
                    }
                    mambocore::AuthEvent::Cleared => {
                        let _ = crate::config::auth_store::clear();
                    }
                }
            }
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

        let resume_position_ms = if has_auth && queue.current_index.is_some() {
            Some(queue.position_ms)
        } else {
            None
        };

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
                parsed_lyrics: ParsedLyrics::Empty,
                follow_playback: true,
            },
            queue,
            cache: DataCache {
                songs: HashMap::new(),
                tags: None,
                playlists: None,
                search_users: Vec::new(),
                search_playlists: Vec::new(),
                loading: HashSet::new(),
                detail_loading: HashSet::new(),
                queue_song_detail: HashMap::new(),
                covers: CoverCache::new(),
            },
            login: LoginState::new(),
            show_help: false,
            help_scroll: 0,
            show_logs: false,
            logs: LogStore::new(),
            username: saved_username,
            scroll_tick: 0,
            msg_tx,
            msg_rx,
            resume_position_ms,
            kitty_supported: crate::ui::kitty::is_supported(),
            pending_cover_load: None,
            active_cover_ids: Vec::new(),
            needs_cover_reupload: false,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();

        let result = self.main_loop(&mut terminal).await;

        // 退出时同步进度并持久化队列
        self.queue.position_ms = (self.player.bar.current_secs as u64) * 1000;
        let _ = self.queue.persist();

        ratatui::restore();

        result
    }

    async fn main_loop(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
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
                tokio::time::interval(std::time::Duration::from_millis(UI_TICK_MS));
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

        // 启动时仅恢复播放栏 UI，不自动播放
        if self.resume_position_ms.is_some() {
            if let Some(song) = self.queue.current_song() {
                self.player.bar.title = song.name.clone();
                self.player.bar.artist = song.artist.clone();
                self.player.bar.total_secs = song.duration_secs as u32;
                self.player.bar.current_secs =
                    (self.resume_position_ms.unwrap_or(0) / 1000) as u32;
                self.player.bar.cover_url = song.cover_url.clone();
            }
        }

        while self.running {
            terminal.draw(|f| self.render(f))?;
            // draw 结束后，将本帧收集的封面放置请求写入终端（光标定位放置，无 cursor-position 歧义）
            let _ = self.render_cover_placements();

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
