use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

/// 标签调色板：用于给不同标签分配不同的背景色块
const TAG_COLORS: &[Color] = &[
    Color::Blue,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Cyan,
    Color::Yellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightRed,
];

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

    /// 按索引返回色块样式，自动跳过 avoid 颜色
    pub fn tag_badge(index: usize, avoid: Option<Color>) -> Style {
        let bg = Self::tag_color(index, avoid);
        let fg = match bg {
            Color::Yellow | Color::LightGreen | Color::Cyan | Color::LightBlue => Color::Black,
            _ => Color::White,
        };
        Style::default().bg(bg).fg(fg)
    }

    /// 按索引返回颜色，若与 avoid 撞色则顺移
    pub fn tag_color(index: usize, avoid: Option<Color>) -> Color {
        let mut idx = index % TAG_COLORS.len();
        if let Some(prev) = avoid {
            if TAG_COLORS[idx] == prev {
                idx = (idx + 1) % TAG_COLORS.len();
            }
        }
        TAG_COLORS[idx]
    }

    /// 外部链接固定样式
    pub fn link_badge() -> Style {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    }
}
