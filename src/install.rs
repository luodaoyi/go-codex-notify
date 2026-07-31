use std::{
    env, fs,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Map, Value};
use tempfile::NamedTempFile;
use toml_edit::{Array, DocumentMut};

use crate::config::Config;

const STOP_EVENT: &str = "Stop";
const SUBAGENT_STOP_EVENT: &str = "SubagentStop";

#[derive(Clone, Debug)]
pub struct Paths {
    pub codex_home: PathBuf,
    pub bin_dir: PathBuf,
    pub installed_binary: PathBuf,
    pub config: PathBuf,
    pub hooks: PathBuf,
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
            hooks: codex_home.join("hooks.json"),
            codex_home,
            bin_dir,
        }
    }
}

#[derive(Debug)]
pub struct ApplyResult {
    pub binary: PathBuf,
    pub config: PathBuf,
    pub hooks: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InstallationStatus {
    pub binary_installed: bool,
    pub binary_current: bool,
    pub hook_configured: bool,
    pub subagent_hook_configured: bool,
    pub legacy_notify_configured: bool,
}

pub fn apply() -> Result<ApplyResult> {
    let source = env::current_exe().context("无法定位当前 EXE")?;
    let paths = Paths::discover()?;
    let config = Config::load()?;
    apply_from(&source, &paths, !config.ignore_subagent_notifications)
}

pub fn apply_from(
    source: &Path,
    paths: &Paths,
    include_subagent_notifications: bool,
) -> Result<ApplyResult> {
    fs::create_dir_all(&paths.bin_dir)
        .with_context(|| format!("创建 Codex bin 目录失败：{}", paths.bin_dir.display()))?;
    copy_executable(source, &paths.installed_binary)?;
    set_lifecycle_hooks(
        &paths.hooks,
        &paths.installed_binary,
        include_subagent_notifications,
    )?;
    remove_notify_hook(&paths.config, &paths.installed_binary)?;
    Ok(ApplyResult {
        binary: paths.installed_binary.clone(),
        config: paths.config.clone(),
        hooks: paths.hooks.clone(),
    })
}

pub fn status(paths: &Paths) -> Result<InstallationStatus> {
    let binary_installed = paths.installed_binary.is_file();
    let binary_current = env::current_exe()
        .ok()
        .filter(|source| binary_installed && source.is_file())
        .map(|source| same_file_contents(&source, &paths.installed_binary))
        .transpose()?
        .unwrap_or(false);

    Ok(InstallationStatus {
        binary_installed,
        binary_current,
        hook_configured: configured_lifecycle_hook(
            &paths.hooks,
            STOP_EVENT,
            &paths.installed_binary,
        )?,
        subagent_hook_configured: configured_lifecycle_hook(
            &paths.hooks,
            SUBAGENT_STOP_EVENT,
            &paths.installed_binary,
        )?,
        legacy_notify_configured: configured_notify_hook(&paths.config, &paths.installed_binary)?,
    })
}

pub fn uninstall() -> Result<bool> {
    let paths = Paths::discover()?;
    let hooks_changed = remove_lifecycle_hooks(&paths.hooks, &paths.installed_binary)?;
    let notify_changed = remove_notify_hook(&paths.config, &paths.installed_binary)?;
    Ok(hooks_changed || notify_changed)
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

fn same_file_contents(left: &Path, right: &Path) -> Result<bool> {
    if let (Ok(left_path), Ok(right_path)) = (left.canonicalize(), right.canonicalize()) {
        if left_path == right_path {
            return Ok(true);
        }
    }
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }

    let mut left = BufReader::new(fs::File::open(left)?);
    let mut right = BufReader::new(fs::File::open(right)?);
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_read = left.read(&mut left_buffer)?;
        let right_read = right.read(&mut right_buffer)?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn set_lifecycle_hooks(
    hooks_path: &Path,
    binary: &Path,
    include_subagent_notifications: bool,
) -> Result<()> {
    edit_hooks(hooks_path, |root| {
        let hooks = hooks_object(root)?;
        let stop_changed = update_event(hooks, STOP_EVENT, binary, true)?;
        let subagent_changed = update_event(
            hooks,
            SUBAGENT_STOP_EVENT,
            binary,
            include_subagent_notifications,
        )?;
        Ok(stop_changed || subagent_changed)
    })?;
    Ok(())
}

fn remove_lifecycle_hooks(hooks_path: &Path, binary: &Path) -> Result<bool> {
    edit_hooks(hooks_path, |root| {
        let Some(Value::Object(hooks)) = root.get_mut("hooks") else {
            return Ok(false);
        };
        let stop_changed = update_event(hooks, STOP_EVENT, binary, false)?;
        let subagent_changed = update_event(hooks, SUBAGENT_STOP_EVENT, binary, false)?;
        if hooks.is_empty() {
            root.remove("hooks");
        }
        Ok(stop_changed || subagent_changed)
    })
}

fn hooks_object(root: &mut Map<String, Value>) -> Result<&mut Map<String, Value>> {
    let hooks = root
        .entry("hooks".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    hooks
        .as_object_mut()
        .ok_or_else(|| anyhow!("Codex hooks.json 的 hooks 字段必须是对象"))
}

fn update_event(
    hooks: &mut Map<String, Value>,
    event: &str,
    binary: &Path,
    enabled: bool,
) -> Result<bool> {
    let canonical_handler = hook_handler(binary);
    let mut found_canonical = false;
    let mut changed = false;

    if let Some(groups_value) = hooks.get_mut(event) {
        let groups = groups_value
            .as_array_mut()
            .ok_or_else(|| anyhow!("Codex hooks.json 的 {event} 字段必须是数组"))?;
        groups.retain_mut(|group| {
            let Some(group_object) = group.as_object_mut() else {
                return true;
            };
            let Some(handlers) = group_object.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };

            handlers.retain(|handler| {
                if !is_our_handler(handler, binary) {
                    return true;
                }
                if enabled && !found_canonical && handler == &canonical_handler {
                    found_canonical = true;
                    true
                } else {
                    changed = true;
                    false
                }
            });
            if handlers.is_empty() {
                changed = true;
                false
            } else {
                true
            }
        });
    }

    if enabled && !found_canonical {
        let groups = hooks
            .entry(event.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| anyhow!("Codex hooks.json 的 {event} 字段必须是数组"))?;
        groups.push(json!({ "hooks": [canonical_handler] }));
        changed = true;
    } else if !enabled
        && hooks
            .get(event)
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        hooks.remove(event);
        changed = true;
    }

    Ok(changed)
}

fn hook_handler(binary: &Path) -> Value {
    let command = hook_command(binary);
    let handler = Map::from_iter([
        ("type".to_owned(), Value::String("command".to_owned())),
        ("command".to_owned(), Value::String(command)),
        ("timeout".to_owned(), Value::from(30)),
        (
            "statusMessage".to_owned(),
            Value::String("Sending codex-notify".to_owned()),
        ),
        #[cfg(windows)]
        (
            "commandWindows".to_owned(),
            Value::String(hook_command_windows(binary)),
        ),
    ]);
    Value::Object(handler)
}

fn is_our_handler(handler: &Value, binary: &Path) -> bool {
    let Some(handler) = handler.as_object() else {
        return false;
    };
    if handler.get("type").and_then(Value::as_str) != Some("command") {
        return false;
    }
    handler
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| same_command(command, &hook_command(binary)))
        || handler
            .get("commandWindows")
            .and_then(Value::as_str)
            .is_some_and(|command| same_command(command, &hook_command_windows(binary)))
}

