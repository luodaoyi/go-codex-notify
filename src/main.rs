use anyhow::{bail, Result};
use codex_notify::{
    config::{legacy_config_path, Config},
    notify, payload,
};
use std::io;

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
        [arg] if arg == "settings-read" => {
            serde_json::to_writer(io::stdout().lock(), &Config::load()?)?;
            return Ok(());
        }
        [arg] if arg == "settings-save" => {
            let _lock = Config::settings_lock()?;
            let config: Config = serde_json::from_reader(io::stdin().lock())?;
            return config.save(&Config::credential()?);
        }
        [arg] if arg == "settings-import" => {
            let _lock = Config::settings_lock()?;
            return Config::import_legacy(
                &Config::credential()?,
                &legacy_config_path()?,
                Config::legacy_environment(),
            );
        }
        [arg] if arg == "hook" => {}
        _ => bail!("请通过 Codex Notify 插件运行通知 Hook"),
    }

    let mut payload = payload::read()?;
    if !matches!(payload.hook_event_name.as_str(), "Stop" | "SubagentStop") {
        bail!("不支持的 Codex Hook 事件：{}", payload.hook_event_name);
    }
    payload::enrich_user_input_from_transcript(&mut payload);
    let message = payload::build_message(&payload);
    if message.is_empty() {
        return Ok(());
    }
    let Some(config) = Config::load()? else {
        bail!("尚未保存渠道设置，请打开 Codex Notify 设置页");
    };
    if config.ignore_subagent_notifications && payload.hook_event_name == "SubagentStop" {
        return Ok(());
    }
    notify::send_all(&config, &message, &payload)
}
