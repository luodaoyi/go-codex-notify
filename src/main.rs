use std::io::{self, IsTerminal};

use anyhow::{bail, Result};
use codex_notify::{config::Config, install, notify, payload, tui};

const HELP: &str = r#"codex-notify - Codex 多渠道通知工具

用法:
  codex-notify                 启动安装配置 TUI
  codex-notify install         安装 EXE 并配置 Codex notify
  codex-notify status          查看安装状态
  codex-notify uninstall       移除本工具写入的 Codex notify
  codex-notify notify [JSON]   手动处理通知 payload

Codex 会把 JSON 作为参数传入；也兼容从 stdin 读取 JSON。
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("-h" | "--help") => {
            print!("{HELP}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("codex-notify {}", build_version());
            return Ok(());
        }
        Some("ui") => return tui::run(),
        Some("install" | "apply") => {
            let result = install::apply()?;
            println!("已安装：{}", result.binary.display());
            println!("已配置：{}", result.config.display());
            return Ok(());
        }
        Some("status") => {
            let paths = install::Paths::discover()?;
            let status = install::status(&paths)?;
            println!(
                "EXE：{}",
                if status.binary_installed {
                    "已安装"
                } else {
                    "未安装"
                }
            );
            println!(
                "Codex notify：{}",
                if status.hook_configured {
                    "已配置"
                } else {
                    "未配置"
                }
            );
            println!("路径：{}", paths.installed_binary.display());
            return Ok(());
        }
        Some("uninstall" | "remove") => {
            let changed = install::uninstall()?;
            println!(
                "{}",
                if changed {
                    "已移除 codex-notify 的 Codex notify 配置。"
                } else {
                    "当前 Codex notify 不是本工具配置的，未做修改。"
                }
            );
            return Ok(());
        }
        Some("notify") => {
            args.remove(0);
            return run_notify(&args);
        }
        _ => {}
    }

    if args.is_empty() && io::stdin().is_terminal() && io::stdout().is_terminal() {
        return tui::run();
    }

    if args.first().is_some_and(|arg| arg.starts_with('-')) {
        bail!("未知参数：{}\n\n{HELP}", args[0]);
    }

    run_notify(&args)
}

fn build_version() -> &'static str {
    option_env!("CODEX_NOTIFY_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn run_notify(args: &[String]) -> Result<()> {
    let config = Config::load()?;
    let (mut payload, raw_input) = payload::read(args)?;
    payload::enrich_goal_from_transcript(&mut payload);
    let message = payload::build_message(&payload, &raw_input);
    notify::send_all(&config, &message, &payload)
}
