use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Zh,
}

impl Default for Lang {
    fn default() -> Self {
        Self::detect_system()
    }
}

impl Lang {
    /// 从系统环境变量检测语言，首次启动时使用
    fn detect_system() -> Self {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if val.starts_with("zh") {
                    return Lang::Zh;
                }
            }
        }
        Lang::En
    }
}

impl Lang {
    pub fn next(self) -> Self {
        match self {
            Lang::En => Lang::Zh,
            Lang::Zh => Lang::En,
        }
    }
}

static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT_LANG.load(Ordering::Relaxed) {
        1 => Lang::Zh,
        _ => Lang::En,
    }
}

macro_rules! t {
    ($key:expr) => {
        $crate::ui::i18n::tr($key)
    };
}

/// # Safety
/// All keys used with `t!()` must have entries in the translation tables.
/// Unknown keys will return `"???"`.
pub fn tr(key: &str) -> &'static str {
    match lang() {
        Lang::En => tr_en(key),
        Lang::Zh => tr_zh(key),
    }
}

fn tr_en(key: &str) -> &'static str {
    match key {
        // app
        "app.logged_in" => "logged in",
        "app.anonymous" => "anonymous",
        "app.email_password_required" => "Email and password required",
        "app.no_captcha_key" => "No captcha key",

        // help
        "help.title" => "Key Bindings",
        "help.close" => "j/k scroll  \u{00b7}  q / ? / Esc to close",
        "help.section.global" => "Global",
        "help.section.navigation" => "Navigation",
        "help.section.search" => "Search",
        "help.quit" => "Quit",
        "help.play_pause" => "Play / Pause",
        "help.next_prev" => "Next / Prev track",
        "help.volume" => "Volume up / down",
        "help.seek" => "Seek \u{00b1}5s",
        "help.play_mode" => "Cycle play mode",
        "help.player_view" => "Toggle player view",
        "help.search" => "Search",
        "help.help" => "This help",
        "help.logs" => "Show logs",
        "help.logout" => "Logout",
        "help.down_up" => "Down / Up",
        "help.drill_in" => "Drill in",
        "help.drill_out" => "Drill out",
        "help.top_bottom" => "Top / Bottom",
        "help.add_queue" => "Add to queue",
        "help.remove_queue" => "Remove from queue",
        "help.open_link" => "Open external link",
        "help.add_playlist" => "Add to playlist",
        "help.switch_type" => "Switch type",
        "help.switch_sort" => "Switch sort",
        "help.exit_search" => "Exit search",
        "help.fetch_danmaku" => "Fetch Bilibili danmaku to file",
        "help.section.danmaku" => "Danmaku",

        // logs
        "logs.title" => "Logs",
        "logs.empty" => "No logs yet",
        "logs.hint" => "j/k scroll  \u{00b7}  h/l pan  \u{00b7}  Esc/! close",

        // player
        "player.no_song" => "No song playing",
        "player.no_lyrics" => "No lyrics",

        // login
        "login.title" => "LOGIN",
        "login.email" => "Email",
        "login.password" => "Password",
        "login.hint_login" => "Login",
        "login.hint_quit" => "Quit",
        "login.generating_captcha" => "Generating captcha...",
        "login.captcha_opened" => "Captcha opened in browser",
        "login.continue_captcha" => "Continue after completing captcha",
        "login.logging_in" => "Logging in...",

        // queue
        "queue.empty" => "Queue is empty",
        "queue.hint" => "d remove \u{00b7} Enter play",

        // miller
        "miller.no_songs" => "No songs",
        "miller.no_playlists" => "No playlists",
        "miller.loading" => "Loading...",
        "miller.origin" => "Original",
        "miller.release_date" => "Released",
        "miller.crew" => "Credits",
        "miller.links" => "Links",
        "miller.links_hint" => "o to open",

        // navigation
        "nav.root" => "Root",
        "nav.home" => "Home",
        "nav.library" => "Library",
        "nav.queue" => "Queue",
        "nav.settings" => "Settings",
        "nav.latest" => "Latest",
        "nav.daily" => "Daily",
        "nav.weekly" => "Weekly",
        "nav.categories" => "Categories",
        "nav.playlists" => "Playlists",
        "nav.favorites" => "Favorites",
        "nav.history" => "History",
        "nav.detail" => "Detail",
        "nav.tags" => "Tags",
        "nav.playlist" => "Playlist",
        "nav.user" => "User",
        "nav.results" => "Results",
        "nav.settings_page" => "Settings",

        // search
        "search.song" => "song",
        "search.user" => "user",
        "search.playlist" => "playlist",
        "search.no_results" => "No results",
        "search.songs_count" => "Songs",
        "sort.relevance" => "relevance",
        "sort.newest" => "newest",
        "sort.oldest" => "oldest",

        // settings
        "settings.language" => "Language",
        "settings.play_mode" => "Play Mode",
        "settings.replay_gain" => "Loudness Norm",
        "settings.on" => "On",
        "settings.off" => "Off",
        "settings.sequential" => "Sequential",
        "settings.shuffle" => "Shuffle",
        "settings.repeat_one" => "Repeat One",
        "settings.hint" => "Enter/l to change \u{00b7} h/\u{2190} go back",
        "settings.desc.language" => "Interface display language",
        "settings.desc.play_mode" => "Playback order when a track finishes: sequential, shuffle, or repeat one",
        "settings.desc.replay_gain" => "Normalize volume across tracks to reduce loudness differences",
        "settings.lang.en.desc" => "Full English interface",
        "settings.lang.zh.desc" => "Simplified Chinese interface",
        "settings.cover_scale" => "Cover Scale",
        "settings.desc.cover_scale" => "Cover image scale in the browser preview (20%-100%)",

        _ => "???",
    }
}

