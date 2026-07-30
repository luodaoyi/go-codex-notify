use std::{
    fs,
    io::{self, IsTerminal, Read},
};

use anyhow::{Context, Result};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Default)]
pub struct NotifyPayload {
    pub client: String,
    pub task: String,
    pub status: String,
    pub message: String,
    pub event: String,
    pub hook_event_name: String,
    pub session_id: String,
    pub turn_id: String,
    pub cwd: String,
    pub transcript_path: String,
    pub model: String,
    pub permission_mode: String,
    pub last_assistant_message: String,
    pub input_messages: Vec<String>,
    pub tool_name: String,
    pub tool_use_id: String,
    pub raw: Option<Map<String, Value>>,
}

pub fn read(args: &[String]) -> Result<(NotifyPayload, String)> {
    if let Some(value) = from_args(args) {
        return Ok(parse_bytes(value.as_bytes()));
    }

    if !io::stdin().is_terminal() {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .context("读取 stdin 通知 payload 失败")?;
        if bytes.iter().any(|byte| !byte.is_ascii_whitespace()) {
            return Ok(parse_bytes(&bytes));
        }
    }

    Ok((NotifyPayload::default(), String::new()))
}

pub fn from_args(args: &[String]) -> Option<String> {
    let first = args.first()?.trim();
    if first.is_empty() || first.starts_with('-') {
        return None;
    }
    if first.starts_with('{') || first.starts_with('[') || args.len() == 1 {
        return Some(args.join(" ").trim().to_owned());
    }
    None
}

pub fn parse_bytes(bytes: &[u8]) -> (NotifyPayload, String) {
    let raw_input = String::from_utf8_lossy(bytes).trim().to_owned();
    let Ok(Value::Object(raw)) = serde_json::from_slice::<Value>(bytes) else {
        return (
            NotifyPayload {
                message: raw_input.clone(),
                ..NotifyPayload::default()
            },
            raw_input,
        );
    };

    let payload = NotifyPayload {
        client: first_string(&raw, &["client"]),
        task: first_string(&raw, &["task", "title", "session", "thread", "objective"]),
        status: first_string(&raw, &["status", "state", "result"]),
        message: first_string(&raw, &["message", "summary", "text", "body", "reason"]),
        event: first_string(
            &raw,
            &[
                "event",
                "type",
                "kind",
                "hook_event_name",
                "hookEventName",
                "hook-event-name",
            ],
        ),
        hook_event_name: first_string(
            &raw,
            &["hook_event_name", "hookEventName", "hook-event-name"],
        ),
        session_id: first_string(
            &raw,
            &[
                "session_id",
                "sessionId",
                "session-id",
                "thread-id",
                "thread_id",
                "threadId",
            ],
        ),
        turn_id: first_string(&raw, &["turn_id", "turnId", "turn-id"]),
        cwd: first_string(&raw, &["cwd"]),
        transcript_path: first_string(
            &raw,
            &["transcript_path", "transcriptPath", "transcript-path"],
        ),
        model: first_string(&raw, &["model"]),
        permission_mode: first_string(
            &raw,
            &["permission_mode", "permissionMode", "permission-mode"],
        ),
        last_assistant_message: first_string(
            &raw,
            &[
                "last_assistant_message",
                "lastAssistantMessage",
                "last-assistant-message",
            ],
        ),
        input_messages: first_string_slice(
            &raw,
            &["input_messages", "inputMessages", "input-messages"],
        ),
        tool_name: first_string(&raw, &["tool_name", "toolName", "tool-name"]),
        tool_use_id: first_string(&raw, &["tool_use_id", "toolUseId", "tool-use-id"]),
        raw: Some(raw),
    };

    (payload, raw_input)
}

