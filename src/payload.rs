use std::{fs, io};

use anyhow::{Context, Result};
use pulldown_cmark::{Event, Parser, Tag};
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
}

pub fn read() -> Result<NotifyPayload> {
    let raw = serde_json::from_reader(io::stdin().lock()).context("读取 Codex Hook JSON 失败")?;
    Ok(parse(&raw))
}

fn parse(raw: &Map<String, Value>) -> NotifyPayload {
    NotifyPayload {
        client: first_string(raw, &["client"]),
        task: first_string(raw, &["task", "title", "session", "thread", "objective"]),
        status: first_string(raw, &["status", "state", "result"]),
        message: first_string(raw, &["message", "summary", "text", "body", "reason"]),
        event: first_string(
            raw,
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
            raw,
            &["hook_event_name", "hookEventName", "hook-event-name"],
        ),
        session_id: first_string(
            raw,
            &[
                "session_id",
                "sessionId",
                "session-id",
                "thread-id",
                "thread_id",
                "threadId",
            ],
        ),
        turn_id: first_string(raw, &["turn_id", "turnId", "turn-id"]),
        cwd: first_string(raw, &["cwd"]),
        transcript_path: first_string(
            raw,
            &["transcript_path", "transcriptPath", "transcript-path"],
        ),
        model: first_string(raw, &["model"]),
        permission_mode: first_string(
            raw,
            &["permission_mode", "permissionMode", "permission-mode"],
        ),
        last_assistant_message: first_string(
            raw,
            &[
                "last_assistant_message",
                "lastAssistantMessage",
                "last-assistant-message",
            ],
        ),
        input_messages: first_string_slice(
            raw,
            &["input_messages", "inputMessages", "input-messages"],
        ),
        tool_name: first_string(raw, &["tool_name", "toolName", "tool-name"]),
        tool_use_id: first_string(raw, &["tool_use_id", "toolUseId", "tool-use-id"]),
    }
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
    let (user_input, response) = message_parts(payload);
    let mut lines = Vec::with_capacity(3);
    if payload.is_turn_update() && (!user_input.is_empty() || !response.is_empty()) {
        lines.push(notification_title(payload));
    }
    push_field(&mut lines, "用户输入", &user_input);
    push_field(&mut lines, "Codex 回应", &response);
    lines.join("\n")
}

pub fn build_bark_markdown(payload: &NotifyPayload) -> String {
    let (user_input, response) = message_parts(payload);
    let mut sections = Vec::with_capacity(2);
    if !user_input.is_empty() {
        sections.push(format!("**用户输入**\n\n{user_input}"));
    }
    if !response.is_empty() {
        sections.push(format!("**Codex 回应**\n\n{response}"));
    }
    sections.join("\n\n---\n\n")
}

pub fn build_telegram_markdown_v2(payload: &NotifyPayload) -> String {
    let (user_input, response) = message_parts(payload);
    let mut sections = Vec::with_capacity(3);
    if payload.is_turn_update() && (!user_input.is_empty() || !response.is_empty()) {
        sections.push(format!("*{}*", notification_title(payload)));
    }
    if !user_input.is_empty() {
        sections.push(format!(
            "*用户输入*\n{}",
            crate::telegram_markdown::render(&user_input)
        ));
    }
    if !response.is_empty() {
        sections.push(format!(
            "*Codex 回应*\n{}",
            crate::telegram_markdown::render(&response)
        ));
    }
    sections.join("\n\n")
}

fn message_parts(payload: &NotifyPayload) -> (String, String) {
    let user_input = format_input_messages(&payload.input_messages);
    let mut response = visible_response(if payload.last_assistant_message.is_empty() {
        &payload.message
    } else {
        &payload.last_assistant_message
    });
    if user_input.is_empty() && response.is_empty() && !payload.is_turn_update() {
        response = "任务已完成".to_owned();
    }
    (user_input, response)
}

