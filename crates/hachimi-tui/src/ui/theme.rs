use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn highlight() -> Style {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub fn secondary() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn active() -> Style {
        Style::default().fg(Color::Cyan)
    }

    pub fn error() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn success() -> Style {
        Style::default().fg(Color::Green)
    }

    pub fn normal() -> Style {
        Style::default()
    }

    pub fn title() -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    pub fn selected_row() -> Style {
        Style::default().bg(Color::DarkGray)
    }

    pub fn progress_filled() -> Style {
        Style::default().fg(Color::Cyan)
    }

    pub fn progress_empty() -> Style {
        Style::default().fg(Color::DarkGray)
    }
}
