use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::config::Config;
use crate::install::{self, InstallationStatus, Paths};

const ACTIONS: [&str; 4] = [
    "安装 / 更新并自动配置 Codex",
    "移除本工具的 Codex notify 配置",
    "通知配置",
    "退出",
];

const TEXT_FIELDS: [&str; 4] = [
    "TELEGRAM_BOT_TOKEN",
    "TELEGRAM_CHAT_ID",
    "BARK_SERVER_URL",
    "HERMES_WEBHOOK_URL",
];
const FIELD_COUNT: usize = TEXT_FIELDS.len() + 1;

fn mask(s: &str) -> String {
    "•".repeat(s.chars().count())
}

#[derive(Clone, Debug)]
enum State {
    Menu { selected: usize, message: String },
    Config(Box<ConfigForm>),
}

#[derive(Clone, Debug)]
struct ConfigForm {
    config: Config,
    focus: usize,
    values: [String; 4],
    ignore_subagent_notifications: bool,
    editing: bool,
    message: String,
}

impl ConfigForm {
    fn load() -> Result<Self> {
        let config = Config::load_from_file()?;
        Ok(Self::from_config(config))
    }

    fn from_config(config: Config) -> Self {
        let values = [
            config.bot_token.clone(),
            config.chat_id.clone(),
            config.bark_server_url.clone(),
            config.hermes_webhook_url.clone(),
        ];
        Self {
            ignore_subagent_notifications: config.ignore_subagent_notifications,
            config,
            focus: 0,
            values,
            editing: false,
            message: "选择字段后按 Enter 编辑；空字段不会启用对应渠道。".into(),
        }
    }

    fn current_value_mut(&mut self) -> &mut String {
        &mut self.values[self.focus]
    }

    fn config_to_save(&self) -> Config {
        let mut config = self.config.clone();
        config.bot_token = self.values[0].trim().to_owned();
        config.chat_id = self.values[1].trim().to_owned();
        config.bark_server_url = self.values[2].trim().to_owned();
        config.hermes_webhook_url = self.values[3].trim().to_owned();
        config.ignore_subagent_notifications = self.ignore_subagent_notifications;
        config
    }

    fn save(&mut self) {
        let config = self.config_to_save();
        match config.save() {
            Ok(()) => {
                self.config = config;
                self.values = [
                    self.config.bot_token.clone(),
                    self.config.chat_id.clone(),
                    self.config.bark_server_url.clone(),
                    self.config.hermes_webhook_url.clone(),
                ];
                self.editing = false;
                self.ignore_subagent_notifications = self.config.ignore_subagent_notifications;
                self.message = match crate::config::config_path() {
                    Some(path) => format!("配置已保存：{}", path.display()),
                    None => "配置已保存。".into(),
                };
            }
            Err(error) => self.message = format!("保存失败：{error:#}"),
        }
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode().context("启用终端原始模式失败")?;
    execute!(io::stdout(), EnterAlternateScreen).context("进入 TUI 屏幕失败")?;
    let _restore = RestoreTerminal;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("创建 TUI 终端失败")?;
    let result = run_loop(&mut terminal);
    terminal.show_cursor().ok();
    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let paths = Paths::discover()?;
    let mut status = install::status(&paths)?;
    let mut state = State::Menu {
        selected: 0,
        message: "选择第一项并按 Enter，即可完成 EXE 安装和 Codex 配置。".into(),
    };

    loop {
        terminal.draw(|frame| match &state {
            State::Menu { selected, message } => {
                render_menu(frame, &paths, status, *selected, message)
            }
            State::Config(form) => render_config(frame, form),
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && handle_key(key, &mut state, &paths, &mut status)? =>
            {
                return Ok(());
            }
            Event::Paste(text) => {
                if let State::Config(form) = &mut state {
                    if form.editing {
                        form.current_value_mut().push_str(&text);
                    }
                }
            }
            _ => {}
        }
    }
}

fn handle_key(
    key: KeyEvent,
    state: &mut State,
    paths: &Paths,
    status: &mut InstallationStatus,
) -> Result<bool> {
    match state {
        State::Menu { selected, message } => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                *selected = (*selected + 1).min(ACTIONS.len() - 1);
            }
            KeyCode::Enter => match *selected {
                0 => match install::apply() {
                    Ok(result) => {
                        *status = install::status(paths)?;
                        *message =
                            format!("配置完成。Codex notify 已指向：{}", result.binary.display());
                    }
                    Err(error) => *message = format!("配置失败：{error:#}"),
                },
                1 => match install::uninstall() {
                    Ok(true) => {
                        *status = install::status(paths)?;
                        *message = "已移除本工具写入的 Codex notify 配置。".into();
                    }
                    Ok(false) => {
                        *message = "当前 notify 不是本工具配置的，未做修改。".into();
                    }
                    Err(error) => *message = format!("移除失败：{error:#}"),
                },
                2 => match ConfigForm::load() {
                    Ok(form) => *state = State::Config(Box::new(form)),
                    Err(error) => *message = format!("读取通知配置失败：{error:#}"),
                },
                _ => return Ok(true),
            },
            _ => {}
        },
        State::Config(form) => {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('s'))
            {
                form.save();
                return Ok(false);
            }
            if form.editing {
                match key.code {
                    KeyCode::Enter => form.editing = false,
                    KeyCode::Esc => {
                        *state = State::Menu {
                            selected: 2,
                            message: "已取消通知配置修改，文件未保存。".into(),
                        };
                    }
                    KeyCode::Backspace => {
                        form.current_value_mut().pop();
                    }
                    KeyCode::Delete => form.current_value_mut().clear(),
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        form.current_value_mut().clear();
                    }
                    KeyCode::Char(character)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        form.current_value_mut().push(character);
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        *state = State::Menu {
                            selected: 2,
                            message: "已返回主菜单。".into(),
                        };
                    }
                    KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                        form.focus = form.focus.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                        form.focus = (form.focus + 1).min(FIELD_COUNT - 1);
                    }
                    KeyCode::Enter | KeyCode::Char(' ') if form.focus == TEXT_FIELDS.len() => {
                        form.ignore_subagent_notifications = !form.ignore_subagent_notifications;
                    }
                    KeyCode::Enter => form.editing = true,
                    KeyCode::Char('s') => form.save(),
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}

