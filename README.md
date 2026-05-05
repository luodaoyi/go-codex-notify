# go-codex-notify

这是一个给 Codex 发完成通知的小工具。

你把它接到 Codex 的完成钩子上后，任务停下来的时候，它会自动把结果发到你常用的地方，比如 Telegram、OpeniLink Hub，或者你自己的 Hermes Webhook。

## 你会收到什么

默认会发一段中文通知，大致长这样：

```text
父亲，Codex 任务已完成。

客户端：codex-tui
会话：...
轮次：...
项目目录：...
模型：...
权限模式：...
转写记录：...
目标：...
工具：...
工具调用：...
任务：...
状态：...
消息：...
Codex 回应：...
```

如果这次任务里设置了 goal，通知里还会自动带上目标摘要。

## 怎么用

### 1）安装

推荐直接运行：

```bash
npx -y go-codex-notify
```

也可以全局安装：

```bash
npm install -g go-codex-notify
```

### 2）配置通知渠道

任选一种或几种都行。配置了哪个，就往哪个发。

#### Telegram

```bash
export TELEGRAM_BOT_TOKEN="123456789:xxxxxx"
export TELEGRAM_CHAT_ID="123456789"
```

#### OpeniLink Hub

```bash
export OPENILINK_HUB_URL="https://hub.011f.com/bot/v1/message/send"
export OPENILINK_HUB_TOKEN="app_xxxxxxxxxxxxxxxxxxxx"
```

#### Hermes Webhook

```bash
export HERMES_WEBHOOK_URL="https://your-server:8644/webhooks/codex-notify"
export HERMES_WEBHOOK_SECRET="your-hermes-webhook-secret"
```

`HERMES_WEBHOOK_SECRET` 可选；设置后会给请求体签名。

### 3）接到 Codex 上

新版 Codex hook 配置写在 `~/.codex/config.toml` 里。先打开 hook 功能，再配置 `Stop` 事件：

```toml
[features]
codex_hooks = true

[[hooks.Stop]]
[[hooks.Stop.hooks]]
type = "command"
command = "npx -y go-codex-notify"
timeout = 30
statusMessage = "Sending notification"
```

如果你已经全局安装了，也可以直接调用：

```toml
[features]
codex_hooks = true

[[hooks.Stop]]
[[hooks.Stop.hooks]]
type = "command"
command = "go-codex-notify"
timeout = 30
statusMessage = "Sending notification"
```

如果你还在用旧版 Codex，才继续用旧的 `notify` 写法：

```toml
notify = ["npx", "-y", "go-codex-notify"]
```

Windows 旧版 `notify` 如果找不到 `npx`，可以写完整路径：

```toml
notify = [
    'C:\Program Files\nodejs\npx.cmd',
    "-y",
    "go-codex-notify",
]
```

## 配置文件

如果你不想全放环境变量，也可以用配置文件。

默认路径：

```text
~/.codex/notify-telegram.json
```

你也可以自己指定：

```bash
export CODEX_NOTIFY_CONFIG="/path/to/notify-telegram.json"
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

## 给 Hermes 的内容

如果你接的是 Hermes Webhook，请求体里会带上中文正文和一些结构化信息。你可以直接把 `{message}` 原样转发出去，也可以自己再做二次加工。

示例：

```json
{
  "event_type": "codex_notify",
  "message": "渲染后的中文通知正文",
  "client": "codex-tui",
  "hook_event_name": "Stop",
  "session_id": "...",
  "turn_id": "...",
  "cwd": "...",
  "transcript_path": "...",
  "model": "...",
  "permission_mode": "...",
  "last_assistant_message": "...",
  "tool_name": "...",
  "tool_use_id": "...",
  "goal": {
    "objective": "...",
    "status": "active",
    "token_budget": "200000",
    "tokens_used": "12345",
    "time_used": "90",
    "created_at": "1776272400",
    "updated_at": "1776272490",
    "thread_id": "...",
    "turn_id": "..."
  }
}
```

没有对应上下文时，`omitempty` 字段不会出现在请求体里。

## 兼容性说明

- Codex 通知配置写在 `~/.codex/config.toml` 的 `notify` 数组里
- 多个通知通道同时配置时，会一起发送
