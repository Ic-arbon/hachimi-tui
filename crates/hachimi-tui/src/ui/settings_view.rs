use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::config::settings::{PlayMode, Settings};
use super::i18n::Lang;
use super::theme::Theme;

pub const ITEMS_COUNT: usize = 4;

pub fn render_list(frame: &mut Frame, area: Rect, settings: &Settings, selected: usize) {
    let items: Vec<ListItem> = vec![
        setting_item(0, selected, t!("settings.language"), lang_label(settings.display.language)),
        setting_item(1, selected, t!("settings.play_mode"), play_mode_label(&settings.player.default_play_mode)),
        setting_item(2, selected, t!("settings.replay_gain"), bool_label(settings.player.replay_gain)),
        setting_item_owned(3, selected, t!("settings.cover_scale"), format!("{}%", settings.display.cover_scale)),
    ];

    let list = List::new(items);
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Preview 栏渲染（无选中高亮，全部 secondary 风格）
pub fn render_preview(frame: &mut Frame, area: Rect, settings: &Settings) {
    let items: Vec<ListItem> = vec![
        preview_item(t!("settings.language"), lang_label(settings.display.language)),
        preview_item(t!("settings.play_mode"), play_mode_label(&settings.player.default_play_mode)),
        preview_item(t!("settings.replay_gain"), bool_label(settings.player.replay_gain)),
        preview_item_owned(t!("settings.cover_scale"), format!("{}%", settings.display.cover_scale)),
    ];
    let list = List::new(items);
    frame.render_widget(list, area);
}

fn preview_item<'a>(label: &'static str, value: &'static str) -> ListItem<'a> {
    ListItem::new(Line::from(vec![
        Span::styled(format!(" {} : ", label), Theme::secondary()),
        Span::styled(value, Theme::secondary()),
    ]))
}

fn preview_item_owned<'a>(label: &'static str, value: String) -> ListItem<'a> {
    ListItem::new(Line::from(vec![
        Span::styled(format!(" {} : ", label), Theme::secondary()),
        Span::styled(value, Theme::secondary()),
    ]))
}

pub fn render_hint(frame: &mut Frame, area: Rect, selected: usize, settings: &Settings) {
    let desc_key = match selected {
        0 => "settings.desc.language",
        1 => "settings.desc.play_mode",
        2 => "settings.desc.replay_gain",
        3 => "settings.desc.cover_scale",
        _ => "",
    };
    let mut lines = Vec::new();
    if !desc_key.is_empty() {
        lines.push(Line::from(Span::styled(
            t!(desc_key),
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::from(""));
    }

    // 列出当前选项的可选值
    match selected {
        0 => {
            let current = settings.display.language;
            for (lang, label, desc) in [
                (Lang::En, "English", t!("settings.lang.en.desc")),
                (Lang::Zh, "中文", t!("settings.lang.zh.desc")),
            ] {
                let active = lang == current;
                let marker = if active { "● " } else { "○ " };
                let style = if active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Theme::secondary()
                };
                lines.push(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(label, style),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("  {}", desc),
                    Theme::secondary(),
                )));
            }
        }
        3 => {
            let pct = settings.display.cover_scale;
            let bar_width = 20usize;
            let filled = (pct as usize * bar_width / 100).min(bar_width);
            let empty = bar_width - filled;
            lines.push(Line::from(vec![
                Span::styled("▕", Theme::secondary()),
                Span::styled("█".repeat(filled), Style::default().fg(Color::Yellow)),
                Span::styled("░".repeat(empty), Theme::secondary()),
                Span::styled("▏", Theme::secondary()),
                Span::styled(format!(" {}%", pct), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
        }
        _ => {}
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t!("settings.hint"),
        Theme::secondary(),
    )));
    let inner = Rect { x: area.x + 2, width: area.width.saturating_sub(4), ..area };
    let para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn setting_item<'a>(
    index: usize,
    selected: usize,
    label: &'static str,
    value: &'static str,
) -> ListItem<'a> {
    let is_sel = index == selected;
    let (label_style, value_style) = item_styles(is_sel);
    ListItem::new(Line::from(vec![
        Span::styled(format!("  {} : ", label), label_style),
        Span::styled(value, value_style),
    ]))
}

fn setting_item_owned<'a>(
    index: usize,
    selected: usize,
    label: &'static str,
    value: String,
) -> ListItem<'a> {
    let is_sel = index == selected;
    let (label_style, value_style) = item_styles(is_sel);
    ListItem::new(Line::from(vec![
        Span::styled(format!("  {} : ", label), label_style),
        Span::styled(value, value_style),
    ]))
}

fn item_styles(is_sel: bool) -> (Style, Style) {
    if is_sel {
        (
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::default(), Theme::secondary())
    }
}

fn lang_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::Zh => "中文",
    }
}

fn play_mode_label(mode: &PlayMode) -> &'static str {
    match mode {
        PlayMode::Sequential => t!("settings.sequential"),
        PlayMode::Shuffle => t!("settings.shuffle"),
        PlayMode::RepeatOne => t!("settings.repeat_one"),
    }
}

fn bool_label(val: bool) -> &'static str {
    if val { t!("settings.on") } else { t!("settings.off") }
}

/// Cycle the setting at the given index.
pub fn cycle_setting(settings: &mut Settings, index: usize) {
    match index {
        0 => {
            settings.display.language = settings.display.language.next();
            crate::ui::i18n::set_lang(settings.display.language);
        }
        1 => {
            settings.player.default_play_mode = match settings.player.default_play_mode {
                PlayMode::Sequential => PlayMode::Shuffle,
                PlayMode::Shuffle => PlayMode::RepeatOne,
                PlayMode::RepeatOne => PlayMode::Sequential,
            };
        }
        2 => {
            settings.player.replay_gain = !settings.player.replay_gain;
        }
        3 => {
            let v = settings.display.cover_scale;
            settings.display.cover_scale = if v >= 100 { 20 } else { v + 10 };
        }
        _ => {}
    }
}
