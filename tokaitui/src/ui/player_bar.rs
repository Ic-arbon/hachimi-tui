use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use super::theme::Theme;

pub struct PlayerBarState {
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub current_secs: u32,
    pub total_secs: u32,
    pub is_loading: bool,
    pub cover_url: String,
}

impl Default for PlayerBarState {
    fn default() -> Self {
        Self {
            is_playing: false,
            title: String::new(),
            artist: String::new(),
            current_secs: 0,
            total_secs: 0,
            is_loading: false,
            cover_url: String::new(),
        }
    }
}

impl PlayerBarState {
    pub fn has_song(&self) -> bool {
        !self.title.is_empty()
    }
}

/// 渲染底部播放状态栏
pub fn render(frame: &mut Frame, area: Rect, state: &PlayerBarState) {
    if !state.has_song() {
        let empty = Paragraph::new(format!("  {}", t!("player.no_song")))
            .style(Theme::secondary());
        frame.render_widget(empty, area);
        return;
    }

    let status_icon = if state.is_loading {
        "◌"
    } else if state.is_playing {
        "▶"
    } else {
        "⏸"
    };

    let time_current = format_time(state.current_secs);
    let time_total = format_time(state.total_secs);

    let progress_bar = build_progress_bar(state.current_secs, state.total_secs, 10);

    let song_info = format!("{} - {}", state.title, state.artist);
    let right_part = format!(
        " {}/{} {} ",
        time_current, time_total, progress_bar
    );

    let available_width = area.width as usize;
    let right_len = right_part.width();
    let icon_part = format!("  {} ", status_icon);
    let icon_len = icon_part.width();

    let song_max = available_width.saturating_sub(right_len + icon_len);
    let song_display = truncate_str(&song_info, song_max);
    let padding = available_width.saturating_sub(icon_len + song_display.width() + right_len);

    let line = Line::from(vec![
        Span::styled(icon_part, Theme::active()),
        Span::styled(song_display, Style::default()),
        Span::raw(" ".repeat(padding)),
        Span::styled(right_part, Theme::secondary()),
    ]);

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

fn format_time(secs: u32) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
}

fn build_progress_bar(current: u32, total: u32, width: usize) -> String {
    if total == 0 {
        return "⣀".repeat(width);
    }
    let ratio = current as f64 / total as f64;
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "⣿".repeat(filled), "⣀".repeat(empty))
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.width() <= max {
        return s.to_string();
    }
    let content_max = max.saturating_sub(2);
    let mut result = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw > content_max {
            break;
        }
        result.push(c);
        w += cw;
    }
    result.push_str("..");
    result
}
