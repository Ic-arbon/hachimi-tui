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

pub const ITEMS_COUNT: usize = 2;

pub fn render_list(frame: &mut Frame, area: Rect, settings: &Settings, selected: usize) {
    let items: Vec<ListItem> = vec![
        setting_item(0, selected, t!("settings.language"), lang_label(settings.display.language)),
        setting_item(1, selected, t!("settings.play_mode"), play_mode_label(&settings.player.default_play_mode)),
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

pub fn render_hint(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", t!("settings.hint")),
            Theme::secondary(),
        )),
    ];
    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn setting_item<'a>(
    index: usize,
    selected: usize,
    label: &'static str,
    value: &'static str,
) -> ListItem<'a> {
    let is_sel = index == selected;
    let label_style = if is_sel {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let value_style = if is_sel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Theme::secondary()
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!("  {} : ", label), label_style),
        Span::styled(value, value_style),
    ]))
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
        _ => {}
    }
}
