use std::collections::{HashMap, HashSet};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{List, ListItem, ListState, Paragraph},
};

use super::constants::{MILLER_PARENT_PCT, MILLER_CURRENT_PCT, MILLER_PREVIEW_PCT, MILLER_TWO_COL_PCT};
use super::format::song_list_line;
use super::navigation::{NavNode, NavStack, SearchType};
use super::preview::render_preview_column;
use super::theme::Theme;
use crate::config::settings::Settings;
use crate::model::playlist::PlaylistItem;
use crate::model::queue::QueueState;
use crate::model::song::PublicSongDetail;
use crate::model::user::PublicUserProfile;
use crate::model::playlist::PlaylistMetadata;

/// render_column 和 render_preview_column 共享的只读数据
pub struct ColumnData<'a> {
    pub song_cache: &'a HashMap<NavNode, Vec<PublicSongDetail>>,
    pub tag_cache: &'a [String],
    pub playlist_cache: &'a [PlaylistItem],
    pub queue: &'a QueueState,
    pub queue_detail: &'a HashMap<i64, PublicSongDetail>,
    pub loading: &'a HashSet<NavNode>,
    pub settings: &'a Settings,
    pub search_type: SearchType,
    pub search_users: &'a [PublicUserProfile],
    pub search_playlists: &'a [PlaylistMetadata],
    /// URL → Kitty image ID（已上传到终端的封面）
    pub covers: &'a HashMap<String, u32>,
}

/// 渲染 Miller Columns 三栏布局
pub fn render(
    frame: &mut Frame,
    area: Rect,
    nav: &NavStack,
    data: &ColumnData,
    scroll_tick: u16,
) {
    let depth = nav.depth();
    let current = nav.current();

    if depth <= 1 {
        let cols = Layout::horizontal([
                Constraint::Percentage(MILLER_TWO_COL_PCT),
                Constraint::Percentage(MILLER_TWO_COL_PCT),
            ])
            .split(area);

        render_column(frame, cols[0], &current.node, current.selected, true, data, scroll_tick);
        render_preview_column(frame, cols[1], &current.node, current.selected, data);
    } else {
        let cols = Layout::horizontal([
                Constraint::Percentage(MILLER_PARENT_PCT),
                Constraint::Percentage(MILLER_CURRENT_PCT),
                Constraint::Percentage(MILLER_PREVIEW_PCT),
            ])
            .split(area);

        if let Some(parent) = nav.parent() {
            render_column(frame, cols[0], &parent.node, parent.selected, false, data, 0);
        }

        render_column(frame, cols[1], &current.node, current.selected, true, data, scroll_tick);
        render_preview_column(frame, cols[2], &current.node, current.selected, data);
    }
}

