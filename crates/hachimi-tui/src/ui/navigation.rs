use serde::{Deserialize, Serialize};

/// Miller Columns 导航层级树中的节点类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NavNode {
    Root,
    Home,
    Search,
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
    Favorites,
    History,

    // 动态内容
    SongList { title: String },
    SongDetail { id: i64 },
    TagList,
    Tag { name: String },
    PlaylistDetail { id: i64 },
    UserDetail { id: i64 },
    SearchResults,
    SettingsPage,
}

impl NavNode {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Root => t!("nav.root"),
            Self::Home => t!("nav.home"),
            Self::Search => t!("nav.search"),
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
                Self::Search,
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
            Self::Library => vec![Self::MyPlaylists, Self::Favorites, Self::History],
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
        )
    }
}

/// 导航栈，追踪 Miller Columns 当前路径
#[derive(Debug, Clone)]
pub struct NavStack {
    /// 从根到当前的路径
    path: Vec<NavLevel>,
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

    pub fn is_root(&self) -> bool {
        self.path.len() == 1
    }

    pub fn push(&mut self, node: NavNode) {
        self.path.push(NavLevel { node, selected: 0 });
    }

    pub fn pop(&mut self) -> bool {
        if self.path.len() > 1 {
            self.path.pop();
            true
        } else {
            false
        }
    }

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
    pub is_editing: bool,
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
            is_editing: false,
        }
    }
}
