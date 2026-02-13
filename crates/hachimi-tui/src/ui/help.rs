use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::theme::Theme;

/// 按键帮助面板定义
const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Global",
        &[
            ("q / Ctrl+C", "Quit"),
            ("Space", "Play / Pause"),
            ("n / N", "Next / Prev track"),
            ("+/= / -", "Volume up / down"),
            ("> / <", "Seek ±5s"),
            ("s", "Cycle play mode"),
            ("v", "Toggle player view"),
            ("/", "Search"),
            ("?", "This help"),
            ("L", "Logout"),
        ],
    ),
    (
        "Navigation",
        &[
            ("j / k", "Down / Up"),
            ("l / Enter", "Drill in"),
            ("h", "Drill out"),
            ("g / G", "Top / Bottom"),
            ("a", "Add to queue"),
            ("p", "Add to playlist"),
        ],
    ),
    (
        "Search",
        &[
            ("Tab", "Switch type"),
            ("Ctrl+s", "Switch sort"),
            ("Esc", "Exit search"),
        ],
    ),
];

/// 渲染悬浮帮助面板（居中覆盖）
pub fn render(frame: &mut Frame, area: Rect) {
    // 计算内容尺寸
    let content_width = 42u16;
    let content_height = count_lines() as u16 + 4; // +4 for border + padding

    let panel_w = content_width.min(area.width.saturating_sub(4));
    let panel_h = content_height.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(panel_w)) / 2;
    let y = area.y + (area.height.saturating_sub(panel_h)) / 2;
    let panel_area = Rect::new(x, y, panel_w, panel_h);

    // 清除面板背景
    frame.render_widget(Clear, panel_area);

    // 构建内容
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (section_name, bindings) in HELP_SECTIONS {
        lines.push(Line::from(Span::styled(
            format!("  {section_name}"),
            Style::default().add_modifier(Modifier::BOLD),
        )));

        for (key, desc) in *bindings {
            let key_display = format!("  {key:<14}");
            lines.push(Line::from(vec![
                Span::styled(key_display, Theme::active()),
                Span::raw(*desc),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "       Press ? or Esc to close",
        Theme::secondary(),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Key Bindings ",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, panel_area);
}

fn count_lines() -> usize {
    let mut n = 1; // top padding
    for (_, bindings) in HELP_SECTIONS {
        n += 1; // section title
        n += bindings.len();
        n += 1; // gap
    }
    n += 1; // close hint
    n
}
