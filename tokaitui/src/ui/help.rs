use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
                ("i", t!("help.player_view")),
                // ("/", t!("help.search")),  // TODO: 搜索功能尚未实现
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
                ("d", t!("help.remove_queue")),
                ("o", t!("help.open_link")),
                // ("p", t!("help.add_playlist")),  // TODO: 歌单功能尚未实现
            ],
        ),
        (
            t!("help.section.danmaku"),
            vec![
                ("D", t!("help.fetch_danmaku")),
            ],
        ),
        // TODO: 搜索功能尚未实现
        // (
        //     t!("help.section.search"),
        //     vec![
        //         ("Tab", t!("help.switch_type")),
        //         ("Ctrl+s", t!("help.switch_sort")),
        //         ("Esc", t!("help.exit_search")),
        //     ],
        // ),
    ]
}

/// 渲染悬浮帮助面板（居中覆盖）
pub fn render(frame: &mut Frame, area: Rect, scroll: u16) {
    let sections = help_sections();

    // 面板外高度 = 2 (borders) + content_lines + 1 (hint)
    let panel_h = count_lines(&sections) as u16 + 3;

    let (content_area, hint_area) = super::util::overlay_panel(
        frame, area, t!("help.title"),
        super::constants::HELP_PANEL_WIDTH, panel_h,
    );

    // 可滚动内容
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

    let para = Paragraph::new(lines).scroll((scroll, 0));
    frame.render_widget(para, content_area);

    // 固定提示（不受滚动影响）
    let hint = Paragraph::new(Span::styled(
        format!("     {}", t!("help.close")),
        Theme::secondary(),
    ));
    frame.render_widget(hint, hint_area);
}

fn count_lines(sections: &[(&str, Vec<(&str, &str)>)]) -> usize {
    let mut n = 1; // top padding
    for (_, bindings) in sections {
        n += 1; // section title
        n += bindings.len();
        n += 1; // gap
    }
    n
}
