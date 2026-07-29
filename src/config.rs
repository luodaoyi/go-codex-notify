use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub openilink_hub_url: String,
    #[serde(default)]
    pub openilink_hub_token: String,
    #[serde(default)]
    pub hermes_webhook_url: String,
    #[serde(default)]
    pub hermes_webhook_secret: String,
    #[serde(default)]
    pub bark_server_url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Self {
            bot_token: env_value("TELEGRAM_BOT_TOKEN"),
            chat_id: env_value("TELEGRAM_CHAT_ID"),
            openilink_hub_url: env_value("OPENILINK_HUB_URL"),
            openilink_hub_token: env_value("OPENILINK_HUB_TOKEN"),
            hermes_webhook_url: env_value("HERMES_WEBHOOK_URL"),
            hermes_webhook_secret: env_value("HERMES_WEBHOOK_SECRET"),
            bark_server_url: env_value("BARK_SERVER_URL"),
        };

        if let Some(path) = config_path() {
            match fs::read(&path) {
                Ok(bytes) => {
                    let file_config: Self = serde_json::from_slice(&bytes)
                        .with_context(|| format!("解析通知配置失败：{}", path.display()))?;
                    config.fill_empty_from(file_config);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("读取通知配置失败：{}", path.display()));
                }
            }
        }

        Ok(config)
    }

    fn fill_empty_from(&mut self, other: Self) {
        fill(&mut self.bot_token, other.bot_token);
        fill(&mut self.chat_id, other.chat_id);
        fill(&mut self.openilink_hub_url, other.openilink_hub_url);
        fill(&mut self.openilink_hub_token, other.openilink_hub_token);
        fill(&mut self.hermes_webhook_url, other.hermes_webhook_url);
        fill(&mut self.hermes_webhook_secret, other.hermes_webhook_secret);
        fill(&mut self.bark_server_url, other.bark_server_url);
    }
}

fn fill(target: &mut String, source: String) {
    if target.is_empty() {
        *target = source.trim().to_owned();
    }
}

fn env_value(key: &str) -> String {
    env::var(key).unwrap_or_default().trim().to_owned()
}

fn config_path() -> Option<PathBuf> {
    let explicit = env_value("CODEX_NOTIFY_CONFIG");
    if !explicit.is_empty() {
        return Some(PathBuf::from(explicit));
    }
    dirs::home_dir().map(|home| home.join(".codex").join("codex-notify.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_only_empty_values() {
        let mut config = Config {
            bot_token: "from-env".into(),
            ..Config::default()
        };
        config.fill_empty_from(Config {
            bot_token: "from-file".into(),
            chat_id: " chat ".into(),
            ..Config::default()
        });
        assert_eq!(config.bot_token, "from-env");
        assert_eq!(config.chat_id, "chat");
    }
}