/// 渲染单个列（导航项列表或歌曲列表）
fn render_column(
    frame: &mut Frame,
    area: Rect,
    parent_node: &NavNode,
    selected: usize,
    is_active: bool,
    data: &ColumnData,
    scroll_tick: u16,
) {
    if parent_node.has_static_children() {
        let children = parent_node.children();
        if children.is_empty() {
            return;
        }

        let items: Vec<ListItem> = children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                ListItem::new(format!(" {}", child.display_name()))
                    .style(Theme::list_item_style(i == selected, is_active))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if is_active {
            state.select(Some(selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    } else if *parent_node == NavNode::Categories {
        // 渲染标签列表
        if data.tag_cache.is_empty() {
            if data.loading.contains(parent_node) {
                super::util::render_placeholder(frame, area, true, "");
            }
            return;
        }

        let items: Vec<ListItem> = data.tag_cache
            .iter()
            .enumerate()
            .map(|(i, tag)| {
                ListItem::new(format!(" {}", tag))
                    .style(Theme::list_item_style(i == selected, is_active))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if is_active {
            state.select(Some(selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    } else if *parent_node == NavNode::MyPlaylists {
        // 渲染歌单列表
        if data.playlist_cache.is_empty() {
            super::util::render_placeholder(frame, area, data.loading.contains(parent_node), t!("miller.no_playlists"));
            return;
        }

        let items: Vec<ListItem> = data.playlist_cache
            .iter()
            .enumerate()
            .map(|(i, pl)| {
                ListItem::new(format!(" {}", pl.name))
                    .style(Theme::list_item_style(i == selected, is_active))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if is_active {
            state.select(Some(selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    } else if *parent_node == NavNode::Queue {
        // 渲染播放队列
        if data.queue.songs.is_empty() {
            let hint = Paragraph::new(Span::styled(format!("  {}", t!("queue.empty")), Theme::secondary()));
            frame.render_widget(hint, area);
            return;
        }

        let now_playing = data.queue.current_index;
        let items: Vec<ListItem> = data.queue
            .songs
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_sel = i == selected && is_active;
                let tick = if is_sel { scroll_tick } else { 0 };
                let prefix = if Some(i) == now_playing { "\u{25b6} " } else { "  " };
                let title = format!("{}{}", prefix, item.name);
                let line = song_list_line(&title, &item.artist, area.width, is_sel, tick);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if is_active {
            state.select(Some(selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    } else if *parent_node == NavNode::SearchResults {
        // 搜索结果按 search_type 渲染不同列表
        match data.search_type {
            SearchType::Song => {
                if let Some(songs) = data.song_cache.get(&NavNode::SearchResults) {
                    if songs.is_empty() {
                        super::util::render_placeholder(frame, area, false, t!("search.no_results"));
                        return;
                    }
                    let items: Vec<ListItem> = songs.iter().enumerate().map(|(i, song)| {
                        let is_sel = i == selected && is_active;
                        let tick = if is_sel { scroll_tick } else { 0 };
                        ListItem::new(song_list_line(&song.title, &song.uploader_name, area.width, is_sel, tick))
                    }).collect();
                    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
                    let mut state = ListState::default();
                    if is_active { state.select(Some(selected)); }
                    frame.render_stateful_widget(list, area, &mut state);
                } else if data.loading.contains(&NavNode::SearchResults) {
                    super::util::render_placeholder(frame, area, true, "");
                }
            }
            SearchType::User => {
                if data.search_users.is_empty() {
                    super::util::render_placeholder(frame, area, data.loading.contains(&NavNode::SearchResults), t!("search.no_results"));
                    return;
                }
                let items: Vec<ListItem> = data.search_users.iter().enumerate().map(|(i, user)| {
                    ListItem::new(format!(" {}", user.username))
                        .style(Theme::list_item_style(i == selected, is_active))
                }).collect();
                let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
                let mut state = ListState::default();
                if is_active { state.select(Some(selected)); }
                frame.render_stateful_widget(list, area, &mut state);
            }
            SearchType::Playlist => {
                if data.search_playlists.is_empty() {
                    super::util::render_placeholder(frame, area, data.loading.contains(&NavNode::SearchResults), t!("search.no_results"));
                    return;
                }
                let items: Vec<ListItem> = data.search_playlists.iter().enumerate().map(|(i, pl)| {
                    ListItem::new(format!(" {}", pl.name))
                        .style(Theme::list_item_style(i == selected, is_active))
                }).collect();
                let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
                let mut state = ListState::default();
                if is_active { state.select(Some(selected)); }
                frame.render_stateful_widget(list, area, &mut state);
            }
        }
    } else if let Some(songs) = data.song_cache.get(parent_node) {
        if songs.is_empty() {
            super::util::render_placeholder(frame, area, false, t!("miller.no_songs"));
            return;
        }

        let items: Vec<ListItem> = songs
            .iter()
            .enumerate()
            .map(|(i, song)| {
                let is_sel = i == selected && is_active;
                let tick = if is_sel { scroll_tick } else { 0 };
                ListItem::new(song_list_line(
                    &song.title,
                    &song.uploader_name,
                    area.width,
                    is_sel,
                    tick,
                ))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        if is_active {
            state.select(Some(selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    } else if data.loading.contains(parent_node) {
        super::util::render_placeholder(frame, area, true, "");
    }
}