fn hook_command(binary: &Path) -> String {
    if cfg!(windows) {
        format!("\"{}\" hook", display_path(binary))
    } else {
        format!("{} hook", shell_quote(&display_path(binary)))
    }
}

fn hook_command_windows(binary: &Path) -> String {
    let script = format!("& '{}' hook", display_path(binary).replace('\'', "''"));
    let utf16le = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    format!(
        "powershell.exe -NoProfile -NonInteractive -EncodedCommand {}",
        BASE64.encode(utf16le)
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn same_command(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn configured_lifecycle_hook(hooks_path: &Path, event: &str, binary: &Path) -> Result<bool> {
    let Some(root) = read_hooks(hooks_path)? else {
        return Ok(false);
    };
    Ok(root
        .get("hooks")
        .and_then(Value::as_object)
        .and_then(|hooks| hooks.get(event))
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|group| {
                group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .is_some_and(|handlers| {
                        handlers
                            .iter()
                            .any(|handler| is_our_handler(handler, binary))
                    })
            })
        }))
}

fn read_hooks(path: &Path) -> Result<Option<Map<String, Value>>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Codex hooks 失败：{}", path.display()));
        }
    };
    let text = String::from_utf8(bytes)
        .with_context(|| format!("Codex hooks 不是 UTF-8：{}", path.display()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    if text.trim().is_empty() {
        return Ok(Some(Map::new()));
    }
    let value: Value = serde_json::from_str(text)
        .with_context(|| format!("解析 Codex hooks 失败：{}", path.display()))?;
    value
        .as_object()
        .cloned()
        .map(Some)
        .ok_or_else(|| anyhow!("Codex hooks.json 顶层必须是对象：{}", path.display()))
}

fn edit_hooks(
    hooks_path: &Path,
    edit: impl FnOnce(&mut Map<String, Value>) -> Result<bool>,
) -> Result<bool> {
    let original = match fs::read(hooks_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Codex hooks 失败：{}", hooks_path.display()));
        }
    };
    let text = String::from_utf8(original)
        .with_context(|| format!("Codex hooks 不是 UTF-8：{}", hooks_path.display()))?;
    let has_bom = text.starts_with('\u{feff}');
    let text_without_bom = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let crlf = text_without_bom.contains("\r\n");
    let mut root = if text_without_bom.trim().is_empty() {
        Map::new()
    } else {
        let value: Value = serde_json::from_str(text_without_bom)
            .with_context(|| format!("解析 Codex hooks 失败：{}", hooks_path.display()))?;
        value
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow!("Codex hooks.json 顶层必须是对象：{}", hooks_path.display()))?
    };
    if !edit(&mut root)? {
        return Ok(false);
    }

    let mut rendered = serde_json::to_string_pretty(&Value::Object(root))?;
    rendered.push('\n');
    if crlf {
        rendered = rendered.replace('\n', "\r\n");
    }
    if has_bom {
        rendered.insert(0, '\u{feff}');
    }
    atomic_write(hooks_path, rendered.as_bytes())?;
    Ok(true)
}

