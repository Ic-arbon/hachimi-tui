use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// 按显示宽度截断文本，末尾加 ".."
pub(crate) fn truncate_with_dots(text: &str, max_width: usize) -> String {
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
pub(crate) fn marquee_text(text: &str, max_width: usize, tick: u16) -> String {
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
