use std::collections::VecDeque;
use std::fs::{OpenOptions, File};
use std::io::Write;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::theme::Theme;

pub enum LogLevel {
    Error,
    Warn,
    Info,
}

pub struct LogEntry {
    pub time: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub message: String,
}

const MAX_MEMORY_ENTRIES: usize = 200;

pub struct LogStore {
    pub entries: VecDeque<LogEntry>,
    pub unread_count: usize,
    pub scroll: usize,
    file: Option<File>,
}

impl LogStore {
    pub fn new() -> Self {
        let file = dirs::cache_dir()
            .map(|d| d.join("hachimi.log"))
            .and_then(|path| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .ok()
            });

        Self {
            entries: VecDeque::new(),
            unread_count: 0,
            scroll: 0,
            file,
        }
    }

    pub fn push(&mut self, level: LogLevel, message: String) {
        let time = chrono::Local::now();

        // 写入文件
        if let Some(f) = &mut self.file {
            let level_str = match &level {
                LogLevel::Error => "ERROR",
                LogLevel::Warn => "WARN",
                LogLevel::Info => "INFO",
            };
            let _ = writeln!(f, "[{}] [{}] {}", time.format("%Y-%m-%d %H:%M:%S"), level_str, message);
        }

        self.entries.push_back(LogEntry { time, level, message });
        if self.entries.len() > MAX_MEMORY_ENTRIES {
            self.entries.pop_front();
        }
        self.unread_count += 1;
    }

    pub fn mark_read(&mut self) {
        self.unread_count = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        let max = self.entries.len().saturating_sub(1);
        if self.scroll < max {
            self.scroll += 1;
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, store: &LogStore) {
    let panel_w = 70u16.min(area.width.saturating_sub(4));
    let panel_h = 20u16.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(panel_w)) / 2;
    let y = area.y + (area.height.saturating_sub(panel_h)) / 2;
    let panel_area = Rect::new(x, y, panel_w, panel_h);

    frame.render_widget(Clear, panel_area);

    // 可用内容行数 = 面板高度 - 2 (border) - 1 (底部提示)
    let visible_lines = panel_h.saturating_sub(3) as usize;

    let mut lines: Vec<Line> = Vec::new();

    if store.entries.is_empty() {
        lines.push(Line::from(Span::styled("  No logs yet", Theme::secondary())));
    } else {
        let total = store.entries.len();
        let start = store.scroll.min(total.saturating_sub(visible_lines));
        let end = (start + visible_lines).min(total);

        for entry in store.entries.range(start..end) {
            let time_str = entry.time.format("%H:%M:%S").to_string();
            let (level_str, level_style) = match &entry.level {
                LogLevel::Error => ("ERROR", Style::default().fg(Color::Red)),
                LogLevel::Warn => (" WARN", Style::default().fg(Color::Yellow)),
                LogLevel::Info => (" INFO", Style::default()),
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {time_str} "), Theme::secondary()),
                Span::styled(format!("{level_str} "), level_style),
                Span::raw(&entry.message),
            ]));
        }
    }

    // 底部提示
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "    j/k scroll  ·  Esc/! close",
        Theme::secondary(),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Logs ",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, panel_area);
}