fn render_menu(
    frame: &mut Frame,
    paths: &Paths,
    status: InstallationStatus,
    selected: usize,
    message: &str,
) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "codex-notify",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  Rust 安装配置"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, areas[0]);

    let status_text = vec![
        Line::from(status_line("原生 EXE", status.binary_installed)),
        Line::from(status_line("Codex notify", status.hook_configured)),
        Line::from(format!("目标路径：{}", paths.installed_binary.display())),
        Line::from(format!("配置文件：{}", paths.config.display())),
    ];
    frame.render_widget(
        Paragraph::new(status_text)
            .block(Block::default().title(" 状态 ").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        areas[1],
    );

    let items: Vec<ListItem> = ACTIONS
        .iter()
        .map(|action| ListItem::new(*action))
        .collect();
    let actions = List::new(items)
        .block(Block::default().title(" 操作 ").borders(Borders::ALL))
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let mut list_state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(actions, areas[2], &mut list_state);

    frame.render_widget(
        Paragraph::new(message)
            .block(Block::default().title(" 结果 ").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        areas[3],
    );

    frame.render_widget(
        Paragraph::new("↑/↓ 或 j/k 选择 · Enter 执行 · q/Esc 退出")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        areas[4],
    );
}

fn render_config(frame: &mut Frame, form: &ConfigForm) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(7),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new("通知渠道配置")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL)),
        areas[0],
    );

    let path = crate::config::config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<无法确定>".into());
    frame.render_widget(
        Paragraph::new(format!("配置文件：{path}")).block(Block::default().borders(Borders::ALL)),
        areas[1],
    );

    let mut items: Vec<ListItem> = TEXT_FIELDS
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let value = if form.values[index].is_empty() {
                "<未设置>".into()
            } else if index == 0 {
                mask(&form.values[index])
            } else {
                form.values[index].clone()
            };
            let editing = if index == form.focus && form.editing {
                "  [编辑中]"
            } else {
                ""
            };
            ListItem::new(format!("{label}: {value}{editing}"))
        })
        .collect();
    items.push(ListItem::new(format!(
        "忽略 SubAgent 通知: {}",
        if form.ignore_subagent_notifications {
            "是（仅通知主代理）"
        } else {
            "否（全部通知）"
        }
    )));
    let fields = List::new(items)
        .block(Block::default().title(" 配置项 ").borders(Borders::ALL))
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(if form.editing {
                    Color::Green
                } else {
                    Color::Cyan
                })
                .add_modifier(Modifier::BOLD),
        );
    let mut list_state = ListState::default().with_selected(Some(form.focus));
    frame.render_stateful_widget(fields, areas[2], &mut list_state);

    frame.render_widget(
        Paragraph::new(form.message.as_str())
            .block(Block::default().title(" 结果 ").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        areas[3],
    );

    frame.render_widget(
        Paragraph::new(
            "↑/↓/Tab 切换 · Enter 编辑/切换 · Ctrl+U/Delete 清空 · Ctrl+S 或 s 保存 · Esc 取消",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray)),
        areas[4],
    );
}

fn status_line(label: &str, ready: bool) -> Vec<Span<'static>> {
    vec![
        Span::raw(format!("{label}：")),
        Span::styled(
            if ready { "已就绪" } else { "未配置" },
            Style::default().fg(if ready { Color::Green } else { Color::Yellow }),
        ),
    ]
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_form_preserves_fields_not_shown_in_tui() {
        let source = Config {
            openilink_hub_url: "https://hub.example".into(),
            openilink_hub_token: "secret".into(),
            hermes_webhook_secret: "signing-secret".into(),
            ..Config::default()
        };
        let mut form = ConfigForm::from_config(source);
        form.values[2] = " https://bark.example/device ".into();

        let result = form.config_to_save();

        assert_eq!(result.bark_server_url, "https://bark.example/device");
        assert_eq!(result.openilink_hub_url, "https://hub.example");
        assert_eq!(result.openilink_hub_token, "secret");
        assert_eq!(result.hermes_webhook_secret, "signing-secret");
        assert!(!result.ignore_subagent_notifications);
    }

    #[test]
    fn token_mask_uses_character_count() {
        assert_eq!(mask("ab中"), "•••");
    }
}
