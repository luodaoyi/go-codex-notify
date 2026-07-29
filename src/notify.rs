use anyhow::{anyhow, Context, Result};
use hmac::{Hmac, Mac};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
};
use serde::Serialize;
use sha2::Sha256;

use crate::{
    config::Config,
    payload::{notification_title, GoalContext, NotifyPayload},
};

const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";

#[derive(Serialize)]
struct TelegramRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
}

#[derive(Serialize)]
struct OpeniLinkRequest<'a> {
    content: &'a str,
}

#[derive(Serialize)]
struct BarkRequest<'a> {
    title: String,
    body: &'a str,
    group: &'static str,
}

#[derive(Serialize)]
struct HermesRequest<'a> {
    event_type: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    client: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    hook_event_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    session_id: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    turn_id: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    cwd: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    transcript_path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    model: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    permission_mode: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    last_assistant_message: &'a str,
    #[serde(skip_serializing_if = "string_slice_is_empty")]
    input_messages: &'a [String],
    #[serde(skip_serializing_if = "str::is_empty")]
    tool_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    tool_use_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    goal: Option<&'a GoalContext>,
}

fn string_slice_is_empty(value: &&[String]) -> bool {
    value.is_empty()
}

pub fn send_all(config: &Config, text: &str, payload: &NotifyPayload) -> Result<()> {
    let client = Client::new();
    let mut errors = Vec::new();

    if !config.bot_token.is_empty() && !config.chat_id.is_empty() {
        collect(
            &mut errors,
            "telegram",
            send_telegram(&client, config, text),
        );
    }
    if !config.openilink_hub_url.is_empty() && !config.openilink_hub_token.is_empty() {
        collect(
            &mut errors,
            "openilink hub",
            send_openilink(&client, config, text),
        );
    }
    if !config.bark_server_url.is_empty() {
        collect(
            &mut errors,
            "bark",
            send_bark(&client, config, text, payload),
        );
    }
    if !config.hermes_webhook_url.is_empty() {
        collect(
            &mut errors,
            "hermes webhook",
            send_hermes(&client, config, text, payload),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(errors.join("; "))).context("发送通知失败")
    }
}

fn collect(errors: &mut Vec<String>, provider: &str, result: Result<()>) {
    if let Err(error) = result {
        errors.push(format!("{provider}: {error:#}"));
    }
}

fn send_telegram(client: &Client, config: &Config, text: &str) -> Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.bot_token
    );
    let body = serde_json::to_vec(&TelegramRequest {
        chat_id: &config.chat_id,
        text,
    })?;
    post_json(client, &url, HeaderMap::new(), body, "telegram api")
}

fn send_openilink(client: &Client, config: &Config, text: &str) -> Result<()> {
    let body = serde_json::to_vec(&OpeniLinkRequest { content: text })?;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", config.openilink_hub_token))
            .context("OpeniLink token 不能转换为 HTTP header")?,
    );
    post_json(
        client,
        &config.openilink_hub_url,
        headers,
        body,
        "openilink hub api",
    )
}

fn send_bark(client: &Client, config: &Config, text: &str, payload: &NotifyPayload) -> Result<()> {
    let body = bark_body(text, payload)?;
    post_json(
        client,
        &config.bark_server_url,
        HeaderMap::new(),
        body,
        "bark api",
    )
}

fn bark_body(text: &str, payload: &NotifyPayload) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&BarkRequest {
        title: notification_title(payload),
        body: text,
        group: "Codex",
    })?)
}

fn send_hermes(
    client: &Client,
    config: &Config,
    text: &str,
    payload: &NotifyPayload,
) -> Result<()> {
    let body = hermes_body(text, payload)?;
    let mut headers = HeaderMap::new();
    if !config.hermes_webhook_secret.is_empty() {
        headers.insert(
            "X-Webhook-Signature",
            HeaderValue::from_str(&sign_hermes(&body, &config.hermes_webhook_secret))?,
        );
    }
    post_json(
        client,
        &config.hermes_webhook_url,
        headers,
        body,
        "hermes webhook",
    )
}

fn hermes_body(text: &str, payload: &NotifyPayload) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&HermesRequest {
        event_type: "codex_notify",
        message: text,
        client: &payload.client,
        hook_event_name: &payload.hook_event_name,
        session_id: &payload.session_id,
        turn_id: &payload.turn_id,
        cwd: &payload.cwd,
        transcript_path: &payload.transcript_path,
        model: &payload.model,
        permission_mode: &payload.permission_mode,
        last_assistant_message: &payload.last_assistant_message,
        input_messages: &payload.input_messages,
        tool_name: &payload.tool_name,
        tool_use_id: &payload.tool_use_id,
        goal: (!payload.goal.is_empty()).then_some(&payload.goal),
    })?)
}

fn sign_hermes(body: &[u8], secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC-SHA256 accepts keys of any size");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

fn post_json(
    client: &Client,
    url: &str,
    mut headers: HeaderMap,
    body: Vec<u8>,
    provider: &str,
) -> Result<()> {
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_CONTENT_TYPE));
    let response = client
        .post(url)
        .headers(headers)
        .body(body)
        .send()
        .with_context(|| format!("请求 {provider} 失败"))?;
    let status = response.status();
    let response_body = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("{provider} {status}: {}", response_body.trim()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use serde_json::Value;

    use super::*;

    #[test]
    fn bark_request_matches_existing_contract() {
        let payload = NotifyPayload::default();
        let body = bark_body("Codex finished", &payload).unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["title"], "Codex 任务已完成");
        assert_eq!(body["body"], "Codex finished");
        assert_eq!(body["group"], "Codex");
    }

    #[test]
    fn hermes_request_is_signed_and_structured() {
        let payload = NotifyPayload {
            hook_event_name: "Stop".into(),
            session_id: "session-123".into(),
            goal: GoalContext {
                objective: "ship it".into(),
                status: "active".into(),
                ..GoalContext::default()
            },
            ..NotifyPayload::default()
        };
        let body = hermes_body("done", &payload).unwrap();
        let decoded: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(decoded["event_type"], "codex_notify");
        assert_eq!(decoded["hook_event_name"], "Stop");
        assert_eq!(decoded["goal"]["objective"], "ship it");
        assert_eq!(
            sign_hermes(&body, "test-secret").len(),
            64,
            "SHA-256 HMAC must be lowercase hex"
        );
    }

    #[test]
    fn bark_post_uses_json_content_type_and_url_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if request_complete(&request) {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
            String::from_utf8(request).unwrap()
        });

        let config = Config {
            bark_server_url: format!("http://{address}/device-key"),
            ..Config::default()
        };
        send_all(&config, "Codex finished", &NotifyPayload::default()).unwrap();
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /device-key HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("content-type: application/json; charset=utf-8"));
    }

    fn request_complete(request: &[u8]) -> bool {
        let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        request.len() >= header_end + 4 + content_length
    }
}
