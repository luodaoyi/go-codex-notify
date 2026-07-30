use std::io::{self, IsTerminal};

use anyhow::{bail, Result};
use codex_notify::{config::Config, install, notify, payload, tui};

const HELP: &str = r#"codex-notify - Codex 多渠道通知工具

用法:
  codex-notify                 启动安装配置 TUI
  codex-notify install         安装 EXE 并配置 Codex Hooks
  codex-notify status          查看安装状态
  codex-notify uninstall       移除本工具写入的 Codex Hooks
  codex-notify hook            处理 Codex Hook stdin
  codex-notify notify [JSON]   手动处理通知 payload

Hook 从 stdin 读取 JSON；notify 兼容参数或 stdin。
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
            println!("已配置：{}", result.hooks.display());
            println!("首次使用或 Hook 变更后，请在 Codex 中运行 /hooks 并信任该 Hook。");
            return Ok(());
        }
        Some("status") => {
            let paths = install::Paths::discover()?;
            let status = install::status(&paths)?;
            println!(
                "EXE：{}",
                if status.binary_current {
                    "已安装且与当前版本同步"
                } else if status.binary_installed {
                    "已安装但需要更新"
                } else {
                    "未安装"
                }
            );
            println!(
                "Codex Stop Hook：{}",
                if status.hook_configured {
                    "已配置"
                } else {
                    "未配置"
                }
            );
            println!(
                "Codex SubagentStop Hook：{}",
                if status.subagent_hook_configured {
                    "已配置（通知全部代理）"
                } else {
                    "未配置（仅通知主代理）"
                }
            );
            if status.legacy_notify_configured {
                println!("旧 notify：仍存在，请重新运行 install 完成迁移");
            }
            println!("路径：{}", paths.installed_binary.display());
            println!("Hooks：{}", paths.hooks.display());
            return Ok(());
        }
        Some("uninstall" | "remove") => {
            let changed = install::uninstall()?;
            println!(
                "{}",
                if changed {
                    "已移除 codex-notify 的 Codex Hooks/旧 notify 配置。"
                } else {
                    "没有找到本工具写入的 Codex Hooks/旧 notify，未做修改。"
                }
            );
            return Ok(());
        }
        Some("hook") => {
            args.remove(0);
            return run_notify(&args, false);
        }
        Some("notify") => {
            args.remove(0);
            return run_notify(&args, true);
        }
        _ => {}
    }

    if args.is_empty() && io::stdin().is_terminal() && io::stdout().is_terminal() {
        return tui::run();
    }

    if args.first().is_some_and(|arg| arg.starts_with('-')) {
        bail!("未知参数：{}\n\n{HELP}", args[0]);
    }

    run_notify(&args, true)
}

fn build_version() -> &'static str {
    option_env!("CODEX_NOTIFY_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn run_notify(args: &[String], filter_legacy_subagent: bool) -> Result<()> {
    let config = Config::load()?;
    let (mut payload, _) = payload::read(args)?;
    if filter_legacy_subagent && config.ignore_subagent_notifications && payload.is_subagent() {
        return Ok(());
    }
    payload::enrich_user_input_from_transcript(&mut payload);
    let message = payload::build_message(&payload);
    notify::send_all(&config, &message, &payload)
}
