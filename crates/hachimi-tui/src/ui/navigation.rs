use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Miller Columns 导航层级树中的节点类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NavNode {
    Root,
    Home,
    Library,
    Queue,
    Settings,

    // Home 子项
    LatestReleases,
    DailyRecommend,
    WeeklyHot,
    Categories,

    // Library 子项
    MyPlaylists,
    #[allow(dead_code)] // TODO: 收藏功能
    Favorites,
    History,

    // 动态内容
    #[allow(dead_code)] // TODO: 歌曲列表页
    SongList { title: String },
    #[allow(dead_code)] // TODO: 歌曲详情页
    SongDetail { id: i64 },
    #[allow(dead_code)] // TODO: 标签列表页
    TagList,
    Tag { name: String },
    PlaylistDetail { id: i64 },
    UserDetail { id: i64 },
    #[allow(dead_code)] // TODO: 搜索结果页
    SearchResults,
    #[allow(dead_code)] // TODO: 设置页面
    SettingsPage,
}

impl NavNode {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Root => t!("nav.root"),
            Self::Home => t!("nav.home"),
            Self::Library => t!("nav.library"),
            Self::Queue => t!("nav.queue"),
            Self::Settings => t!("nav.settings"),
            Self::LatestReleases => t!("nav.latest"),
            Self::DailyRecommend => t!("nav.daily"),
            Self::WeeklyHot => t!("nav.weekly"),
            Self::Categories => t!("nav.categories"),
            Self::MyPlaylists => t!("nav.playlists"),
            Self::Favorites => t!("nav.favorites"),
            Self::History => t!("nav.history"),
            Self::SongList { title } => title,
            Self::SongDetail { .. } => t!("nav.detail"),
            Self::TagList => t!("nav.tags"),
            Self::Tag { name } => name,
            Self::PlaylistDetail { .. } => t!("nav.playlist"),
            Self::UserDetail { .. } => t!("nav.user"),
            Self::SearchResults => t!("nav.results"),
            Self::SettingsPage => t!("nav.settings_page"),
        }
    }

    pub fn children(&self) -> Vec<NavNode> {
        match self {
            Self::Root => vec![
                Self::Home,
                Self::Library,
                Self::Queue,
                Self::Settings,
            ],
            Self::Home => vec![
                Self::LatestReleases,
                Self::DailyRecommend,
                Self::WeeklyHot,
                Self::Categories,
            ],
            Self::Library => vec![Self::MyPlaylists, Self::History],
            _ => vec![],
        }
    }

    pub fn has_static_children(&self) -> bool {
        matches!(self, Self::Root | Self::Home | Self::Library)
    }

    pub fn needs_dynamic_data(&self) -> bool {
        matches!(
            self,
            Self::LatestReleases
                | Self::DailyRecommend
                | Self::WeeklyHot
                | Self::Categories
                | Self::Tag { .. }
                | Self::History
                | Self::MyPlaylists
                | Self::PlaylistDetail { .. }
                | Self::UserDetail { .. }
        )
    }
}

/// 导航栈，追踪 Miller Columns 当前路径
#[derive(Debug, Clone)]
pub struct NavStack {
    /// 从根到当前的路径
    path: Vec<NavLevel>,
    /// 退出子级时记忆光标位置，重新进入时恢复
    cursor_memory: HashMap<NavNode, usize>,
}

#[derive(Debug, Clone)]
pub struct NavLevel {
    pub node: NavNode,
    pub selected: usize,
}

impl NavStack {
    pub fn new() -> Self {
        Self {
            path: vec![NavLevel {
                node: NavNode::Root,
                selected: 0,
            }],
            cursor_memory: HashMap::new(),
        }
    }

    pub fn current(&self) -> &NavLevel {
        self.path.last().expect("nav stack never empty")
    }

    pub fn current_mut(&mut self) -> &mut NavLevel {
        self.path.last_mut().expect("nav stack never empty")
    }

    pub fn parent(&self) -> Option<&NavLevel> {
        if self.path.len() >= 2 {
            Some(&self.path[self.path.len() - 2])
        } else {
            None
        }
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    #[allow(dead_code)] // TODO: 导航状态判断
    pub fn is_root(&self) -> bool {
        self.path.len() == 1
    }

    pub fn push(&mut self, node: NavNode) {
        let selected = self.cursor_memory.get(&node).copied().unwrap_or(0);
        self.path.push(NavLevel { node, selected });
    }

    pub fn pop(&mut self) -> bool {
        if self.path.len() > 1 {
            let level = self.path.pop().unwrap();
            self.cursor_memory.insert(level.node, level.selected);
            true
        } else {
            false
        }
    }

    /// 检查导航栈中是否包含指定节点
    pub fn contains(&self, node: &NavNode) -> bool {
        self.path.iter().any(|l| l.node == *node)
    }

    /// 回退到栈中已有的 `node`，截断其上方的层级并重置选中索引。
    /// 返回是否找到并回退成功。
    pub fn pop_to(&mut self, node: &NavNode) -> bool {
        if let Some(pos) = self.path.iter().position(|l| l.node == *node) {
            for level in self.path.drain(pos + 1..) {
                self.cursor_memory.insert(level.node, level.selected);
            }
            self.path[pos].selected = 0;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)] // TODO: 面包屑导航
    pub fn path(&self) -> &[NavLevel] {
        &self.path
    }
}

/// 搜索状态
#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub search_type: SearchType,
    pub sort: SearchSort,
    pub cursor_pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SearchType {
    #[default]
    Song,
    User,
    Playlist,
}

impl SearchType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Song => t!("search.song"),
            Self::User => t!("search.user"),
            Self::Playlist => t!("search.playlist"),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Song => Self::User,
            Self::User => Self::Playlist,
            Self::Playlist => Self::Song,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SearchSort {
    #[default]
    Relevance,
    Newest,
    Oldest,
}

impl SearchSort {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Relevance => t!("sort.relevance"),
            Self::Newest => t!("sort.newest"),
            Self::Oldest => t!("sort.oldest"),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Relevance => Self::Newest,
            Self::Newest => Self::Oldest,
            Self::Oldest => Self::Relevance,
        }
    }
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            search_type: SearchType::default(),
            sort: SearchSort::default(),
            cursor_pos: 0,
        }
    }

    /// 清空查询和光标，保留 type/sort 偏好
    #[allow(dead_code)] // TODO: 搜索重置
    pub fn clear(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
    }
}
