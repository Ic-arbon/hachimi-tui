use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use ratatui_image::{StatefulImage, protocol::StatefulProtocol};

use super::player_bar::PlayerBarState;
use super::theme::Theme;

/// 渲染展开播放器视图
pub fn render(
    frame: &mut Frame,
    area: Rect,
    player_bar: &PlayerBarState,
    image_cache: &mut HashMap<String, StatefulProtocol>,
) {
    if !player_bar.has_song() {
        let hint = Paragraph::new(Span::styled(
            format!("  {}", t!("player.no_song")),
            Theme::secondary(),
        ));
        frame.render_widget(hint, area);
        return;
    }

    let has_cover = !player_bar.cover_url.is_empty()
        && image_cache.contains_key(&player_bar.cover_url);

    let (cover_area, info_area) = if has_cover {
        let cover_width = area.width.min(area.height * 2).min(40);
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(cover_width), Constraint::Min(1)])
            .split(area);
        (Some(cols[0]), cols[1])
    } else {
        (None, area)
    };

    // 渲染封面图
    if let Some(img_rect) = cover_area {
        if let Some(protocol) = image_cache.get_mut(&player_bar.cover_url) {
            let image = StatefulImage::new(None);
            frame.render_stateful_widget(image, img_rect, protocol);
        }
    }

    // 渲染歌曲信息
    let pad = 2u16.min(info_area.width / 2);
    let inner = Rect {
        x: info_area.x + pad,
        y: info_area.y + 1,
        width: info_area.width.saturating_sub(pad * 2),
        height: info_area.height.saturating_sub(1),
    };

    let time_current = format_time(player_bar.current_secs);
    let time_total = format_time(player_bar.total_secs);

    let status_icon = if player_bar.is_loading {
        "◌"
    } else if player_bar.is_playing {
        "▶"
    } else {
        "⏸"
    };

    let lines = vec![
        Line::from(Span::styled(
            player_bar.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("by {}", player_bar.artist),
            Theme::secondary(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{status_icon} {time_current} / {time_total}"),
            Theme::active(),
        )),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn format_time(secs: u32) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
}
