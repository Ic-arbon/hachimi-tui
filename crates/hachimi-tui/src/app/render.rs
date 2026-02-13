use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::ui::navigation::NavNode;

use super::{App, InputMode};

impl App {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);

        match self.input_mode {
            InputMode::Login => {
                crate::ui::login::render(frame, chunks[1], &self.login);
            }
            _ => {
                if self.player.expanded {
                    self.render_player_view(frame, chunks[1]);
                } else if self.nav.current().node == NavNode::Settings {
                    self.render_settings(frame, chunks[1]);
                } else {
                    self.render_miller(frame, chunks[1]);
                }
            }
        }

        self.render_player_bar(frame, chunks[2]);

        if self.show_logs {
            crate::ui::log_view::render(frame, frame.area(), &self.logs);
        }

        if self.show_help {
            crate::ui::help::render(frame, frame.area());
        }
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::Alignment;
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;
        use unicode_width::UnicodeWidthStr;

        let status = if let Some(name) = &self.username {
            Span::styled(
                format!("  {name}"),
                crate::ui::theme::Theme::secondary(),
            )
        } else if self.client.is_authenticated_sync() {
            Span::styled(
                format!("  {}", t!("app.logged_in")),
                crate::ui::theme::Theme::secondary(),
            )
        } else {
            Span::styled(
                format!("  {}", t!("app.anonymous")),
                crate::ui::theme::Theme::secondary(),
            )
        };

        let title_span = Span::styled("  HACHIMI", crate::ui::theme::Theme::title());

        // 右侧色块段
        let mode_str = match self.settings.player.default_play_mode {
            crate::config::settings::PlayMode::Sequential => " [>] ",
            crate::config::settings::PlayMode::Shuffle => " [x] ",
            crate::config::settings::PlayMode::RepeatOne => " [1] ",
        };
        let vol_str = if self.player.is_muted {
            " vol -- ".to_string()
        } else {
            format!(" vol {}% ", self.player.volume)
        };
        let now = chrono::Local::now();
        let time_str = now.format(" %H:%M ").to_string();

        let block_bg = Style::default().fg(Color::Black).bg(Color::DarkGray);
        let block_accent = Style::default().fg(Color::Black).bg(Color::Cyan);

        let mut right_spans: Vec<Span> = Vec::new();

        if self.logs.unread_count > 0 {
            right_spans.push(Span::styled(
                format!(" ! {} ", self.logs.unread_count),
                Style::default().fg(Color::White).bg(Color::Red),
            ));
        }
        right_spans.push(Span::styled(mode_str, block_bg));
        right_spans.push(Span::styled(vol_str, block_accent));
        right_spans.push(Span::styled(time_str.clone(), block_bg));

        let right_width: u16 = right_spans
            .iter()
            .map(|s| s.content.width() as u16)
            .sum();

        // 左侧
        let left = Line::from(vec![title_span, status]);
        let left_p = Paragraph::new(left);

        let right_p = Paragraph::new(Line::from(right_spans))
            .alignment(Alignment::Right);

        use ratatui::layout::{Constraint as C, Direction as D, Layout as L};
        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([C::Min(1), C::Length(right_width)])
            .split(area);

        frame.render_widget(left_p, cols[0]);
        frame.render_widget(right_p, cols[1]);
    }

    fn render_miller(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let font_size = self.cache.picker.as_ref()
            .map(|p| p.font_size()).unwrap_or((8, 16));
        crate::ui::miller::render(
            frame,
            area,
            &self.nav,
            &self.cache.songs,
            &self.cache.tags,
            &self.cache.loading,
            self.scroll_tick,
            &self.settings,
            &mut self.cache.images,
            font_size,
            &mut self.cache.last_image_rect,
        );
    }

    fn render_player_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::player_bar::render(frame, area, &self.player.bar);
    }

    fn render_settings(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint as C, Direction as D, Layout as L};
        use ratatui::style::{Modifier, Style};
        use ratatui::widgets::{List, ListItem};

        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([
                C::Percentage(15),
                C::Percentage(45),
                C::Percentage(40),
            ])
            .split(area);

        // Left: Root's children as parent column
        if let Some(parent) = self.nav.parent() {
            let children = parent.node.children();
            let items: Vec<ListItem> = children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let style = if i == parent.selected {
                        crate::ui::theme::Theme::secondary().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" {}", child.display_name())).style(style)
                })
                .collect();
            let list = List::new(items);
            frame.render_widget(list, cols[0]);
        }

        // Center: settings items
        let selected = self.nav.current().selected;
        crate::ui::settings_view::render_list(frame, cols[1], &self.settings, selected);

        // Right: hint
        crate::ui::settings_view::render_hint(frame, cols[2]);
    }

    fn render_player_view(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let font_size = self.cache.picker.as_ref()
            .map(|p| p.font_size()).unwrap_or((8, 16));
        crate::ui::player_view::render(
            frame,
            area,
            &self.player.bar,
            &mut self.cache.images,
            self.player.current_detail.as_ref(),
            font_size,
            &mut self.cache.last_image_rect,
        );
    }
}
