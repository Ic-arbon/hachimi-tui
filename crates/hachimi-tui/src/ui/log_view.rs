use std::collections::VecDeque;
use std::fs::{OpenOptions, File};
use std::io::Write;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::theme::Theme;

pub enum LogLevel {
    Error,
    #[allow(dead_code)] // TODO: 警告日志
    Warn,
    #[allow(dead_code)] // TODO: 信息日志
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
    pub h_scroll: u16,
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
            h_scroll: 0,
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

    pub fn scroll_left(&mut self) {
        self.h_scroll = self.h_scroll.saturating_sub(4);
    }

    pub fn scroll_right(&mut self) {
        self.h_scroll = self.h_scroll.saturating_add(4);
    }
}

pub fn render(frame: &mut Frame, area: Rect, store: &LogStore) {
    let (content_area, hint_area) = super::util::overlay_panel(
        frame, area, t!("logs.title"),
        super::constants::LOG_PANEL_WIDTH, super::constants::LOG_PANEL_HEIGHT,
    );

    let visible_lines = content_area.height as usize;

    let mut lines: Vec<Line> = Vec::new();

    if store.entries.is_empty() {
        lines.push(Line::from(Span::styled(format!("  {}", t!("logs.empty")), Theme::secondary())));
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

    let para = Paragraph::new(lines).scroll((0, store.h_scroll));
    frame.render_widget(para, content_area);

    // 固定提示（不受滚动影响）
    let hint = Paragraph::new(Span::styled(
        format!("    {}", t!("logs.hint")),
        Theme::secondary(),
    ));
    frame.render_widget(hint, hint_area);
}
