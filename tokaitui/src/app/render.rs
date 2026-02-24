use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::navigation::NavNode;

use super::{App, InputMode};

impl App {
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);

        // 浮层打开时跳过底层内容渲染，避免 Kitty 图片协议残留
        let has_overlay = self.show_help || self.show_logs;

        match self.input_mode {
            InputMode::Login => {
                crate::ui::login::render(frame, chunks[1], &self.login);
            }
            _ if !has_overlay => {
                if self.player.expanded {
                    self.render_player_view(frame, chunks[1]);
                } else if self.nav.current().node == NavNode::Settings {
                    self.render_settings(frame, chunks[1]);
                } else if self.input_mode == InputMode::Search
                    || self.nav.contains(&NavNode::SearchResults)
                {
                    // 搜索模式或搜索结果导航中：顶部搜索栏 + 下方 miller
                    let search_chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)])
                        .split(chunks[1]);
                    self.render_search_bar(frame, search_chunks[0]);
                    self.render_miller(frame, search_chunks[1]);
                } else {
                    self.render_miller(frame, chunks[1]);
                }
            }
            _ => {}
        }

        if !has_overlay {
            self.render_player_bar(frame, chunks[2]);
        }

        if self.show_logs {
            crate::ui::log_view::render(frame, frame.area(), &self.logs);
        }

        if self.show_help {
            crate::ui::help::render(frame, frame.area(), self.help_scroll);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::Alignment;

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

        let cols = Layout::horizontal([Constraint::Min(1), Constraint::Length(right_width)])
            .split(area);

        frame.render_widget(left_p, cols[0]);
        frame.render_widget(right_p, cols[1]);
    }

    fn render_miller(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let data = crate::ui::miller::ColumnData {
            song_cache: &self.cache.songs,
            tag_cache: self.cache.tags.as_deref().unwrap_or_default(),
            playlist_cache: self.cache.playlists.as_deref().unwrap_or_default(),
            queue: &self.queue,
            queue_detail: &self.cache.queue_song_detail,
            loading: &self.cache.loading,
            settings: &self.settings,
            search_type: self.search.search_type,
            search_users: &self.cache.search_users,
            search_playlists: &self.cache.search_playlists,
            covers: self.cache.covers.id_map(),
        };
        crate::ui::miller::render(
            frame,
            area,
            &self.nav,
            &data,
            self.scroll_tick,
        );
    }

    fn render_search_bar(&self, frame: &mut Frame, area: Rect) {
        let type_label = self.search.search_type.label();
        let sort_label = self.search.sort.label();
        let query = &self.search.query;
        let cursor = self.search.cursor_pos;

        // 构建搜索栏: [类型▾] /query|  排序▾
        let mut spans = vec![
            Span::styled(
                format!(" [{}▾] ", type_label),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
        ];

        if self.input_mode == InputMode::Search {
            // 编辑模式：query 中光标位置用高亮显示
            let before: String = query.chars().take(cursor).collect();
            let cursor_char: String = query.chars().skip(cursor).take(1).collect();
            let after: String = query.chars().skip(cursor + 1).collect();

            spans.push(Span::raw(before));
            if cursor_char.is_empty() {
                spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
            } else {
                spans.push(Span::styled(cursor_char, Style::default().bg(Color::White).fg(Color::Black)));
            }
            spans.push(Span::raw(after));
        } else {
            // 非编辑模式：仅显示查询文本
            spans.push(Span::raw(query.clone()));
        }

        // 右侧排序标签
        let sort_str = format!("  {}▾ ", sort_label);
        let sort_width = sort_str.width() as u16;
        let left_width = area.width.saturating_sub(sort_width);

        let left_line = Line::from(spans);
        let left_p = Paragraph::new(left_line);

        let right_p = Paragraph::new(Line::from(Span::styled(
            sort_str,
            Style::default().fg(Color::DarkGray),
        )));

        let cols = Layout::horizontal([Constraint::Length(left_width), Constraint::Length(sort_width)])
            .split(area);

        frame.render_widget(left_p, cols[0]);
        frame.render_widget(right_p, cols[1]);
    }

    fn render_player_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::player_bar::render(frame, area, &self.player.bar);
    }

    /// draw 结束后，将本帧收集的封面放置请求写入 stdout
    /// 每帧先删除所有上帧 placement（用 d=i 保留 image data），再重新放置本帧封面。
    /// 这样可以正确处理：切换曲目、miller↔player_view 切换导致的位置变化、无封面时的清除。
    pub(crate) fn render_cover_placements(&mut self) -> anyhow::Result<()> {
        use std::io::Write;
        use crate::ui::cover_widget::PENDING_PLACEMENTS;

        let placements: Vec<(u32, ratatui::layout::Rect)> =
            PENDING_PLACEMENTS.with(|p| p.borrow_mut().drain(..).collect());

        let new_ids: Vec<u32> = placements.iter().map(|(id, _)| *id).collect();

        if self.active_cover_ids.is_empty() && placements.is_empty() && !self.needs_cover_reupload {
            return Ok(());
        }

        let mut out = std::io::stdout().lock();

        // 终端缩放后 image data 被清除，需先重新上传再放置
        if self.needs_cover_reupload {
            for seq in self.cache.covers.all_upload_seqs() {
                out.write_all(seq)?;
            }
            self.needs_cover_reupload = false;
        }

        if self.active_cover_ids.is_empty() && placements.is_empty() {
            out.flush()?;
            return Ok(());
        }

        out.write_all(b"\x1b7")?;

        // 删除所有上帧 placement（d=i 小写：保留 image data，避免 re-place 时数据不存在）
        for &id in &self.active_cover_ids {
            out.write_all(&crate::ui::kitty::delete_placement(id))?;
        }

        // 放置本帧所有封面
        for (id, rect) in &placements {
            write!(out, "\x1b[{};{}H", rect.y + 1, rect.x + 1)?;
            out.write_all(&crate::ui::kitty::place_at_cursor(*id, rect.width, rect.height))?;
        }

        out.write_all(b"\x1b8")?;
        out.flush()?;

        self.active_cover_ids = new_ids;
        Ok(())
    }

    fn render_settings(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::style::Modifier;
        use ratatui::widgets::{List, ListItem};

        let cols = Layout::horizontal([
                Constraint::Percentage(15),
                Constraint::Percentage(45),
                Constraint::Percentage(40),
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
        crate::ui::settings_view::render_hint(frame, cols[2], selected, &self.settings);
    }

    fn render_player_view(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // 优先展示导航中选中的歌曲，回退到播放中歌曲
        let node = self.nav.current().node.clone();
        let sel_idx = self.nav.current().selected;

        let browsed_detail: Option<crate::model::song::PublicSongDetail> = if node == NavNode::Queue {
            // 优先使用完整详情，回退到队列项基本信息
            self.queue.songs.get(sel_idx).map(|item| {
                self.cache.queue_song_detail.get(&item.id).cloned()
                    .unwrap_or_else(|| item.to_song_detail())
            })
        } else if node == NavNode::SearchResults {
            // 仅歌曲搜索才有歌曲详情
            match self.search.search_type {
                crate::ui::navigation::SearchType::Song => {
                    self.cache.songs.get(&node).and_then(|s| s.get(sel_idx)).cloned()
                }
                _ => None,
            }
        } else if !node.has_static_children() && node != NavNode::Settings {
            self.cache.songs.get(&node).and_then(|s| s.get(sel_idx)).cloned()
        } else {
            None
        };

        // 跟随播放时优先展示播放中歌曲，浏览模式优先展示导航选中歌曲
        let detail = if self.player.follow_playback {
            self.player.current_detail.clone().or(browsed_detail)
        } else {
            browsed_detail.or_else(|| self.player.current_detail.clone())
        };

        let Some(detail) = detail else {
            let hint = Paragraph::new(Span::styled(
                format!("  {}", t!("player.no_song")),
                crate::ui::theme::Theme::secondary(),
            ));
            frame.render_widget(hint, area);
            return;
        };

        let is_playing = self.player.current_detail.as_ref()
            .map_or(false, |p| p.id == detail.id);

        let playback = if is_playing {
            Some(crate::ui::player_view::PlaybackInfo {
                current_secs: self.player.bar.current_secs,
                parsed_lyrics: &self.player.parsed_lyrics,
            })
        } else {
            None
        };

        crate::ui::player_view::render(
            frame,
            area,
            &detail,
            playback,
            self.cache.covers.id_map(),
        );
    }
}
