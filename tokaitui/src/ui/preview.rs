use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Wrap},
};

use super::miller::ColumnData;
use super::navigation::{NavNode, SearchType};
use super::theme::Theme;
use crate::model::song::PublicSongDetail;
use crate::model::playlist::PlaylistMetadata;
use crate::model::user::PublicUserProfile;

/// 渲染 Preview 栏
pub fn render_preview_column(
    frame: &mut Frame,
    area: Rect,
    parent_node: &NavNode,
    selected: usize,
    data: &ColumnData,
) {
    let covers = data.covers;
    let scale = data.settings.display.cover_scale;
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
                render_song_detail(frame, area, detail, covers, scale);
            } else {
                render_queue_item_detail(frame, area, item, data.queue.current_index == Some(selected), covers, scale);
            }
        }
    } else if *parent_node == NavNode::SearchResults {
        match data.search_type {
            SearchType::Song => {
                if let Some(song) = data.song_cache.get(&NavNode::SearchResults).and_then(|s| s.get(selected)) {
                    render_song_detail(frame, area, song, covers, scale);
                }
            }
            SearchType::User => {
                if let Some(user) = data.search_users.get(selected) {
                    render_user_preview(frame, area, user, covers, scale);
                }
            }
            SearchType::Playlist => {
                if let Some(pl) = data.search_playlists.get(selected) {
                    render_playlist_preview(frame, area, pl, covers, scale);
                }
            }
        }
    } else if let Some(songs) = data.song_cache.get(parent_node) {
        if let Some(song) = songs.get(selected) {
            render_song_detail(frame, area, song, covers, scale);
        }
    }
}

/// 渲染队列项目详情预览
fn render_queue_item_detail(
    frame: &mut Frame,
    area: Rect,
    item: &crate::model::queue::MusicQueueItem,
    is_playing: bool,
    covers: &HashMap<String, u32>,
    cover_scale: u8,
) {
    let inner = super::util::padded_rect(area, 2);
    let inner = apply_cover(frame, inner, &item.cover_url, covers, cover_scale);

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
    covers: &HashMap<String, u32>,
    cover_scale: u8,
) {
    let inner = super::util::padded_rect(area, 2);
    let inner = apply_cover(frame, inner, &song.cover_url, covers, cover_scale);

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

/// 渲染用户搜索结果预览
fn render_user_preview(
    frame: &mut Frame,
    area: Rect,
    user: &PublicUserProfile,
    covers: &HashMap<String, u32>,
    cover_scale: u8,
) {
    let inner = super::util::padded_rect(area, 2);
    let inner = if let Some(ref url) = user.avatar_url {
        apply_cover(frame, inner, url, covers, cover_scale)
    } else {
        inner
    };
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
    covers: &HashMap<String, u32>,
    cover_scale: u8,
) {
    let inner = super::util::padded_rect(area, 2);
    let inner = if let Some(ref url) = pl.cover_url {
        apply_cover(frame, inner, url, covers, cover_scale)
    } else {
        inner
    };
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

/// 若封面已加载，在 inner 顶部渲染封面并返回收缩后的文字区域；否则原样返回
pub fn apply_cover(
    frame: &mut Frame,
    inner: Rect,
    cover_url: &str,
    covers: &HashMap<String, u32>,
    cover_scale: u8,
) -> Rect {
    let base_h = (inner.height / 3).min(20);
    if base_h < 4 {
        return inner;
    }
    let cover_h = (base_h as u32 * cover_scale as u32 / 100).max(2) as u16;
    if let Some(&id) = covers.get(cover_url) {
        let cover_w = (cover_h * 2).min(inner.width);
        let cx = inner.x + (inner.width - cover_w) / 2;
        let cover_rect = Rect::new(cx, inner.y, cover_w, cover_h);
        frame.render_widget(
            super::cover_widget::CoverWidget { image_id: id },
            cover_rect,
        );
        Rect {
            y: inner.y + cover_h + 1,
            height: inner.height.saturating_sub(cover_h + 1),
            ..inner
        }
    } else {
        inner
    }
}
