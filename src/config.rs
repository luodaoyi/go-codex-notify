use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Default, Deserialize, Serialize)]
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
    #[serde(default)]
    pub ignore_subagent_notifications: bool,
}

impl Config {
    pub fn settings_lock() -> Result<fs::File> {
        let data = plugin_data()?;
        fs::create_dir_all(&data).context("无法创建插件数据目录")?;
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(data.join("settings.lock"))
            .context("无法打开设置锁")?;
        fs2::FileExt::lock_exclusive(&file).context("无法锁定设置")?;
        Ok(file)
    }

    pub fn credential() -> Result<Entry> {
        let data = plugin_data()?;
        // Use the same key for the settings launcher and the hook, even if the directory
        // does not yet exist. Codex canonicalizes its home before constructing PLUGIN_DATA.
        let root = data.ancestors().nth(3).context("无效的插件数据目录")?;
        let canonical = root.canonicalize().context("无法定位 Codex 配置目录")?;
        let relative = data.strip_prefix(root).context("无效的插件数据目录")?;
        let hash = Sha256::digest(canonical.join(relative).as_os_str().as_encoded_bytes());
        Entry::new("Codex Notify", &hex::encode(hash)).context("无法打开系统凭据库")
    }

    pub fn load() -> Result<Option<Self>> {
        Self::read_credential(&Self::credential()?)
    }

    pub fn read_credential(entry: &Entry) -> Result<Option<Self>> {
        match entry.get_secret() {
            Ok(bytes) => Ok(Some(
                serde_json::from_slice(&bytes).context("无法解析已保存的渠道设置")?,
            )),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error).context("无法读取系统凭据库"),
        }
    }

    pub fn save(self, entry: &Entry) -> Result<()> {
        let mut config = Self::default();
        config.fill_empty_from(self);
        for (label, url) in [
            ("Bark", &config.bark_server_url),
            ("OpeniLink", &config.openilink_hub_url),
            ("Hermes", &config.hermes_webhook_url),
        ] {
            if url.is_empty() {
                continue;
            }
            let valid = reqwest::Url::parse(url).ok().is_some_and(|url| {
                matches!(url.scheme(), "https" | "http") && url.host().is_some()
            });
            if !valid {
                bail!("{label} URL 必须是有效的 HTTP 或 HTTPS 地址");
            }
        }
        if config.bot_token.is_empty() != config.chat_id.is_empty() {
            bail!("Telegram 必须同时填写 Bot Token 和 Chat ID");
        }
        if config.openilink_hub_url.is_empty() != config.openilink_hub_token.is_empty() {
            bail!("OpeniLink 必须同时填写 Hub URL 和 Token");
        }
        entry
            .set_secret(&serde_json::to_vec(&config)?)
            .context("无法保存到系统凭据库")
    }

    pub fn import_legacy(entry: &Entry, path: &Path, mut config: Self) -> Result<()> {
        if Self::read_credential(entry)?.is_some() {
            bail!("已保存新设置，只能在首次配置时导入旧配置");
        }
        match fs::read(path) {
            Ok(bytes) => {
                config.fill_empty_from(serde_json::from_slice(&bytes).context("旧 JSON 格式无效")?)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if config.has_no_channels() {
                    bail!("未找到旧 JSON 或渠道环境变量，请手动填写设置");
                }
            }
            Err(error) => return Err(error).context("无法读取旧配置文件"),
        }
        config.save(entry)
    }

    pub fn legacy_environment() -> Self {
        Self {
            bot_token: env_value("TELEGRAM_BOT_TOKEN"),
            chat_id: env_value("TELEGRAM_CHAT_ID"),
            openilink_hub_url: env_value("OPENILINK_HUB_URL"),
            openilink_hub_token: env_value("OPENILINK_HUB_TOKEN"),
            hermes_webhook_url: env_value("HERMES_WEBHOOK_URL"),
            hermes_webhook_secret: env_value("HERMES_WEBHOOK_SECRET"),
            bark_server_url: env_value("BARK_SERVER_URL"),
            ignore_subagent_notifications: false,
        }
    }

    fn has_no_channels(&self) -> bool {
        [
            &self.bot_token,
            &self.chat_id,
            &self.openilink_hub_url,
            &self.openilink_hub_token,
            &self.hermes_webhook_url,
            &self.hermes_webhook_secret,
            &self.bark_server_url,
        ]
        .iter()
        .all(|value| value.is_empty())
    }

    fn fill_empty_from(&mut self, other: Self) {
        fill(&mut self.bot_token, other.bot_token);
        fill(&mut self.chat_id, other.chat_id);
        fill(&mut self.openilink_hub_url, other.openilink_hub_url);
        fill(&mut self.openilink_hub_token, other.openilink_hub_token);
        fill(&mut self.hermes_webhook_url, other.hermes_webhook_url);
        fill(&mut self.hermes_webhook_secret, other.hermes_webhook_secret);
        fill(&mut self.bark_server_url, other.bark_server_url);
        self.ignore_subagent_notifications = other.ignore_subagent_notifications;
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

pub fn legacy_config_path() -> Result<PathBuf> {
    let explicit = env_value("CODEX_NOTIFY_CONFIG");
    if !explicit.is_empty() {
        return Ok(PathBuf::from(explicit));
    }
    Ok(dirs::home_dir()
        .context("无法定位用户目录")?
        .join(".codex/codex-notify.json"))
}

fn plugin_data() -> Result<PathBuf> {
    if let Some(path) = env::var_os("PLUGIN_DATA").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(codex_home()?.join("plugins/data/codex-notify-codex-notify"))
}

fn codex_home() -> Result<PathBuf> {
    if let Some(home) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    Ok(dirs::home_dir().context("无法定位用户目录")?.join(".codex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_only_once_and_never_reloads_legacy_config() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("old.json");
        let entry = Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default()));
        fs::write(
            &file,
            r#"{"bark_server_url":"https://example.com/bark","ignore_subagent_notifications":true}"#,
        )?;
        assert!(Config::read_credential(&entry)?.is_none());
        let legacy_environment = Config {
            chat_id: " 123 ".into(),
            bot_token: " synthetic ".into(),
            ..Config::default()
        };
        Config::import_legacy(&entry, &file, legacy_environment)?;
        fs::write(&file, "deliberately invalid JSON")?;
        let saved = Config::read_credential(&entry)?.context("setting was not saved")?;
        assert_eq!(saved.bark_server_url, "https://example.com/bark");
        assert_eq!(saved.chat_id, "123");
        assert!(saved.ignore_subagent_notifications);
        assert!(Config::import_legacy(&entry, &file, Config::default()).is_err());
        assert!(Config {
            bark_server_url: "file:///not-a-webhook".into(),
            ..Config::default()
        }
        .save(&entry)
        .is_err());
        assert_eq!(
            Config::read_credential(&entry)?
                .context("setting was lost")?
                .chat_id,
            "123"
        );
        Ok(())
    }
}
