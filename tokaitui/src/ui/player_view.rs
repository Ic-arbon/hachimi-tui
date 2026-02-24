use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use super::lyrics::ParsedLyrics;
use super::theme::Theme;
use crate::model::song::PublicSongDetail;

/// 正在播放时传入的回放信息，用于时间同步歌词
pub struct PlaybackInfo<'a> {
    pub current_secs: u32,
    pub parsed_lyrics: &'a ParsedLyrics,
}

/// 渲染展开详情视图（选中歌曲 或 播放中歌曲）
pub fn render(
    frame: &mut Frame,
    area: Rect,
    detail: &PublicSongDetail,
    playback: Option<PlaybackInfo<'_>>,
    covers: &HashMap<String, u32>,
) {
    let padded = super::util::padded_rect(area, 2);

    // 左右对半分
    let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(padded);
    let left = cols[0];
    let inner = Rect {
        y: cols[1].y + 1,
        height: cols[1].height.saturating_sub(1),
        ..cols[1]
    };

    // 左栏：封面水平垂直居中，视觉正方形（终端格子高≈宽的2倍，故 w=2h）
    // 限制边：h = min(left.width/2, left.height) * 3/4，w = h*2
    if let Some(&id) = covers.get(&detail.cover_url) {
        let max_h = (left.width / 2).min(left.height) * 3 / 4;
        if max_h >= 2 {
            let cover_h = max_h;
            let cover_w = cover_h * 2;
            let cx = left.x + left.width.saturating_sub(cover_w) / 2;
            let cy = left.y + left.height.saturating_sub(cover_h) / 2;
            let cover_rect = Rect::new(cx, cy, cover_w, cover_h);
            frame.render_widget(super::cover_widget::CoverWidget { image_id: id }, cover_rect);
        }
    }

    let header_lines = vec![
        Line::from(Span::styled(
            detail.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("by {}", detail.uploader_name),
            Theme::secondary(),
        )),
    ];

    let header_height = header_lines.len() as u16;

    if let Some(pb) = playback {
        // 播放中：展示时间同步歌词
        render_playing(frame, inner, header_lines, header_height, pb);
    } else {
        // 浏览：展示歌曲元数据 + 歌词
        render_browsing(frame, inner, header_lines, detail);
    }
}

/// 播放中歌曲的右侧内容：标题 + 时间同步歌词
fn render_playing(
    frame: &mut Frame,
    inner: Rect,
    header_lines: Vec<Line<'static>>,
    header_height: u16,
    pb: PlaybackInfo<'_>,
) {
    match pb.parsed_lyrics {
        ParsedLyrics::Synced(lrc_lines) => {
            let header_para = Paragraph::new(header_lines);
            let header_rect = Rect { height: header_height.min(inner.height), ..inner };
            frame.render_widget(header_para, header_rect);

            let lyrics_y = inner.y + header_height + 1;
            if lyrics_y < inner.y + inner.height {
                let lyrics_rect = Rect {
                    x: inner.x,
                    y: lyrics_y,
                    width: inner.width,
                    height: inner.height.saturating_sub(header_height + 1),
                };
                render_synced_lyrics(frame, lyrics_rect, lrc_lines, pb.current_secs);
            }
        }
        ParsedLyrics::Plain(plain_lines) => {
            let mut lines = header_lines;
            if !plain_lines.is_empty() {
                lines.push(Line::from(""));
                for line in plain_lines {
                    lines.push(Line::from(Span::styled(
                        line.clone(),
                        Theme::secondary(),
                    )));
                }
            }
            let para = Paragraph::new(lines).wrap(Wrap { trim: false });
            frame.render_widget(para, inner);
        }
        ParsedLyrics::Empty => {
            let mut lines = header_lines;
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                t!("player.no_lyrics"),
                Theme::secondary(),
            )));
            let para = Paragraph::new(lines).wrap(Wrap { trim: false });
            frame.render_widget(para, inner);
        }
    }
}

