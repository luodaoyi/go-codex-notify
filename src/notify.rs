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
    payload::{build_bark_markdown, build_telegram_markdown_v2, notification_title, NotifyPayload},
};

const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";

#[derive(Serialize)]
struct TelegramRequest<'a> {
    chat_id: &'a str,
    text: String,
    parse_mode: &'static str,
}

#[derive(Serialize)]
struct OpeniLinkRequest<'a> {
    content: &'a str,
}

#[derive(Serialize)]
struct BarkRequest {
    title: String,
    markdown: String,
    group: &'static str,
}

#[derive(Serialize)]
struct HermesRequest<'a> {
    event_type: &'static str,
    message: &'a str,
}

pub fn send_all(config: &Config, text: &str, payload: &NotifyPayload) -> Result<()> {
    let client = Client::new();
    let mut errors = Vec::new();

    if !config.bot_token.is_empty() && !config.chat_id.is_empty() {
        collect(
            &mut errors,
            "telegram",
            send_telegram(&client, config, payload),
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
        collect(&mut errors, "bark", send_bark(&client, config, payload));
    }
    if !config.hermes_webhook_url.is_empty() {
        collect(
            &mut errors,
            "hermes webhook",
            send_hermes(&client, config, text),
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

fn send_telegram(client: &Client, config: &Config, payload: &NotifyPayload) -> Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.bot_token
    );
    let body = telegram_body(config, payload)?;
    post_json(client, &url, HeaderMap::new(), body, "telegram api")
}

fn telegram_body(config: &Config, payload: &NotifyPayload) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&TelegramRequest {
        chat_id: &config.chat_id,
        text: build_telegram_markdown_v2(payload),
        parse_mode: "MarkdownV2",
    })?)
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

fn send_bark(client: &Client, config: &Config, payload: &NotifyPayload) -> Result<()> {
    let body = bark_body(payload)?;
    post_json(
        client,
        &config.bark_server_url,
        HeaderMap::new(),
        body,
        "bark api",
    )
}

fn bark_body(payload: &NotifyPayload) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&BarkRequest {
        title: notification_title(payload),
        markdown: build_bark_markdown(payload),
        group: "Codex",
    })?)
}

fn send_hermes(client: &Client, config: &Config, text: &str) -> Result<()> {
    let body = hermes_body(text)?;
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

fn hermes_body(text: &str) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&HermesRequest {
        event_type: "codex_notify",
        message: text,
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
    fn bark_request_uses_markdown_field() {
        let payload = NotifyPayload {
            input_messages: vec!["测试 **Markdown**".into()],
            last_assistant_message: "已完成".into(),
            ..NotifyPayload::default()
        };
        let body = bark_body(&payload).unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["title"], "Codex 任务已完成");
        assert_eq!(
            body["markdown"],
            "**用户输入**\n\n测试 **Markdown**\n\n---\n\n**Codex 回应**\n\n已完成"
        );
        assert!(body.get("body").is_none());
        assert_eq!(body["group"], "Codex");
    }

    #[test]
    fn telegram_request_uses_markdown_v2() {
        let config = Config {
            chat_id: "chat-1".into(),
            ..Config::default()
        };
        let payload = NotifyPayload {
            input_messages: vec!["测试 *格式*!".into()],
            last_assistant_message: "完成_v2.".into(),
            ..NotifyPayload::default()
        };
        let body: Value =
            serde_json::from_slice(&telegram_body(&config, &payload).unwrap()).unwrap();

        assert_eq!(body["chat_id"], "chat-1");
        assert_eq!(body["parse_mode"], "MarkdownV2");
        assert_eq!(
            body["text"],
            "*用户输入*\n测试 \\*格式\\*\\!\n\n*Codex 回应*\n完成\\_v2\\."
        );
    }

    #[test]
    fn hermes_request_contains_only_readable_message() {
        let body = hermes_body("用户输入：测试\nCodex 回应：完成").unwrap();
        let decoded: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(decoded["event_type"], "codex_notify");
        assert_eq!(decoded.as_object().unwrap().len(), 2);
        assert_eq!(decoded["message"], "用户输入：测试\nCodex 回应：完成");
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
