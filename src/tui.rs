use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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

use crate::install::{self, InstallationStatus, Paths};

const ACTIONS: [&str; 3] = [
    "安装 / 更新并自动配置 Codex",
    "移除本工具的 Codex notify 配置",
    "退出",
];

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
    let mut selected = 0_usize;
    let mut message = String::from("选择第一项并按 Enter，即可完成 EXE 安装和 Codex 配置。");

    loop {
        terminal.draw(|frame| render(frame, &paths, status, selected, &message))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(ACTIONS.len() - 1);
            }
            KeyCode::Enter => match selected {
                0 => match install::apply() {
                    Ok(result) => {
                        status = install::status(&paths)?;
                        message =
                            format!("配置完成。Codex notify 已指向：{}", result.binary.display());
                    }
                    Err(error) => message = format!("配置失败：{error:#}"),
                },
                1 => match install::uninstall() {
                    Ok(true) => {
                        status = install::status(&paths)?;
                        message = "已移除本工具写入的 Codex notify 配置。".into();
                    }
                    Ok(false) => {
                        message = "当前 notify 不是本工具配置的，未做修改。".into();
                    }
                    Err(error) => message = format!("移除失败：{error:#}"),
                },
                _ => return Ok(()),
            },
            _ => {}
        }
    }
}

fn render(
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