fn first_string(map: &Map<String, Value>, keys: &[&str]) -> String {
    for key in keys {
        let Some(value) = map.get(*key) else {
            continue;
        };
        let converted = match value {
            Value::String(value) => value.trim().to_owned(),
            Value::Number(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            _ => String::new(),
        };
        if !converted.is_empty() {
            return converted;
        }
    }
    String::new()
}

fn first_string_slice(map: &Map<String, Value>, keys: &[&str]) -> Vec<String> {
    for key in keys {
        let Some(value) = map.get(*key) else {
            continue;
        };
        let values = match value {
            Value::Array(values) => values.iter().filter_map(value_as_string).collect(),
            value => value_as_string(value).into_iter().collect(),
        };
        return clean_strings(values);
    }
    Vec::new()
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn clean_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

pub fn build_message(payload: &NotifyPayload) -> String {
    let mut lines = Vec::with_capacity(2);
    if !payload.input_messages.is_empty() {
        push_field(
            &mut lines,
            "用户输入",
            &format_input_messages(&payload.input_messages),
        );
    }
    let response = if payload.last_assistant_message.is_empty() {
        &payload.message
    } else {
        &payload.last_assistant_message
    };
    push_field(&mut lines, "Codex 回应", response);
    if lines.is_empty() {
        lines.push("Codex 回应：任务已完成".to_owned());
    }

    lines.join("\n")
}

fn push_field(lines: &mut Vec<String>, name: &str, value: &str) {
    if !value.is_empty() {
        lines.push(format!("{name}：{value}"));
    }
}

fn format_input_messages(messages: &[String]) -> String {
    const MAX_MESSAGES: usize = 3;
    const MAX_CHARS: usize = 500;

    let mut result = messages
        .iter()
        .take(MAX_MESSAGES)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" / ");
    if messages.len() > MAX_MESSAGES {
        result.push_str(&format!(" / ...(+{})", messages.len() - MAX_MESSAGES));
    }
    truncate_chars(&result, MAX_CHARS)
}

fn truncate_chars(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_owned();
    }
    let mut result: String = value.chars().take(limit).collect();
    result.push_str("...");
    result
}

pub fn notification_headline(payload: &NotifyPayload) -> String {
    let event_text = format!(
        "{} {} {} {}",
        payload.hook_event_name, payload.event, payload.status, payload.message
    )
    .to_lowercase();

    if payload.hook_event_name.eq_ignore_ascii_case("stop")
        || payload.event.eq_ignore_ascii_case("stop")
        || contains_any(
            &event_text,
            &[
                "turn-complete",
                "turn_complete",
                "completed",
                "finished",
                "done",
            ],
        )
    {
        return "父亲，Codex 任务已完成。".into();
    }
    if contains_any(
        &event_text,
        &["permission", "approval", "approve", "审批", "批准", "权限"],
    ) {
        return "父亲，Codex 等待你审批。".into();
    }
    if contains_any(
        &event_text,
        &[
            "input",
            "interaction",
            "question",
            "prompt",
            "waiting",
            "resume",
            "continue",
            "继续",
            "对话",
            "处理",
        ],
    ) {
        return "父亲，Codex 需要你继续处理。".into();
    }
    if payload.permission_mode.eq_ignore_ascii_case("plan") && payload.hook_event_name.is_empty() {
        return "父亲，Codex Plan Mode 需要你处理。".into();
    }
    if !payload.event.is_empty() || !payload.status.is_empty() || !payload.message.is_empty() {
        return "父亲，Codex 有新的通知。".into();
    }
    "父亲，Codex 任务已完成。".into()
}

pub fn notification_title(payload: &NotifyPayload) -> String {
    notification_headline(payload)
        .trim_start_matches("父亲，")
        .trim_end_matches('。')
        .to_owned()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

impl NotifyPayload {
    pub fn is_subagent(&self) -> bool {
        if self.hook_event_name.eq_ignore_ascii_case("SubagentStop")
            || self.event.eq_ignore_ascii_case("SubagentStop")
            || self.client.to_ascii_lowercase().contains("subagent")
        {
            return true;
        }

        if let Some(raw) = &self.raw {
            let has_subagent_identity = [
                "agent_id",
                "agentId",
                "agent-id",
                "agent_type",
                "agentType",
                "agent-type",
                "parent_thread_id",
                "parentThreadId",
                "parent-thread-id",
            ]
            .iter()
            .any(|key| raw.get(*key).is_some_and(value_is_present));
            let explicit_subagent_flag = ["is_subagent", "isSubagent", "is-subagent"]
                .iter()
                .any(|key| raw.get(*key).is_some_and(value_is_true));
            let subagent_source = first_string(raw, &["source", "thread_source", "threadSource"])
                .to_ascii_lowercase()
                .contains("subagent");
            if has_subagent_identity || explicit_subagent_flag || subagent_source {
                return true;
            }
        }

        self.event.eq_ignore_ascii_case("agent-turn-complete")
            && !self.session_id.is_empty()
            && self.client.is_empty()
            && self.input_messages.is_empty()
    }
}

fn value_is_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        _ => true,
    }
}

fn value_is_true(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::String(value) => value.eq_ignore_ascii_case("true") || value == "1",
        Value::Number(value) => value.as_i64().is_some_and(|value| value != 0),
        _ => false,
    }
}

