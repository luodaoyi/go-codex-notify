# go-codex-notify

`go-codex-notify` 是一个给 OpenAI Codex `notify` 使用的通知工具。

它会在 Codex 任务结束后自动推送一条消息，支持：

- Telegram
- OpeniLink Hub
- Hermes Webhook

只要配置了对应通道就会推送；多个都配置就同时推送。

## 用法

### 安装

推荐直接使用：

```bash
npx -y go-codex-notify
```

也可以全局安装：

```bash
npm install -g go-codex-notify
```

### 配置环境变量

```bash
export TELEGRAM_BOT_TOKEN="123456789:xxxxxx"
export TELEGRAM_CHAT_ID="123456789"
export OPENILINK_HUB_URL="https://hub.011f.com/bot/v1/message/send"
export OPENILINK_HUB_TOKEN="app_xxxxxxxxxxxxxxxxxxxx"
export HERMES_WEBHOOK_URL="https://your-server:8644/webhooks/codex-notify"
export HERMES_WEBHOOK_SECRET="your-hermes-webhook-secret"
```

`HERMES_WEBHOOK_SECRET` 可选；设置后会用请求体生成 HMAC-SHA256，并写入 `X-Webhook-Signature`。

### 或使用配置文件

默认路径：

```text
~/.codex/notify-telegram.json
```

示例：

```json
{
  "bot_token": "123456789:xxxxxx",
  "chat_id": "123456789",
  "openilink_hub_url": "https://hub.011f.com/bot/v1/message/send",
  "openilink_hub_token": "app_xxxxxxxxxxxxxxxxxxxx",
  "hermes_webhook_url": "https://your-server:8644/webhooks/codex-notify",
  "hermes_webhook_secret": "your-hermes-webhook-secret"
}
```

也可以自定义路径：

```bash
export CODEX_NOTIFY_CONFIG="/path/to/notify-telegram.json"
```

### 在 Codex 中配置

```toml
notify = ["npx", "-y", "go-codex-notify"]
```

如果已经全局安装：

```toml
notify = ["go-codex-notify"]
```
