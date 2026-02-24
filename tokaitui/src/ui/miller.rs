use std::collections::{HashMap, HashSet};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

use super::navigation::{NavNode, NavStack, SearchType};
use super::theme::Theme;
use crate::config::settings::Settings;
use crate::model::playlist::{PlaylistItem, PlaylistMetadata};
use crate::model::queue::QueueState;
use crate::model::song::PublicSongDetail;
use crate::model::user::PublicUserProfile;

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
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_column(frame, cols[0], &current.node, current.selected, true, data, scroll_tick);
        render_preview_column(frame, cols[1], &current.node, current.selected, data);
    } else {
        let cols = Layout::horizontal([
                Constraint::Percentage(15),
                Constraint::Percentage(45),
                Constraint::Percentage(40),
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

/// 渲染 Preview 栏
fn render_preview_column(
    frame: &mut Frame,
    area: Rect,
    parent_node: &NavNode,
    selected: usize,
    data: &ColumnData,
) {
    if parent_node.has_static_children() {
        let children = parent_node.children();
        if children.is_empty() {
            return;
        }

        let Some(selected_node) = children.get(selected) else {
            return;
        };

        if selected_node.has_static_children() {
            let sub_children = selected_node.children();
            let items: Vec<ListItem> = sub_children
                .iter()
                .map(|child| {
                    ListItem::new(format!(" {}", child.display_name()))
                        .style(Theme::secondary())
                })
                .collect();
            let list = List::new(items);
            frame.render_widget(list, area);
        } else if *selected_node == NavNode::Settings {
            super::settings_view::render_preview(frame, area, data.settings);
        } else if *selected_node == NavNode::Categories {
            if data.tag_cache.is_empty() {
                if data.loading.contains(selected_node) {
                    super::util::render_placeholder(frame, area, true, "");
                }
            } else {
                let items: Vec<ListItem> = data.tag_cache
                    .iter()
                    .map(|tag| {
                        ListItem::new(format!(" {}", tag)).style(Theme::secondary())
                    })
                    .collect();
                let list = List::new(items);
                frame.render_widget(list, area);
            }
        } else if *selected_node == NavNode::MyPlaylists {
            if data.playlist_cache.is_empty() {
                if data.loading.contains(selected_node) {
                    super::util::render_placeholder(frame, area, true, "");
                }
            } else {
                let items: Vec<ListItem> = data.playlist_cache
                    .iter()
                    .map(|pl| {
                        ListItem::new(format!(" {}", pl.name)).style(Theme::secondary())
                    })
                    .collect();
                let list = List::new(items);
                frame.render_widget(list, area);
            }
        } else if *selected_node == NavNode::Queue {
            if data.queue.songs.is_empty() {
                let hint = Paragraph::new(Span::styled(format!("  {}", t!("queue.empty")), Theme::secondary()));
                frame.render_widget(hint, area);
            } else {
                let now_playing = data.queue.current_index;
                let items: Vec<ListItem> = data.queue.songs.iter().enumerate().map(|(i, item)| {
                    let prefix = if Some(i) == now_playing { "\u{25b6} " } else { "  " };
                    ListItem::new(format!("{}{}", prefix, item.name)).style(Theme::secondary())
                }).collect();
                let list = List::new(items);
                frame.render_widget(list, area);
            }
        } else if let Some(songs) = data.song_cache.get(selected_node) {
            render_song_list_preview(frame, area, songs);
        } else if data.loading.contains(selected_node) {
            super::util::render_placeholder(frame, area, true, "");
        } else {
            let hint = Paragraph::new(vec![Line::from(Span::styled(
                format!("  {}", selected_node.display_name()),
                Theme::title(),
            ))]);
            frame.render_widget(hint, area);
        }
    } else if *parent_node == NavNode::Categories {
        if let Some(tag_name) = data.tag_cache.get(selected) {
            let tag_node = NavNode::Tag { name: tag_name.clone() };
            if let Some(songs) = data.song_cache.get(&tag_node) {
                render_song_list_preview(frame, area, songs);
            } else if data.loading.contains(&tag_node) {
                super::util::render_placeholder(frame, area, true, "");
            }
        }
    } else if *parent_node == NavNode::MyPlaylists {
        if let Some(pl) = data.playlist_cache.get(selected) {
            let pl_node = NavNode::PlaylistDetail { id: pl.id };
            if let Some(songs) = data.song_cache.get(&pl_node) {
                render_song_list_preview(frame, area, songs);
            } else if data.loading.contains(&pl_node) {
                super::util::render_placeholder(frame, area, true, "");
            }
        }
    } else if *parent_node == NavNode::Queue {
        if let Some(item) = data.queue.songs.get(selected) {
            if let Some(detail) = data.queue_detail.get(&item.id) {
                render_song_detail(frame, area, detail);
            } else {
                render_queue_item_detail(frame, area, item, data.queue.current_index == Some(selected));
            }
        }
    } else if *parent_node == NavNode::SearchResults {
        match data.search_type {
            SearchType::Song => {
                if let Some(song) = data.song_cache.get(&NavNode::SearchResults).and_then(|s| s.get(selected)) {
                    render_song_detail(frame, area, song);
                }
            }
            SearchType::User => {
                if let Some(user) = data.search_users.get(selected) {
                    render_user_preview(frame, area, user);
                }
            }
            SearchType::Playlist => {
                if let Some(pl) = data.search_playlists.get(selected) {
                    render_playlist_preview(frame, area, pl);
                }
            }
        }
    } else if let Some(songs) = data.song_cache.get(parent_node) {
        if let Some(song) = songs.get(selected) {
            render_song_detail(frame, area, song);
        }
    }
}

/// 渲染队列项目详情预览
fn render_queue_item_detail(
    frame: &mut Frame,
    area: Rect,
    item: &crate::model::queue::MusicQueueItem,
    is_playing: bool,
) {
    let inner = super::util::padded_rect(area, 2);

    let mut lines = Vec::new();

    if is_playing {
        lines.push(Line::from(Span::styled(
            "\u{25b6} Now Playing",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(Span::styled(
        item.name.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("by {}", item.artist),
        Theme::secondary(),
    )));
    lines.push(Line::from(""));

    let mins = item.duration_secs / 60;
    let secs = item.duration_secs % 60;
    lines.push(Line::from(Span::styled(
        format!("{}:{:02}", mins, secs),
        Theme::active(),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t!("queue.hint").to_string(),
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

/// 渲染歌曲列表预览（Preview 栏中显示标题列表）
fn render_song_list_preview(frame: &mut Frame, area: Rect, songs: &[PublicSongDetail]) {
    if songs.is_empty() {
        let hint = Paragraph::new(Span::styled(format!("  {}", t!("miller.no_songs")), Theme::secondary()));
        frame.render_widget(hint, area);
        return;
    }
    let items: Vec<ListItem> = songs
        .iter()
        .map(|song| ListItem::new(format!(" {}", song.title)).style(Theme::secondary()))
        .collect();
    let list = List::new(items);
    frame.render_widget(list, area);
}

/// 渲染歌曲详情预览
fn render_song_detail(
    frame: &mut Frame,
    area: Rect,
    song: &PublicSongDetail,
) {
    let inner = super::util::padded_rect(area, 2);

    let mut lines = vec![
        Line::from(Span::styled(
            song.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ];

    // 副标题
    if !song.subtitle.is_empty() {
        lines.push(Line::from(Span::styled(
            song.subtitle.clone(),
            Theme::secondary(),
        )));
    }

    lines.push(Line::from(Span::styled(
        format!("by {}", song.uploader_name),
        Theme::secondary(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{}  ", song.format_duration()), Theme::active()),
        Span::styled(format!("▶ {}  ", song.play_count), Theme::secondary()),
        Span::styled(format!("♥ {}", song.like_count), Theme::secondary()),
    ]));

    // 标签（彩色色块）
    if !song.tags.is_empty() {
        let mut tag_spans: Vec<Span> = Vec::new();
        let mut prev_color: Option<Color> = None;
        for (i, tag) in song.tags.iter().enumerate() {
            let style = Theme::tag_badge(i, prev_color);
            prev_color = style.bg;
            tag_spans.push(Span::styled(
                format!(" {} ", tag.name),
                style,
            ));
        }
        lines.push(Line::from(tag_spans));
    }

    // 原作信息
    if !song.origin_infos.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("miller.origin").to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for info in &song.origin_infos {
            let title = info.title.as_deref().unwrap_or("?");
            let artist = info.artist.as_deref().unwrap_or("");
            let text = if artist.is_empty() {
                format!("  {title}")
            } else {
                format!("  {title} - {artist}")
            };
            lines.push(Line::from(Span::styled(text, Theme::secondary())));
        }
    }

    // 发行日期
    {
        let date_str = song.release_time.format("%Y-%m-%d").to_string();
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}: ", t!("miller.release_date")),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(date_str, Theme::secondary()),
        ]));
    }

    // 创作团队
    if !song.production_crew.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("miller.crew").to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for member in &song.production_crew {
            let name = member.person_name.as_deref().unwrap_or("?");
            lines.push(Line::from(Span::styled(
                format!("  {} — {name}", member.role),
                Theme::secondary(),
            )));
        }
    }

    // 外部链接（仅提示存在，按 o 打开）
    if !song.external_links.is_empty() {
        let mut link_spans: Vec<Span> = Vec::new();
        for (i, link) in song.external_links.iter().enumerate() {
            if i > 0 {
                link_spans.push(Span::styled(" · ", Theme::secondary()));
            }
            link_spans.push(Span::styled(
                format!(" {} ", link.platform),
                Theme::link_badge(),
            ));
        }
        link_spans.push(Span::styled(
            format!("  {}", t!("miller.links_hint")),
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::from(link_spans));
    }

    // 简介
    if !song.description.is_empty() {
        for line in song.description.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Theme::secondary(),
            )));
        }
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

/// 按显示宽度截断文本，末尾加 ".."
fn truncate_with_dots(text: &str, max_width: usize) -> String {
    let dots_width = 2; // ".." 占 2 列
    let content_width = max_width.saturating_sub(dots_width);
    let mut result = String::new();
    let mut width = 0;
    for c in text.chars() {
        let cw = c.width().unwrap_or(0);
        if width + cw > content_width {
            break;
        }
        result.push(c);
        width += cw;
    }
    result.push_str("..");
    result
}

/// Marquee 文字滚动：在固定宽度内循环显示超长文本
/// 开头和结尾各停顿 pause 个 tick，中间每 tick 滚动一个字符
/// 使用 unicode 显示宽度，正确处理 CJK 双宽字符
fn marquee_text(text: &str, max_width: usize, tick: u16) -> String {
    let text_width = text.width();
    if text_width <= max_width {
        return text.to_string();
    }

    // 按字符逐个累积显示宽度，建立字符边界 → 显示位置映射
    let char_widths: Vec<(char, usize)> = text
        .chars()
        .scan(0usize, |acc, c| {
            let w = *acc;
            *acc += c.width().unwrap_or(0);
            Some((c, w))
        })
        .collect();

    let max_scroll = text_width - max_width;
    let pause: u16 = 4;
    let cycle = pause + max_scroll as u16 + pause;
    let pos = tick % cycle;

    let offset = if pos < pause {
        0
    } else if pos < pause + max_scroll as u16 {
        (pos - pause) as usize
    } else {
        max_scroll
    };

    // 从 offset 显示位置开始，收集 max_width 显示宽度的字符
    let mut result = String::new();
    let mut width = 0;
    for &(c, w) in &char_widths {
        if w < offset {
            continue;
        }
        let cw = c.width().unwrap_or(0);
        if width + cw > max_width {
            break;
        }
        result.push(c);
        width += cw;
    }
    result
}

/// 渲染歌曲列表行（标题左对齐 + Artist 右对齐 DarkGray）
/// 选中项支持 marquee 滚动显示超长文字
pub fn song_list_line(
    title: &str,
    artist: &str,
    width: u16,
    is_selected: bool,
    scroll_tick: u16,
) -> Line<'static> {
    let available = width as usize;

    // Artist 保持完整显示，标题占剩余空间（使用显示宽度）
    let artist_display = format!(" {}", artist);
    let artist_width = artist_display.width();

    let title_max = available.saturating_sub(artist_width + 1);
    let title_full = format!(" {}", title);
    let title_width = title_full.width();
    let title_truncated = title_width > title_max;

    // 仅对歌曲名做截断和 marquee 滚动
    let title_display: String = if title_truncated {
        if is_selected {
            marquee_text(&title_full, title_max, scroll_tick)
        } else {
            truncate_with_dots(&title_full, title_max)
        }
    } else {
        title_full
    };

    let title_display_width = title_display.width();
    let artist_display_width = artist_width;
    let padding = available.saturating_sub(title_display_width + artist_display_width);
    let pad: String = " ".repeat(padding);

    let title_style = if is_selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let artist_style = if is_selected {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Line::from(vec![
        Span::styled(title_display, title_style),
        Span::raw(pad),
        Span::styled(artist_display, artist_style),
    ])
}

/// 渲染用户搜索结果预览
fn render_user_preview(
    frame: &mut Frame,
    area: Rect,
    user: &PublicUserProfile,
) {
    let inner = super::util::padded_rect(area, 2);
    let mut lines = vec![Line::from(Span::styled(user.username.clone(), Style::default().add_modifier(Modifier::BOLD)))];
    if let Some(bio) = &user.bio {
        if !bio.is_empty() {
            lines.push(Line::from(""));
            for l in bio.lines() { lines.push(Line::from(Span::styled(l.to_string(), Theme::secondary()))); }
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// 渲染歌单搜索结果预览
fn render_playlist_preview(
    frame: &mut Frame,
    area: Rect,
    pl: &PlaylistMetadata,
) {
    let inner = super::util::padded_rect(area, 2);
    let mut lines = vec![
        Line::from(Span::styled(pl.name.clone(), Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("by {}", pl.user_name), Theme::secondary())),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{} ", t!("search.songs_count")), Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", pl.songs_count), Theme::active()),
        ]),
    ];
    if let Some(desc) = &pl.description {
        if !desc.is_empty() {
            lines.push(Line::from(""));
            for l in desc.lines() { lines.push(Line::from(Span::styled(l.to_string(), Theme::secondary()))); }
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