pub fn enrich_user_input_from_transcript(payload: &mut NotifyPayload) {
    if !payload.input_messages.is_empty() {
        return;
    }
    let path = payload.transcript_path.trim();
    if path.is_empty() {
        return;
    }
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    for line in content
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(message) = transcript_user_message(&value) {
            payload.input_messages.push(message);
            break;
        }
    }
}

fn transcript_user_message(value: &Value) -> Option<String> {
    let raw = value.as_object()?;
    let entry = raw.get("payload").and_then(Value::as_object).unwrap_or(raw);
    let entry_type = first_string(entry, &["type"]);
    if entry_type == "user_message" {
        let message = first_string(entry, &["message", "text"]);
        return (!message.is_empty()).then_some(message);
    }
    if entry_type != "message" || !first_string(entry, &["role"]).eq_ignore_ascii_case("user") {
        return None;
    }

    let content = entry.get("content")?.as_array()?;
    let message = content
        .iter()
        .filter_map(Value::as_object)
        .filter(|part| first_string(part, &["type"]) == "input_text")
        .map(|part| first_string(part, &["text"]))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!message.is_empty()).then_some(message)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn parses_codex_notify_aliases() {
        let input = r#"{"type":"agent-turn-complete","thread-id":"thread-123","turn-id":"turn-456","cwd":"D:\\repo","input-messages":["修复 Bark 通知",2,true],"last-assistant-message":"已完成"}"#;
        let (payload, raw) = parse_bytes(input.as_bytes());
        assert!(!raw.is_empty());
        assert_eq!(payload.event, "agent-turn-complete");
        assert_eq!(payload.session_id, "thread-123");
        assert_eq!(payload.turn_id, "turn-456");
        assert_eq!(payload.input_messages, ["修复 Bark 通知", "2", "true"]);
    }

    #[test]
    fn invalid_json_becomes_plain_message() {
        let (payload, raw) = parse_bytes(b" plain text ");
        assert_eq!(payload.message, "plain text");
        assert_eq!(raw, "plain text");
    }

    #[test]
    fn builds_completion_and_attention_messages() {
        let complete = NotifyPayload {
            event: "agent-turn-complete".into(),
            session_id: "thread-123".into(),
            input_messages: vec!["修复 Bark 通知".into(), "确认 TUI".into()],
            last_assistant_message: "已完成".into(),
            ..NotifyPayload::default()
        };
        let message = build_message(&complete);
        assert_eq!(
            message,
            "用户输入：修复 Bark 通知 / 确认 TUI\nCodex 回应：已完成"
        );
        assert!(!message.contains("thread-123"));

        let attention = NotifyPayload {
            event: "user-interaction-required".into(),
            message: "Plan Mode 等待用户输入".into(),
            ..NotifyPayload::default()
        };
        assert_eq!(notification_title(&attention), "Codex 需要你继续处理");
    }

    #[test]
    fn reads_latest_user_input_from_codex_transcript() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"旧问题"}}}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"修复通知正文"}}]}}}}"#).unwrap();

        let mut payload = NotifyPayload {
            transcript_path: file.path().to_string_lossy().into_owned(),
            last_assistant_message: "已只保留可读内容".into(),
            ..NotifyPayload::default()
        };
        enrich_user_input_from_transcript(&mut payload);

        assert_eq!(payload.input_messages, ["修复通知正文"]);
        assert_eq!(
            build_message(&payload),
            "用户输入：修复通知正文\nCodex 回应：已只保留可读内容"
        );
    }

    #[test]
    fn distinguishes_current_codex_subagent_notifications() {
        let (child, _) = parse_bytes(
            br#"{"type":"agent-turn-complete","thread-id":"child-thread","turn-id":"child-turn","cwd":"D:\\repo","input-messages":[],"last-assistant-message":"CHILD_DONE"}"#,
        );
        let (parent, _) = parse_bytes(
            br#"{"type":"agent-turn-complete","thread-id":"parent-thread","turn-id":"parent-turn","cwd":"D:\\repo","client":"codex_exec","input-messages":["do work"],"last-assistant-message":"PARENT_DONE"}"#,
        );

        assert!(child.is_subagent());
        assert!(!parent.is_subagent());
    }

    #[test]
    fn recognizes_explicit_subagent_markers() {
        let (payload, _) = parse_bytes(
            br#"{"type":"agent-turn-complete","thread-id":"child","agent_id":"agent-1","input-messages":["delegated task"]}"#,
        );
        assert!(payload.is_subagent());
    }
}
