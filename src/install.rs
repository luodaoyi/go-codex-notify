use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use tempfile::NamedTempFile;
use toml_edit::{value, Array, DocumentMut};

#[derive(Clone, Debug)]
pub struct Paths {
    pub codex_home: PathBuf,
    pub bin_dir: PathBuf,
    pub installed_binary: PathBuf,
    pub config: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let codex_home = env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))
            .ok_or_else(|| anyhow!("无法确定 Codex 主目录；请设置 CODEX_HOME"))?;
        Ok(Self::from_codex_home(codex_home))
    }

    pub fn from_codex_home(codex_home: PathBuf) -> Self {
        let bin_dir = codex_home.join("bin");
        Self {
            installed_binary: bin_dir.join(binary_name()),
            config: codex_home.join("config.toml"),
            codex_home,
            bin_dir,
        }
    }
}

#[derive(Debug)]
pub struct ApplyResult {
    pub binary: PathBuf,
    pub config: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InstallationStatus {
    pub binary_installed: bool,
    pub hook_configured: bool,
}

pub fn apply() -> Result<ApplyResult> {
    let source = env::current_exe().context("无法定位当前 EXE")?;
    let paths = Paths::discover()?;
    apply_from(&source, &paths)
}

pub fn apply_from(source: &Path, paths: &Paths) -> Result<ApplyResult> {
    fs::create_dir_all(&paths.bin_dir)
        .with_context(|| format!("创建 Codex bin 目录失败：{}", paths.bin_dir.display()))?;
    copy_executable(source, &paths.installed_binary)?;
    set_notify_hook(&paths.config, &paths.installed_binary)?;
    Ok(ApplyResult {
        binary: paths.installed_binary.clone(),
        config: paths.config.clone(),
    })
}

pub fn status(paths: &Paths) -> Result<InstallationStatus> {
    Ok(InstallationStatus {
        binary_installed: paths.installed_binary.is_file(),
        hook_configured: configured_hook(&paths.config)?
            .is_some_and(|path| same_display_path(&path, &paths.installed_binary)),
    })
}

pub fn uninstall() -> Result<bool> {
    let paths = Paths::discover()?;
    remove_notify_hook(&paths.config, &paths.installed_binary)
}

fn copy_executable(source: &Path, destination: &Path) -> Result<()> {
    if source.canonicalize().ok().as_ref() == destination.canonicalize().ok().as_ref()
        && destination.exists()
    {
        return Ok(());
    }

    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("安装路径没有父目录：{}", destination.display()))?;
    fs::create_dir_all(parent)?;

    let mut source_file = fs::File::open(source)
        .with_context(|| format!("打开当前 EXE 失败：{}", source.display()))?;
    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("在 {} 创建临时文件失败", parent.display()))?;
    std::io::copy(&mut source_file, temporary.as_file_mut())
        .with_context(|| format!("复制 EXE 到 {} 失败", destination.display()))?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    make_executable(temporary.path())?;
    temporary
        .persist(destination)
        .map_err(|error| error.error)
        .with_context(|| format!("替换 EXE 失败：{}", destination.display()))?;
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn set_notify_hook(config_path: &Path, binary: &Path) -> Result<()> {
    edit_config(config_path, |document| {
        let mut command = Array::new();
        command.push(display_path(binary));
        document["notify"] = value(command);
        true
    })?;
    Ok(())
}

fn remove_notify_hook(config_path: &Path, binary: &Path) -> Result<bool> {
    edit_config(config_path, |document| {
        let is_ours = document
            .get("notify")
            .and_then(|item| item.as_array())
            .and_then(|array| array.iter().next())
            .and_then(|value| value.as_str())
            .is_some_and(|path| same_display_path(path, binary));
        if is_ours {
            document.remove("notify");
        }
        is_ours
    })
}

fn configured_hook(config_path: &Path) -> Result<Option<String>> {
    let source = match fs::read_to_string(config_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Codex 配置失败：{}", config_path.display()));
        }
    };
    let document = parse_document(&source, config_path)?;
    Ok(document
        .get("notify")
        .and_then(|item| item.as_array())
        .and_then(|array| array.iter().next())
        .and_then(|value| value.as_str())
        .map(str::to_owned))
}

fn edit_config(config_path: &Path, edit: impl FnOnce(&mut DocumentMut) -> bool) -> Result<bool> {
    let original = match fs::read(config_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Codex 配置失败：{}", config_path.display()));
        }
    };
    let text = String::from_utf8(original).with_context(|| {
        format!(
            "Codex 配置不是 UTF-8，无法安全修改：{}",
            config_path.display()
        )
    })?;
    let has_bom = text.starts_with('\u{feff}');
    let text_without_bom = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let crlf = text_without_bom.contains("\r\n");
    let mut document = parse_document(text_without_bom, config_path)?;
    if !edit(&mut document) {
        return Ok(false);
    }

    let mut rendered = document.to_string();
    if crlf {
        rendered = rendered.replace("\n", "\r\n");
    }
    if has_bom {
        rendered.insert(0, '\u{feff}');
    }
    atomic_write(config_path, rendered.as_bytes())?;
    Ok(true)
}

fn parse_document(source: &str, path: &Path) -> Result<DocumentMut> {
    source
        .parse::<DocumentMut>()
        .with_context(|| format!("解析 Codex 配置失败：{}", path.display()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("配置路径没有父目录：{}", path.display()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("在 {} 创建临时配置失败", parent.display()))?;
    temporary.write_all(bytes)?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("原子替换配置失败：{}", path.display()))?;
    Ok(())
}

fn same_display_path(configured: &str, expected: &Path) -> bool {
    if cfg!(windows) {
        configured.eq_ignore_ascii_case(&display_path(expected))
    } else {
        configured == display_path(expected)
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "codex-notify.exe"
    } else {
        "codex-notify"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_binary_and_preserves_toml_content() {
        let directory = tempfile::tempdir().unwrap();
        let codex_home = directory.path().join("codex");
        fs::create_dir_all(&codex_home).unwrap();
        let config = codex_home.join("config.toml");
        fs::write(
            &config,
            "# keep this comment\r\nmodel = \"gpt-test\"\r\n\r\n[features]\r\nweb_search = true\r\n",
        )
        .unwrap();
        let source = directory.path().join("source.exe");
        fs::write(&source, b"native-binary").unwrap();
        let paths = Paths::from_codex_home(codex_home);

        apply_from(&source, &paths).unwrap();

        assert_eq!(fs::read(&paths.installed_binary).unwrap(), b"native-binary");
        let updated = fs::read_to_string(&config).unwrap();
        assert!(updated.contains("# keep this comment\r\n"));
        assert!(updated.contains("model = \"gpt-test\"\r\n"));
        assert!(updated.contains("[features]\r\nweb_search = true\r\n"));
        let document = updated.parse::<DocumentMut>().unwrap();
        assert_eq!(
            document["notify"]
                .as_array()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .as_str(),
            Some(display_path(&paths.installed_binary).as_str())
        );
        assert_eq!(
            status(&paths).unwrap(),
            InstallationStatus {
                binary_installed: true,
                hook_configured: true,
            }
        );
    }

    #[test]
    fn uninstall_only_removes_our_hook() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::write(&paths.config, "notify = [\"other-tool\"]\n").unwrap();
        assert!(!remove_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(fs::read_to_string(&paths.config)
            .unwrap()
            .contains("other-tool"));

        set_notify_hook(&paths.config, &paths.installed_binary).unwrap();
        assert!(remove_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(configured_hook(&paths.config).unwrap().is_none());
    }
}
