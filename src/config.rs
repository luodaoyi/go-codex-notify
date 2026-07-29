use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

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

    pub fn load_from_file() -> Result<Self> {
        let path = config_path().context("无法确定配置路径")?;
        Self::load_from_path(&path)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path().context("无法确定配置路径")?;
        self.save_to_path(&path)
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("解析通知配置失败：{}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => {
                Err(error).with_context(|| format!("读取通知配置失败：{}", path.display()))
            }
        }
    }

    fn save_to_path(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("无效配置路径")?;
        fs::create_dir_all(parent).context("创建配置目录失败")?;
        let mut normalized = self.clone();
        normalized.trim_values();
        let mut json = serde_json::to_vec_pretty(&normalized).context("序列化配置失败")?;
        json.push(b'\n');

        let mut temp_file = NamedTempFile::new_in(parent).context("创建临时文件失败")?;
        temp_file.write_all(&json).context("写入临时文件失败")?;
        temp_file.flush().context("刷新临时文件失败")?;
        temp_file.persist(path).context("原子保存配置失败")?;
        Ok(())
    }

    fn trim_values(&mut self) {
        self.bot_token = self.bot_token.trim().to_owned();
        self.chat_id = self.chat_id.trim().to_owned();
        self.openilink_hub_url = self.openilink_hub_url.trim().to_owned();
        self.openilink_hub_token = self.openilink_hub_token.trim().to_owned();
        self.hermes_webhook_url = self.hermes_webhook_url.trim().to_owned();
        self.hermes_webhook_secret = self.hermes_webhook_secret.trim().to_owned();
        self.bark_server_url = self.bark_server_url.trim().to_owned();
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

pub fn config_path() -> Option<PathBuf> {
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

    #[test]
    fn load_from_file_returns_empty_if_no_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let custom_path = temp_dir.path().join("missing.json");
        let config = Config::load_from_path(&custom_path).unwrap();
        assert_eq!(config.bot_token, "");
        assert_eq!(config.bark_server_url, "");
    }

    #[test]
    fn save_load_preservation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let custom_path = temp_dir.path().join("test.json");
        let config = Config {
            bot_token: "  newtoken  ".into(),
            chat_id: "123".into(),
            bark_server_url: "https://example.com/bark".into(),
            hermes_webhook_url: "".into(),
            openilink_hub_url: " https://example.com/hub ".into(),
            ..Config::default()
        };
        config.save_to_path(&custom_path).unwrap();
        let loaded = Config::load_from_path(&custom_path).unwrap();
        assert_eq!(loaded.bot_token, "newtoken");
        assert_eq!(loaded.chat_id, "123");
        assert_eq!(loaded.bark_server_url, "https://example.com/bark");
        assert_eq!(loaded.hermes_webhook_url, "");
        assert_eq!(loaded.openilink_hub_url, "https://example.com/hub");

        let updated = Config {
            bot_token: "updated-token".into(),
            ..loaded
        };
        updated.save_to_path(&custom_path).unwrap();
        let reloaded = Config::load_from_path(&custom_path).unwrap();
        assert_eq!(reloaded.bot_token, "updated-token");
        assert_eq!(reloaded.openilink_hub_url, "https://example.com/hub");
    }
}