fn visible_response(text: &str) -> String {
    if !text.contains(":codex-annotation{") {
        return text.trim().to_owned();
    }

    let mut code_blocks = Parser::new(text)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(event, Event::Start(Tag::CodeBlock(_))).then_some(range)
        })
        .peekable();
    let mut result = String::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        while code_blocks.peek().is_some_and(|range| range.end <= offset) {
            code_blocks.next();
        }
        let in_code = code_blocks
            .peek()
            .is_some_and(|range| range.start < offset + line.len());
        let annotation = line
            .trim()
            .strip_prefix(":codex-annotation{")
            .and_then(|attributes| attributes.strip_suffix('}'))
            .is_some_and(|attributes| {
                attributes.starts_with("index=") && attributes.contains(" id=")
            });
        if in_code || !annotation {
            result.push_str(line);
        }
        offset += line.len();
    }
    result.trim().to_owned()
}

fn is_visible_user_text(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.starts_with("<send_user_message_question_reply>")
        && !text.starts_with("<goal_context>")
        && !text.starts_with("<codex_internal_context ")
}

fn push_field(lines: &mut Vec<String>, name: &str, value: &str) {
    if !value.is_empty() {
        lines.push(format!("{name}：{value}"));
    }
}

fn format_input_messages(messages: &[String]) -> String {
    const MAX_MESSAGES: usize = 3;
    const MAX_CHARS: usize = 500;

    let visible: Vec<_> = messages
        .iter()
        .map(String::as_str)
        .filter(|text| is_visible_user_text(text))
        .collect();
    let mut result = visible
        .iter()
        .take(MAX_MESSAGES)
        .copied()
        .collect::<Vec<_>>()
        .join(" / ");
    if visible.len() > MAX_MESSAGES {
        result.push_str(&format!(" / ...(+{})", visible.len() - MAX_MESSAGES));
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

    if payload.is_turn_update() {
        return "父亲，Codex 本轮更新。".into();
    }
    if contains_any(
        &event_text,
        &[
            "turn-complete",
            "turn_complete",
            "completed",
            "finished",
            "done",
        ],
    ) {
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
    fn is_turn_update(&self) -> bool {
        ["Stop", "SubagentStop"]
            .iter()
            .any(|event| self.hook_event_name.eq_ignore_ascii_case(event))
            || [
                "Stop",
                "SubagentStop",
                "agent-turn-complete",
                "turn-complete",
                "turn_complete",
            ]
            .iter()
            .any(|event| self.event.eq_ignore_ascii_case(event))
    }
}

pub fn enrich_user_input_from_transcript(payload: &mut NotifyPayload) {
    payload
        .input_messages
        .retain(|message| is_visible_user_text(message));
    if !payload.input_messages.is_empty() || payload.turn_id.is_empty() {
        return;
    }
    let Ok(content) = fs::read_to_string(payload.transcript_path.trim()) else {
        return;
    };
    let mut in_turn = false;
    let mut user_event = None;
    let mut user_item = None;

    for line in content.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(entry) = value.get("payload").and_then(Value::as_object) else {
            continue;
        };
        let kind = entry.get("type").and_then(Value::as_str);
        if matches!(kind, Some("task_started" | "turn_started")) {
            if in_turn {
                break;
            }
            in_turn =
                entry.get("turn_id").and_then(Value::as_str) == Some(payload.turn_id.as_str());
        } else if in_turn {
            if matches!(kind, Some("task_complete" | "turn_complete")) {
                break;
            }
            if let Some(message) = transcript_user_message(entry) {
                if kind == Some("user_message") {
                    user_event = Some(message);
                } else {
                    user_item = Some(message);
                }
            }
        }
    }
    payload.input_messages.extend(user_event.or(user_item));
}

fn transcript_user_message(entry: &Map<String, Value>) -> Option<String> {
    let entry_type = first_string(entry, &["type"]);
    if entry_type == "user_message" {
        let message = first_string(entry, &["message", "text"]);
        return is_visible_user_text(&message).then_some(message);
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
        .filter(|text| is_visible_user_text(text))
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
    fn parses_stop_hook_payload() -> Result<()> {
        let input = r#"{"hook_event_name":"Stop","session_id":"thread-123","turn_id":"turn-456","cwd":"D:\\repo","last_assistant_message":"已完成"}"#;
        let payload = parse(&serde_json::from_str(input)?);
        assert_eq!(payload.hook_event_name, "Stop");
        assert_eq!(payload.session_id, "thread-123");
        assert_eq!(payload.turn_id, "turn-456");
        assert_eq!(payload.last_assistant_message, "已完成");
        Ok(())
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
            "Codex 本轮更新\n用户输入：修复 Bark 通知 / 确认 TUI\nCodex 回应：已完成"
        );
        assert!(!message.contains("thread-123"));
        assert_eq!(
            build_bark_markdown(&complete),
            "**用户输入**\n\n修复 Bark 通知 / 确认 TUI\n\n---\n\n**Codex 回应**\n\n已完成"
        );

        let attention = NotifyPayload {
            event: "user-interaction-required".into(),
            message: "Plan Mode 等待用户输入".into(),
            ..NotifyPayload::default()
        };
        assert_eq!(notification_title(&attention), "Codex 需要你继续处理");
    }

    #[test]
    fn converts_commonmark_for_telegram_markdown_v2() {
        let payload = NotifyPayload {
            input_messages: vec!["修复 **Markdown** [链接](https://example.com)!".into()],
            last_assistant_message: "### 结果\n\n- 已完成_v2.\n- 使用 `C:\\tmp`".into(),
            ..NotifyPayload::default()
        };

        assert_eq!(
            build_telegram_markdown_v2(&payload),
            "*用户输入*\n修复 *Markdown* [链接](https://example.com)\\!\n\n*Codex 回应*\n*结果*\n\n• 已完成\\_v2\\.\n• 使用 `C:\\\\tmp`"
        );
    }

    #[test]
    fn reads_latest_user_input_from_codex_transcript() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        for entry in [
            serde_json::json!({"type":"user_message","message":"旧问题"}),
            serde_json::json!({"type":"task_started","turn_id":"current"}),
            serde_json::json!({"type":"user_message","message":"修复通知正文"}),
            serde_json::json!({"type":"message","role":"user","content":[{"type":"input_text","text":"<send_user_message_question_reply>synthetic answer</send_user_message_question_reply>"}]}),
            serde_json::json!({"type":"task_started","turn_id":"next"}),
            serde_json::json!({"type":"user_message","message":"下一轮"}),
        ] {
            writeln!(file, "{}", serde_json::json!({"payload":entry}))?;
        }
        let mut payload = NotifyPayload {
            hook_event_name: "Stop".into(),
            turn_id: "current".into(),
            transcript_path: file.path().to_string_lossy().into_owned(),
            last_assistant_message: ":codex-annotation{index=\"1\" id=\"ui\"}\n\n已只保留可读内容"
                .into(),
            ..NotifyPayload::default()
        };
        enrich_user_input_from_transcript(&mut payload);
        assert_eq!(payload.input_messages, ["修复通知正文"]);
        assert_eq!(
            build_message(&payload),
            "Codex 本轮更新\n用户输入：修复通知正文\nCodex 回应：已只保留可读内容"
        );
        payload.turn_id = "missing-turn".into();
        payload.input_messages = vec![
            "<send_user_message_question_reply>synthetic answer</send_user_message_question_reply>".into(),
            "<codex_internal_context source=\"goal\">internal continuation</codex_internal_context>".into(),
        ];
        payload.last_assistant_message = concat!(
            ":codex-annotation{index=\"1\" id=\"ui\"}\n\n",
            "```json\n{\"status\":\"active\"}\n```",
        )
        .into();
        enrich_user_input_from_transcript(&mut payload);
        assert!(payload.input_messages.is_empty());
        assert_eq!(
            build_message(&payload),
            "Codex 本轮更新\nCodex 回应：```json\n{\"status\":\"active\"}\n```"
        );
        payload.last_assistant_message.clear();
        assert!(build_message(&payload).is_empty());
        Ok(())
    }
}
