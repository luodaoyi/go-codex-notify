use anyhow::{bail, Result};
use codex_notify::{config::Config, notify, payload};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [arg] if arg == "--version" => {
            let version = option_env!("CODEX_NOTIFY_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
            println!("codex-notify {version}");
            return Ok(());
        }
        [arg] if arg == "hook" => {}
        _ => bail!("请通过 Codex Notify 插件运行通知 Hook"),
    }

    let mut payload = payload::read()?;
    if !matches!(payload.hook_event_name.as_str(), "Stop" | "SubagentStop") {
        bail!("不支持的 Codex Hook 事件：{}", payload.hook_event_name);
    }
    let config = Config::load()?;
    if config.ignore_subagent_notifications && payload.hook_event_name == "SubagentStop" {
        return Ok(());
    }

    payload::enrich_user_input_from_transcript(&mut payload);
    let message = payload::build_message(&payload);
    if message.is_empty() {
        return Ok(());
    }
    notify::send_all(&config, &message, &payload)
}