fn remove_notify_hook(config_path: &Path, binary: &Path) -> Result<bool> {
    edit_config(config_path, |document| {
        let is_ours = document
            .get("notify")
            .and_then(|item| item.as_array())
            .is_some_and(|array| is_our_notify(array, binary));
        if is_ours {
            document.remove("notify");
        }
        is_ours
    })
}

fn configured_notify_hook(config_path: &Path, binary: &Path) -> Result<bool> {
    let source = match fs::read_to_string(config_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Codex 配置失败：{}", config_path.display()));
        }
    };
    let document = parse_document(
        source.strip_prefix('\u{feff}').unwrap_or(&source),
        config_path,
    )?;
    Ok(document
        .get("notify")
        .and_then(|item| item.as_array())
        .is_some_and(|array| is_our_notify(array, binary)))
}

fn is_our_notify(arguments: &Array, binary: &Path) -> bool {
    let Some(command) = arguments.iter().next().and_then(|value| value.as_str()) else {
        return false;
    };
    if same_display_path(command, binary)
        || same_display_path(command, &binary.with_file_name(legacy_binary_name()))
    {
        return true;
    }
    is_npx_command(command)
        && arguments
            .iter()
            .skip(1)
            .filter_map(|value| value.as_str())
            .any(is_our_package_spec)
}

fn is_npx_command(command: &str) -> bool {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("npx")
                || name.eq_ignore_ascii_case("npx.cmd")
                || name.eq_ignore_ascii_case("npx.exe")
        })
}

