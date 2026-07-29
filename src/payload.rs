use std::{
    fs,
    io::{self, IsTerminal, Read},
};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Clone, Debug, Default, Serialize)]
pub struct GoalContext {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub objective: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub token_budget: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tokens_used: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub time_used: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub updated_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub thread_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub turn_id: String,
}

impl GoalContext {
    pub fn is_empty(&self) -> bool {
        self.objective.is_empty()
            && self.status.is_empty()
            && self.token_budget.is_empty()
            && self.tokens_used.is_empty()
            && self.time_used.is_empty()
            && self.created_at.is_empty()
            && self.updated_at.is_empty()
            && self.thread_id.is_empty()
            && self.turn_id.is_empty()
    }
}

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
    pub goal: GoalContext,
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
        ..NotifyPayload::default()
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

pub fn build_message(payload: &NotifyPayload, raw_input: &str) -> String {
    let mut lines = vec![notification_headline(payload), String::new()];

    push_field(&mut lines, "客户端", &payload.client);
    if payload.event != payload.hook_event_name {
        push_field(&mut lines, "事件", &payload.event);
    }
    push_field(&mut lines, "会话", &payload.session_id);
    push_field(&mut lines, "轮次", &payload.turn_id);
    push_field(&mut lines, "项目目录", &payload.cwd);
    push_field(&mut lines, "模型", &payload.model);
    push_field(&mut lines, "权限模式", &payload.permission_mode);
    push_field(&mut lines, "转写记录", &payload.transcript_path);
    push_field(&mut lines, "目标", &payload.goal.objective);
    push_field(&mut lines, "目标状态", &payload.goal.status);
    push_field(&mut lines, "目标耗时", &payload.goal.time_used);
    if !payload.goal.token_budget.is_empty() || !payload.goal.tokens_used.is_empty() {
        let value = if payload.goal.token_budget.is_empty() {
            payload.goal.tokens_used.clone()
        } else {
            format!(
                "{} / {}",
                payload.goal.tokens_used, payload.goal.token_budget
            )
        };
        push_field(&mut lines, "目标 Token", &value);
    }
    push_field(&mut lines, "目标轮次", &payload.goal.turn_id);
    push_field(&mut lines, "目标线程", &payload.goal.thread_id);
    push_field(&mut lines, "工具", &payload.tool_name);
    push_field(&mut lines, "工具调用", &payload.tool_use_id);
    push_field(&mut lines, "任务", &payload.task);
    if !payload.input_messages.is_empty() {
        push_field(
            &mut lines,
            "用户输入",
            &format_input_messages(&payload.input_messages),
        );
    }
    push_field(&mut lines, "状态", &payload.status);
    push_field(&mut lines, "消息", &payload.message);
    push_field(&mut lines, "Codex 回应", &payload.last_assistant_message);

    if !raw_input.is_empty() && payload.message.is_empty() && !payload.has_lifecycle_context() {
        lines.push(String::new());
        lines.push(format!("原始输入：{raw_input}"));
    }

    lines.join("\n").trim().to_owned()
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
    fn has_lifecycle_context(&self) -> bool {
        !self.hook_event_name.is_empty()
            || !self.session_id.is_empty()
            || !self.turn_id.is_empty()
            || !self.cwd.is_empty()
            || !self.transcript_path.is_empty()
            || !self.model.is_empty()
            || !self.permission_mode.is_empty()
            || !self.last_assistant_message.is_empty()
            || !self.input_messages.is_empty()
            || !self.tool_name.is_empty()
            || !self.tool_use_id.is_empty()
    }
}

pub fn enrich_goal_from_transcript(payload: &mut NotifyPayload) {
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
        let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let method = first_string(&raw, &["method", "event", "type"]);
        if method != "thread/goal/updated" && method != "threadGoalUpdated" {
            continue;
        }

        let params = raw.get("params").and_then(Value::as_object).unwrap_or(&raw);
        let goal = params
            .get("goal")
            .and_then(Value::as_object)
            .unwrap_or(params);
        payload.goal = GoalContext {
            objective: first_string(goal, &["objective"]),
            status: first_string(goal, &["status"]),
            token_budget: first_string(goal, &["tokenBudget", "token_budget"]),
            tokens_used: first_string(goal, &["tokensUsed", "tokens_used"]),
            time_used: first_string(goal, &["timeUsedSeconds", "time_used_seconds"]),
            created_at: first_string(goal, &["createdAt", "created_at"]),
            updated_at: first_string(goal, &["updatedAt", "updated_at"]),
            thread_id: first_string(goal, &["threadId", "thread_id"]),
            turn_id: first_string(params, &["turnId", "turn_id"]),
        };
        if payload.goal.thread_id.is_empty() {
            payload.goal.thread_id = first_string(params, &["threadId", "thread_id"]);
        }
        if !payload.goal.objective.is_empty() {
            return;
        }
    }
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
        let message = build_message(&complete, "");
        assert!(message.starts_with("父亲，Codex 任务已完成。"));
        assert!(message.contains("会话：thread-123"));
        assert!(message.contains("用户输入：修复 Bark 通知 / 确认 TUI"));

        let attention = NotifyPayload {
            event: "user-interaction-required".into(),
            message: "Plan Mode 等待用户输入".into(),
            ..NotifyPayload::default()
        };
        assert_eq!(notification_title(&attention), "Codex 需要你继续处理");
    }

    #[test]
    fn reads_latest_goal_from_transcript_without_truncating_numbers() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"method":"thread/goal/updated","params":{{"threadId":"old","goal":{{"objective":"old"}}}}}}"#).unwrap();
        writeln!(file, r#"{{"method":"thread/goal/updated","params":{{"threadId":"thread-1","turnId":"turn-2","goal":{{"objective":"迁移 Rust","status":"active","tokenBudget":200000,"tokensUsed":12340,"timeUsedSeconds":90}}}}}}"#).unwrap();

        let mut payload = NotifyPayload {
            transcript_path: file.path().to_string_lossy().into_owned(),
            ..NotifyPayload::default()
        };
        enrich_goal_from_transcript(&mut payload);
        assert_eq!(payload.goal.objective, "迁移 Rust");
        assert_eq!(payload.goal.thread_id, "thread-1");
        assert_eq!(payload.goal.turn_id, "turn-2");
        assert_eq!(payload.goal.token_budget, "200000");
        assert_eq!(payload.goal.tokens_used, "12340");
        assert_eq!(payload.goal.time_used, "90");
    }
}
