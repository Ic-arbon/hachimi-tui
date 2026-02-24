use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;

/// 左右各留 `h_pad` 列边距
pub fn padded_rect(area: Rect, h_pad: u16) -> Rect {
    let pad = h_pad.min(area.width / 2);
    Rect {
        x: area.x + pad,
        width: area.width.saturating_sub(pad * 2),
        ..area
    }
}

/// 渲染加载中或空列表提示
pub fn render_placeholder(frame: &mut Frame, area: Rect, is_loading: bool, empty_text: &str) {
    let (text, style) = if is_loading {
        (t!("miller.loading"), Theme::active())
    } else {
        (empty_text, Theme::secondary())
    };
    frame.render_widget(Paragraph::new(Span::styled(format!("  {text}"), style)), area);
}

/// 渲染居中浮层面板骨架（清除背景 + 边框 + 标题），
/// 返回 `(content_area, hint_area)`：content 可滚动，hint 钉在底部不受滚动影响。
pub fn overlay_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    max_w: u16,
    max_h: u16,
) -> (Rect, Rect) {
    let panel_w = max_w.min(area.width.saturating_sub(4));
    let panel_h = max_h.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(panel_w)) / 2;
    let y = area.y + (area.height.saturating_sub(panel_h)) / 2;
    let panel_area = Rect::new(x, y, panel_w, panel_h);

    // 左右各多清 1 列，避免双宽字符被截断导致边框消失
    let clear_area = Rect::new(
        panel_area.x.saturating_sub(1),
        panel_area.y,
        (panel_area.width + 2).min(area.width - panel_area.x.saturating_sub(1) + area.x),
        panel_area.height,
    );
    frame.render_widget(Clear, clear_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(panel_area);
    frame.render_widget(block, panel_area);

    // 底部 1 行留给固定提示，其余给可滚动内容
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    (chunks[0], chunks[1])
}

