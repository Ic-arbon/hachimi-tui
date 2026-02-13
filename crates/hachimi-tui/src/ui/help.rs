use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::theme::Theme;

fn help_sections() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    vec![
        (
            t!("help.section.global"),
            vec![
                ("q / Ctrl+C", t!("help.quit")),
                ("Space", t!("help.play_pause")),
                ("n / N", t!("help.next_prev")),
                ("+/= / -", t!("help.volume")),
                ("> / <", t!("help.seek")),
                ("s", t!("help.play_mode")),
                ("v", t!("help.player_view")),
                ("/", t!("help.search")),
                ("?", t!("help.help")),
                ("!", t!("help.logs")),
                ("L", t!("help.logout")),
            ],
        ),
        (
            t!("help.section.navigation"),
            vec![
                ("j / k", t!("help.down_up")),
                ("l / Enter", t!("help.drill_in")),
                ("h", t!("help.drill_out")),
                ("g / G", t!("help.top_bottom")),
                ("a", t!("help.add_queue")),
                ("p", t!("help.add_playlist")),
            ],
        ),
        (
            t!("help.section.search"),
            vec![
                ("Tab", t!("help.switch_type")),
                ("Ctrl+s", t!("help.switch_sort")),
                ("Esc", t!("help.exit_search")),
            ],
        ),
    ]
}

/// 渲染悬浮帮助面板（居中覆盖）
pub fn render(frame: &mut Frame, area: Rect) {
    let sections = help_sections();

    // 计算内容尺寸
    let content_width = 42u16;
    let content_height = count_lines(&sections) as u16 + 4; // +4 for border + padding

    let panel_w = content_width.min(area.width.saturating_sub(4));
    let panel_h = content_height.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(panel_w)) / 2;
    let y = area.y + (area.height.saturating_sub(panel_h)) / 2;
    let panel_area = Rect::new(x, y, panel_w, panel_h);

    // 清除面板背景（左右各多 1 列，避免双宽字符被截断导致边框消失）
    let clear_area = Rect::new(
        panel_area.x.saturating_sub(1),
        panel_area.y,
        (panel_area.width + 2).min(area.width - panel_area.x.saturating_sub(1) + area.x),
        panel_area.height,
    );
    frame.render_widget(Clear, clear_area);

    // 构建内容
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (section_name, bindings) in &sections {
        lines.push(Line::from(Span::styled(
            format!("  {section_name}"),
            Style::default().add_modifier(Modifier::BOLD),
        )));

        for (key, desc) in bindings {
            let key_display = format!("  {key:<14}");
            lines.push(Line::from(vec![
                Span::styled(key_display, Theme::active()),
                Span::raw(*desc),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        format!("     {}", t!("help.close")),
        Theme::secondary(),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {} ", t!("help.title")),
            Style::default().add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, panel_area);
}

fn count_lines(sections: &[(&str, Vec<(&str, &str)>)]) -> usize {
    let mut n = 1; // top padding
    for (_, bindings) in sections {
        n += 1; // section title
        n += bindings.len();
        n += 1; // gap
    }
    n += 1; // close hint
    n
}
