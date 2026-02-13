use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Email,
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStep {
    /// 输入邮箱和密码
    Input,
    /// 正在生成 captcha
    GeneratingCaptcha,
    /// 等待用户在浏览器中完成 captcha
    WaitingCaptcha,
    /// 正在提交登录请求
    Submitting,
}

pub struct LoginState {
    pub email: String,
    pub password: String,
    pub focused_field: LoginField,
    pub email_cursor: usize,
    pub password_cursor: usize,
    pub error: Option<String>,
    pub step: LoginStep,
    pub captcha_key: Option<String>,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            email: String::new(),
            password: String::new(),
            focused_field: LoginField::Email,
            email_cursor: 0,
            password_cursor: 0,
            error: None,
            step: LoginStep::Input,
            captcha_key: None,
        }
    }

    pub fn current_input(&mut self) -> (&mut String, &mut usize) {
        match self.focused_field {
            LoginField::Email => (&mut self.email, &mut self.email_cursor),
            LoginField::Password => (&mut self.password, &mut self.password_cursor),
        }
    }

    pub fn toggle_field(&mut self) {
        self.focused_field = match self.focused_field {
            LoginField::Email => LoginField::Password,
            LoginField::Password => LoginField::Email,
        };
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self.step,
            LoginStep::GeneratingCaptcha | LoginStep::Submitting
        )
    }
}

/// 渲染登录表单（居中显示在主内容区）
pub fn render(frame: &mut Frame, area: Rect, state: &LoginState) {
    // 垂直居中
    let form_height = 16u16;
    let v_pad = area.height.saturating_sub(form_height) / 2;
    let v_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_pad),
            Constraint::Length(form_height),
            Constraint::Min(0),
        ])
        .split(area);

    // 水平居中，表单宽度 44
    let form_width = 44u16.min(area.width.saturating_sub(4));
    let h_pad = area.width.saturating_sub(form_width) / 2;
    let h_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_pad),
            Constraint::Length(form_width),
            Constraint::Min(0),
        ])
        .split(v_layout[1]);

    let form_area = h_layout[1];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // 标题
            Constraint::Length(1), // Email 标签
            Constraint::Length(1), // Email 输入
            Constraint::Length(1), // 空行
            Constraint::Length(1), // Password 标签
            Constraint::Length(1), // Password 输入
            Constraint::Length(1), // 空行
            Constraint::Length(1), // 提示行 1
            Constraint::Length(1), // 提示行 2
            Constraint::Length(1), // 空行
            Constraint::Length(1), // 错误信息
            Constraint::Min(0),
        ])
        .split(form_area);

    // 标题
    let title = Paragraph::new(Line::from(Span::styled(
        "LOGIN",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, rows[0]);

    let input_dimmed = state.step != LoginStep::Input;

    // Email
    let email_label_style = if !input_dimmed && state.focused_field == LoginField::Email {
        Theme::highlight()
    } else {
        Theme::secondary()
    };
    let email_label = Paragraph::new(Span::styled("Email", email_label_style));
    frame.render_widget(email_label, rows[1]);

    let email_line = build_input_line(
        &state.email,
        state.email_cursor,
        !input_dimmed && state.focused_field == LoginField::Email,
        false,
    );
    frame.render_widget(Paragraph::new(email_line), rows[2]);

    // Password
    let pw_label_style = if !input_dimmed && state.focused_field == LoginField::Password {
        Theme::highlight()
    } else {
        Theme::secondary()
    };
    let pw_label = Paragraph::new(Span::styled("Password", pw_label_style));
    frame.render_widget(pw_label, rows[4]);

    let pw_line = build_input_line(
        &state.password,
        state.password_cursor,
        !input_dimmed && state.focused_field == LoginField::Password,
        true,
    );
    frame.render_widget(Paragraph::new(pw_line), rows[5]);

    // 提示行（根据 step 不同显示不同内容）
    match state.step {
        LoginStep::Input => {
            let hint = Line::from(vec![
                Span::styled("[Enter]", Theme::highlight()),
                Span::raw(" Login  "),
                Span::styled("[q]", Theme::secondary()),
                Span::raw(" Quit"),
            ]);
            frame.render_widget(Paragraph::new(hint), rows[7]);
        }
        LoginStep::GeneratingCaptcha => {
            let hint = Line::from(Span::styled(
                "Generating captcha...",
                Theme::active(),
            ));
            frame.render_widget(Paragraph::new(hint), rows[7]);
        }
        LoginStep::WaitingCaptcha => {
            let hint1 = Line::from(Span::styled(
                "Captcha opened in browser",
                Theme::active(),
            ));
            let hint2 = Line::from(vec![
                Span::styled("[Enter]", Theme::highlight()),
                Span::raw(" Continue after completing captcha"),
            ]);
            frame.render_widget(Paragraph::new(hint1), rows[7]);
            frame.render_widget(Paragraph::new(hint2), rows[8]);
        }
        LoginStep::Submitting => {
            let hint = Line::from(Span::styled("Logging in...", Theme::active()));
            frame.render_widget(Paragraph::new(hint), rows[7]);
        }
    }

    // 错误信息
    if let Some(err) = &state.error {
        let err_line = Paragraph::new(Span::styled(err.as_str(), Theme::error()));
        frame.render_widget(err_line, rows[10]);
    }
}

/// 构建输入行：> content  (光标用反色块表示)
fn build_input_line(
    text: &str,
    cursor: usize,
    is_focused: bool,
    is_password: bool,
) -> Line<'static> {
    let prefix = if is_focused { "> " } else { "  " };
    let display: String = if is_password {
        "\u{2022}".repeat(text.len())
    } else {
        text.to_string()
    };

    if is_focused {
        let before: String = display.chars().take(cursor).collect();
        let cursor_char: String = display
            .chars()
            .nth(cursor)
            .map_or(" ".to_string(), |c| c.to_string());
        let after: String = display.chars().skip(cursor + 1).collect();

        Line::from(vec![
            Span::styled(prefix.to_string(), Theme::active()),
            Span::raw(before),
            Span::styled(
                cursor_char,
                Style::default().add_modifier(Modifier::REVERSED),
            ),
            Span::raw(after),
        ])
    } else {
        Line::from(vec![
            Span::styled(prefix.to_string(), Theme::secondary()),
            Span::styled(display, Theme::secondary()),
        ])
    }
}