fn is_our_package_spec(specification: &str) -> bool {
    specification == "go-codex-notify"
        || specification.starts_with("go-codex-notify@")
        || specification == "@asural/codex-notify"
        || specification.starts_with("@asural/codex-notify@")
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
        rendered = rendered.replace('\n', "\r\n");
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

fn legacy_binary_name() -> &'static str {
    if cfg!(windows) {
        "go-codex-notify.exe"
    } else {
        "go-codex-notify"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    use std::process::Command;

    fn event_handlers<'a>(root: &'a Value, event: &str) -> Vec<&'a Value> {
        root["hooks"][event]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|group| group.get("hooks").and_then(Value::as_array))
            .flatten()
            .collect()
    }

    #[test]
    fn installs_binary_migrates_notify_and_configures_main_only_hook() {
        let directory = tempfile::tempdir().unwrap();
        let codex_home = directory.path().join("codex");
        fs::create_dir_all(&codex_home).unwrap();
        let source = directory.path().join("source.exe");
        fs::write(&source, b"native-binary").unwrap();
        let paths = Paths::from_codex_home(codex_home);
        fs::write(
            &paths.config,
            format!(
                "# keep this comment\r\nmodel = \"gpt-test\"\r\nnotify = [\"{}\"]\r\n\r\n[features]\r\nweb_search = true\r\n",
                display_path(&paths.installed_binary).replace('\\', "\\\\")
            ),
        )
        .unwrap();

        apply_from(&source, &paths, false).unwrap();

        assert_eq!(fs::read(&paths.installed_binary).unwrap(), b"native-binary");
        let updated = fs::read_to_string(&paths.config).unwrap();
        assert!(updated.contains("# keep this comment\r\n"));
        assert!(updated.contains("[features]\r\nweb_search = true\r\n"));
        assert!(!updated.contains("notify ="));
        let hooks: Value = serde_json::from_slice(&fs::read(&paths.hooks).unwrap()).unwrap();
        let stop_handlers = event_handlers(&hooks, STOP_EVENT);
        assert_eq!(stop_handlers.len(), 1);
        assert_eq!(
            stop_handlers[0]["command"],
            hook_command(&paths.installed_binary)
        );
        #[cfg(windows)]
        assert_eq!(
            stop_handlers[0]["commandWindows"],
            hook_command_windows(&paths.installed_binary)
        );
        assert!(event_handlers(&hooks, SUBAGENT_STOP_EVENT).is_empty());
        assert_eq!(
            status(&paths).unwrap(),
            InstallationStatus {
                binary_installed: true,
                binary_current: false,
                hook_configured: true,
                subagent_hook_configured: false,
                legacy_notify_configured: false,
            }
        );
    }

    #[test]
    fn configures_all_agents_and_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        let source = directory.path().join("source.exe");
        fs::write(&source, b"native-binary").unwrap();

        apply_from(&source, &paths, true).unwrap();
        let first = fs::read(&paths.hooks).unwrap();
        apply_from(&source, &paths, true).unwrap();
        let second = fs::read(&paths.hooks).unwrap();

        assert_eq!(first, second);
        let hooks: Value = serde_json::from_slice(&second).unwrap();
        assert_eq!(event_handlers(&hooks, STOP_EVENT).len(), 1);
        assert_eq!(event_handlers(&hooks, SUBAGENT_STOP_EVENT).len(), 1);
    }

    #[test]
    fn preserves_unrelated_hooks_and_uninstall_removes_only_ours() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::write(
            &paths.hooks,
            r#"{
  "description": "keep me",
  "hooks": {
    "Stop": [{"hooks": [{"type": "command", "command": "other-tool"}]}],
    "SessionEnd": [{"hooks": [{"type": "command", "command": "cleanup"}]}]
  }
}
"#,
        )
        .unwrap();
        set_lifecycle_hooks(&paths.hooks, &paths.installed_binary, true).unwrap();

        assert!(remove_lifecycle_hooks(&paths.hooks, &paths.installed_binary).unwrap());
        let hooks: Value = serde_json::from_slice(&fs::read(&paths.hooks).unwrap()).unwrap();
        assert_eq!(hooks["description"], "keep me");
        assert_eq!(event_handlers(&hooks, STOP_EVENT).len(), 1);
        assert_eq!(
            event_handlers(&hooks, STOP_EVENT)[0]["command"],
            "other-tool"
        );
        assert_eq!(event_handlers(&hooks, "SessionEnd").len(), 1);
        assert!(!remove_lifecycle_hooks(&paths.hooks, &paths.installed_binary).unwrap());
    }

    #[test]
    fn switching_to_main_only_removes_only_our_subagent_hook() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        set_lifecycle_hooks(&paths.hooks, &paths.installed_binary, true).unwrap();
        let mut hooks: Value = serde_json::from_slice(&fs::read(&paths.hooks).unwrap()).unwrap();
        hooks["hooks"][SUBAGENT_STOP_EVENT]
            .as_array_mut()
            .unwrap()
            .push(json!({"hooks": [{"type": "command", "command": "other-subagent-hook"}]}));
        fs::write(
            &paths.hooks,
            format!("{}\n", serde_json::to_string_pretty(&hooks).unwrap()),
        )
        .unwrap();

        set_lifecycle_hooks(&paths.hooks, &paths.installed_binary, false).unwrap();
        let hooks: Value = serde_json::from_slice(&fs::read(&paths.hooks).unwrap()).unwrap();
        let handlers = event_handlers(&hooks, SUBAGENT_STOP_EVENT);
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0]["command"], "other-subagent-hook");
    }

    #[test]
    fn preserves_json_bom_and_crlf() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::write(
            &paths.hooks,
            b"\xef\xbb\xbf{\r\n  \"custom\": true\r\n}\r\n",
        )
        .unwrap();

        set_lifecycle_hooks(&paths.hooks, &paths.installed_binary, false).unwrap();
        let bytes = fs::read(&paths.hooks).unwrap();
        assert!(bytes.starts_with(b"\xef\xbb\xbf"));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\r\n"));
        assert!(!text.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn preserves_other_notify_during_migration() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::write(
            &paths.config,
            "notify = [\"npx\", \"-y\", \"other-tool@latest\"]\n",
        )
        .unwrap();

        assert!(!remove_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(fs::read_to_string(&paths.config)
            .unwrap()
            .contains("other-tool@latest"));
    }

    #[test]
    fn migrates_legacy_go_package_notify_commands() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();

        fs::write(
            &paths.config,
            "notify = [\"npx\", \"-y\", \"go-codex-notify@latest\"]\n",
        )
        .unwrap();
        assert!(configured_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(remove_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(!fs::read_to_string(&paths.config)
            .unwrap()
            .contains("notify"));

        let legacy_binary = paths.installed_binary.with_file_name(legacy_binary_name());
        fs::write(
            &paths.config,
            format!("notify = ['{}']\n", display_path(&legacy_binary)),
        )
        .unwrap();
        assert!(configured_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(remove_notify_hook(&paths.config, &paths.installed_binary).unwrap());
        assert!(!fs::read_to_string(&paths.config)
            .unwrap()
            .contains("notify"));
    }

    #[test]
    fn rejects_malformed_event_without_overwriting_file() {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::from_codex_home(directory.path().join("codex"));
        fs::create_dir_all(&paths.codex_home).unwrap();
        let original = b"{\n  \"hooks\": {\n    \"Stop\": {}\n  }\n}\n";
        fs::write(&paths.hooks, original).unwrap();

        let error = set_lifecycle_hooks(&paths.hooks, &paths.installed_binary, false).unwrap_err();

        assert!(error.to_string().contains("Stop"));
        assert_eq!(fs::read(&paths.hooks).unwrap(), original);
    }

    #[cfg(windows)]
    #[test]
    fn windows_hook_command_runs_from_powershell_and_cmd() {
        let directory = tempfile::tempdir().unwrap();
        let hook_dir = directory.path().join("hook with spaces");
        fs::create_dir_all(&hook_dir).unwrap();
        let hook = hook_dir.join("notify hook.cmd");
        let marker = hook_dir.join("called.txt");
        fs::write(
            &hook,
            "@echo off\r\nif not \"%~1\"==\"hook\" exit /B 7\r\necho called>\"%~dp0called.txt\"\r\n",
        )
        .unwrap();
        let command = hook_command_windows(&hook);

        let powershell_status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .status()
            .unwrap();
        assert!(powershell_status.success());
        assert!(marker.is_file());
        fs::remove_file(&marker).unwrap();

        let cmd_status = Command::new("cmd.exe")
            .args(["/D", "/S", "/C", &command])
            .status()
            .unwrap();
        assert!(cmd_status.success());
        assert!(marker.is_file());
    }
}
