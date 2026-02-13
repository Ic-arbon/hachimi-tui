use std::collections::{HashMap, HashSet};

use unicode_width::UnicodeWidthStr;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

use super::navigation::{NavNode, NavStack};
use super::theme::Theme;
use crate::model::song::PublicSongDetail;

/// 渲染 Miller Columns 三栏布局
pub fn render(
    frame: &mut Frame,
    area: Rect,
    nav: &NavStack,
    song_cache: &HashMap<NavNode, Vec<PublicSongDetail>>,
    loading: &HashSet<NavNode>,
    scroll_tick: u16,
) {
    let depth = nav.depth();
    let current = nav.current();

    if depth <= 1 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_column(
            frame,
            cols[0],
            &current.node,
            current.selected,
            true,
            song_cache,
            loading,
            scroll_tick,
        );
        render_preview_column(
            frame,
            cols[1],
            &current.node,
            current.selected,
            song_cache,
            loading,
        );
    } else {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(45),
                Constraint::Percentage(40),
            ])
            .split(area);

        if let Some(parent) = nav.parent() {
            render_column(
                frame,
                cols[0],
                &parent.node,
                parent.selected,
                false,
                song_cache,
                loading,
                0, // 父列不滚动
            );
        }

        render_column(
            frame,
            cols[1],
            &current.node,
            current.selected,
            true,
            song_cache,
            loading,
            scroll_tick,
        );
        render_preview_column(
            frame,
            cols[2],
            &current.node,
            current.selected,
            song_cache,
            loading,
        );
    }
}

/// 渲染单个列（导航项列表或歌曲列表）
fn render_column(
    frame: &mut Frame,
    area: Rect,
    parent_node: &NavNode,
    selected: usize,
    is_active: bool,
    song_cache: &HashMap<NavNode, Vec<PublicSongDetail>>,
    loading: &HashSet<NavNode>,
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
                let style = if i == selected && is_active {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if i == selected {
                    Theme::secondary().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!(" {}", child.display_name())).style(style)
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
    } else if let Some(songs) = song_cache.get(parent_node) {
        if songs.is_empty() {
            let hint = Paragraph::new(Span::styled("  No songs", Theme::secondary()));
            frame.render_widget(hint, area);
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
    } else if loading.contains(parent_node) {
        let hint = Paragraph::new(Span::styled("  Loading...", Theme::active()));
        frame.render_widget(hint, area);
    }
}

/// 渲染 Preview 栏
fn render_preview_column(
    frame: &mut Frame,
    area: Rect,
    parent_node: &NavNode,
    selected: usize,
    song_cache: &HashMap<NavNode, Vec<PublicSongDetail>>,
    loading: &HashSet<NavNode>,
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
        } else if let Some(songs) = song_cache.get(selected_node) {
            render_song_list_preview(frame, area, songs);
        } else if loading.contains(selected_node) {
            let hint = Paragraph::new(Span::styled("  Loading...", Theme::active()));
            frame.render_widget(hint, area);
        } else {
            let hint = Paragraph::new(vec![Line::from(Span::styled(
                format!("  {}", selected_node.display_name()),
                Theme::title(),
            ))]);
            frame.render_widget(hint, area);
        }
    } else if let Some(songs) = song_cache.get(parent_node) {
        // 当前节点是歌曲列表，Preview 显示选中歌曲详情
        if let Some(song) = songs.get(selected) {
            render_song_detail(frame, area, song);
        }
    }
}

/// 渲染歌曲列表预览（Preview 栏中显示标题列表）
fn render_song_list_preview(frame: &mut Frame, area: Rect, songs: &[PublicSongDetail]) {
    if songs.is_empty() {
        let hint = Paragraph::new(Span::styled("  No songs", Theme::secondary()));
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
fn render_song_detail(frame: &mut Frame, area: Rect, song: &PublicSongDetail) {
    // 左右各留 2 列边距
    let pad = 2u16.min(area.width / 2);
    let inner = Rect {
        x: area.x + pad,
        width: area.width.saturating_sub(pad * 2),
        ..area
    };

    let mut lines = vec![
        Line::from(Span::styled(
            song.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("by {}", song.uploader_name),
            Theme::secondary(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{}  ", song.format_duration()), Theme::active()),
            Span::styled(
                format!("▶ {}  ", song.play_count),
                Theme::secondary(),
            ),
            Span::styled(format!("♥ {}", song.like_count), Theme::secondary()),
        ]),
    ];

    if !song.tags.is_empty() {
        let tags: String = song
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(tags, Theme::secondary())));
    }

    if !song.description.is_empty() {
        lines.push(Line::from(""));
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
        let cw = UnicodeWidthStr::width(c.to_string().as_str());
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
            *acc += UnicodeWidthStr::width(c.to_string().as_str());
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
        let cw = UnicodeWidthStr::width(c.to_string().as_str());
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