fn tr_zh(key: &str) -> &'static str {
    match key {
        // app
        "app.logged_in" => "已登录",
        "app.anonymous" => "匿名",
        "app.email_password_required" => "请输入邮箱和密码",
        "app.no_captcha_key" => "验证码密钥缺失",

        // help
        "help.title" => "快捷键",
        "help.close" => "j/k 滚动  \u{00b7}  q / ? / Esc 关闭",
        "help.section.global" => "全局",
        "help.section.navigation" => "导航",
        "help.section.search" => "搜索",
        "help.quit" => "退出",
        "help.play_pause" => "播放 / 暂停",
        "help.next_prev" => "下一首 / 上一首",
        "help.volume" => "音量 +/-",
        "help.seek" => "快进/快退 \u{00b1}5s",
        "help.play_mode" => "切换播放模式",
        "help.player_view" => "切换播放器视图",
        "help.search" => "搜索",
        "help.help" => "帮助",
        "help.logs" => "显示日志",
        "help.logout" => "退出登录",
        "help.down_up" => "下 / 上",
        "help.drill_in" => "进入",
        "help.drill_out" => "返回",
        "help.top_bottom" => "顶部 / 底部",
        "help.add_queue" => "加入队列",
        "help.remove_queue" => "从队列移除",
        "help.open_link" => "打开外部链接",
        "help.add_playlist" => "加入歌单",
        "help.switch_type" => "切换类型",
        "help.switch_sort" => "切换排序",
        "help.exit_search" => "退出搜索",
        "help.fetch_danmaku" => "下载 B 站弹幕到文件",
        "help.section.danmaku" => "弹幕",

        // logs
        "logs.title" => "日志",
        "logs.empty" => "暂无日志",
        "logs.hint" => "j/k 滚动  \u{00b7}  h/l 左右滚动  \u{00b7}  Esc/! 关闭",

        // player
        "player.no_song" => "未在播放",
        "player.no_lyrics" => "无歌词",

        // login
        "login.title" => "登录",
        "login.email" => "邮箱",
        "login.password" => "密码",
        "login.hint_login" => "登录",
        "login.hint_quit" => "退出",
        "login.generating_captcha" => "正在生成验证码...",
        "login.captcha_opened" => "验证码已在浏览器中打开",
        "login.continue_captcha" => "完成验证码后按 Enter 继续",
        "login.logging_in" => "正在登录...",

        // queue
        "queue.empty" => "队列为空",
        "queue.hint" => "d 删除 \u{00b7} Enter 播放",

        // miller
        "miller.no_songs" => "暂无歌曲",
        "miller.no_playlists" => "暂无歌单",
        "miller.loading" => "加载中...",
        "miller.origin" => "原作",
        "miller.release_date" => "发行日期",
        "miller.crew" => "创作团队",
        "miller.links" => "外部链接",
        "miller.links_hint" => "按 o 打开",

        // navigation
        "nav.root" => "根",
        "nav.home" => "首页",
        "nav.library" => "曲库",
        "nav.queue" => "队列",
        "nav.settings" => "设置",
        "nav.latest" => "最新",
        "nav.daily" => "日推",
        "nav.weekly" => "周榜",
        "nav.categories" => "分类",
        "nav.playlists" => "歌单",
        "nav.favorites" => "收藏",
        "nav.history" => "历史",
        "nav.detail" => "详情",
        "nav.tags" => "标签",
        "nav.playlist" => "歌单",
        "nav.user" => "用户",
        "nav.results" => "结果",
        "nav.settings_page" => "设置",

        // search
        "search.song" => "歌曲",
        "search.user" => "用户",
        "search.playlist" => "歌单",
        "search.no_results" => "无结果",
        "search.songs_count" => "歌曲数",
        "sort.relevance" => "相关度",
        "sort.newest" => "最新",
        "sort.oldest" => "最早",

        // settings
        "settings.language" => "语言",
        "settings.play_mode" => "播放模式",
        "settings.replay_gain" => "响度均衡",
        "settings.on" => "开",
        "settings.off" => "关",
        "settings.sequential" => "顺序播放",
        "settings.shuffle" => "随机播放",
        "settings.repeat_one" => "单曲循环",
        "settings.hint" => "Enter/l 切换 \u{00b7} h/\u{2190} 返回",
        "settings.desc.language" => "界面显示语言",
        "settings.desc.play_mode" => "曲目结束后的播放顺序：顺序、随机或单曲循环",
        "settings.desc.replay_gain" => "均衡各曲目音量，减少响度差异",
        "settings.lang.en.desc" => "英文界面",
        "settings.lang.zh.desc" => "简体中文界面",
        "settings.cover_scale" => "封面缩放",
        "settings.desc.cover_scale" => "浏览视图中预览封面图的缩放比例 (20%-100%)",

        _ => tr_en(key),
    }
}