/// 浏览歌曲的右侧内容：标题 + 元数据 + 歌词
fn render_browsing(
    frame: &mut Frame,
    inner: Rect,
    header_lines: Vec<Line<'static>>,
    detail: &PublicSongDetail,
) {
    let mut lines = header_lines;

    // 副标题
    if !detail.subtitle.is_empty() {
        lines.push(Line::from(Span::styled(
            detail.subtitle.clone(),
            Theme::secondary(),
        )));
    }

    lines.push(Line::from(""));

    // 时长 · 播放数 · 喜欢数
    lines.push(Line::from(vec![
        Span::styled(format!("{}  ", detail.format_duration()), Theme::active()),
        Span::styled(format!("\u{25b6} {}  ", detail.play_count), Theme::secondary()),
        Span::styled(format!("\u{2665} {}", detail.like_count), Theme::secondary()),
    ]));

    // 标签
    if !detail.tags.is_empty() {
        let mut tag_spans: Vec<Span> = Vec::new();
        let mut prev_color: Option<Color> = None;
        for (i, tag) in detail.tags.iter().enumerate() {
            let style = Theme::tag_badge(i, prev_color);
            prev_color = style.bg;
            tag_spans.push(Span::styled(format!(" {} ", tag.name), style));
        }
        lines.push(Line::from(tag_spans));
    }

    // 原作信息
    if !detail.origin_infos.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("miller.origin").to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for info in &detail.origin_infos {
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
        let date_str = detail.release_time.format("%Y-%m-%d").to_string();
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}: ", t!("miller.release_date")),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(date_str, Theme::secondary()),
        ]));
    }

    // 创作团队
    if !detail.production_crew.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("miller.crew").to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for member in &detail.production_crew {
            let name = member.person_name.as_deref().unwrap_or("?");
            lines.push(Line::from(Span::styled(
                format!("  {} \u{2014} {name}", member.role),
                Theme::secondary(),
            )));
        }
    }

    // 外部链接
    if !detail.external_links.is_empty() {
        let mut link_spans: Vec<Span> = Vec::new();
        for (i, link) in detail.external_links.iter().enumerate() {
            if i > 0 {
                link_spans.push(Span::styled(" \u{00b7} ", Theme::secondary()));
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
    if !detail.description.is_empty() {
        lines.push(Line::from(""));
        for line in detail.description.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Theme::secondary(),
            )));
        }
    }

    // 歌词（解析为纯文本展示，不做时间同步）
    let parsed = crate::ui::lyrics::parse(&detail.lyrics);
    match &parsed {
        ParsedLyrics::Synced(lrc) if !lrc.is_empty() => {
            lines.push(Line::from(""));
            for l in lrc {
                lines.push(Line::from(Span::styled(
                    format!("  {}", l.text),
                    Theme::secondary(),
                )));
            }
        }
        ParsedLyrics::Plain(plain) if !plain.is_empty() => {
            lines.push(Line::from(""));
            for l in plain {
                lines.push(Line::from(Span::styled(l.clone(), Theme::secondary())));
            }
        }
        _ => {}
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

/// 渲染时间同步歌词：当前行高亮居中，上下文淡色
fn render_synced_lyrics(
    frame: &mut Frame,
    area: Rect,
    lrc_lines: &[super::lyrics::LrcLine],
    current_secs: u32,
) {
    let visible_rows = area.height as usize;
    if visible_rows == 0 || lrc_lines.is_empty() {
        return;
    }

    // 当前行索引
    let cur_idx = {
        let idx = lrc_lines.partition_point(|l| l.time_secs <= current_secs);
        if idx == 0 { 0 } else { idx - 1 }
    };

    // 每行歌词占 2 行（歌词 + 空行），计算可显示的歌词条数
    let visible_items = (visible_rows + 1) / 2; // 最后一条不需要尾部空行

    // 计算窗口起始位置，让当前行尽量居中
    let half = visible_items / 2;
    let start = if cur_idx <= half {
        0
    } else if cur_idx + visible_items - half > lrc_lines.len() {
        lrc_lines.len().saturating_sub(visible_items)
    } else {
        cur_idx - half
    };
    let end = (start + visible_items).min(lrc_lines.len());

    let mut lines: Vec<Line> = Vec::new();
    for i in start..end {
        if i == cur_idx {
            lines.push(Line::from(Span::styled(
                format!("\u{25b6} {}", lrc_lines[i].text),
                Theme::highlight(),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {}", lrc_lines[i].text),
                Theme::secondary(),
            )));
        }
        // 每行歌词后加空行（最后一行除外）
        if i + 1 < end {
            lines.push(Line::from(""));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
