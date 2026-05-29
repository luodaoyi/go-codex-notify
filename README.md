# go-codex-notify

[![npm version](https://img.shields.io/npm/v/go-codex-notify?logo=npm&label=npm)](https://www.npmjs.com/package/go-codex-notify)
[![npm downloads](https://img.shields.io/npm/dm/go-codex-notify?logo=npm&label=downloads%2Fmonth)](https://www.npmjs.com/package/go-codex-notify)
[![GitHub release](https://img.shields.io/github/v/release/luodaoyi/go-codex-notify?logo=github)](https://github.com/luodaoyi/go-codex-notify/releases)
[![release downloads](https://img.shields.io/github/downloads/luodaoyi/go-codex-notify/total?logo=github&label=release%20downloads)](https://github.com/luodaoyi/go-codex-notify/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/luodaoyi/go-codex-notify/ci.yml?branch=main&logo=githubactions)](https://github.com/luodaoyi/go-codex-notify/actions/workflows/ci.yml)
[![node](https://img.shields.io/node/v/go-codex-notify?logo=nodedotjs)](https://www.npmjs.com/package/go-codex-notify)
[![license](https://img.shields.io/github/license/luodaoyi/go-codex-notify)](./LICENSE)

这是一个给 Codex 发通知的小工具，覆盖任务完成、等待审批、等待你继续对话等场景。

你把它接到 Codex 的 `notify` 上后，Codex 需要你处理或任务结束时，它会自动把消息发到你常用的地方，比如 Telegram、Bark、OpeniLink Hub，或者你自己的 Hermes Webhook。

它是通知型工具：成功时不会向 stdout 输出内容，避免 Codex 把通知工具的输出误当成下一轮上下文。

## 你会收到什么

默认会发一段中文通知，大致长这样：

```text
父亲，Codex 需要你继续处理。

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

任务真正结束时，开头会变成 `父亲，Codex 任务已完成。`。

如果这次任务里设置了 goal，通知里还会自动带上目标摘要。

## 怎么用

### 1）安装

推荐直接运行：

```bash
npx -y go-codex-notify@latest
```

也可以全局安装：

```bash
npm install -g go-codex-notify
```

安装完成后，`postinstall` 会自动把当前平台的原生二进制放到稳定路径：

- macOS / Linux：`~/.codex/bin/go-codex-notify`
- Windows：`C:\Users\你的用户名\.codex\bin\go-codex-notify.exe`

Codex 里推荐直接用 `npx -y go-codex-notify@latest`，配置最少；如果你想完全避免 `npx` 启动开销，也可以改用上面的稳定二进制路径。

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

#### Bark

Bark 只保留一个配置：`BARK_SERVER_URL`。这里填 Bark App 或你的 Bark 服务给出的完整推送地址，工具会直接 POST 到这个 URL：

```bash
export BARK_SERVER_URL="https://t.011f.com/BGMZbH4PLKPpGZ8vmJLTcb"
```

配置文件里同样只写 `bark_server_url`。不要再拆成 server、device key 或其它 Bark 字段。

#### Hermes Webhook

```bash
export HERMES_WEBHOOK_URL="https://your-server:8644/webhooks/codex-notify"
export HERMES_WEBHOOK_SECRET="your-hermes-webhook-secret"
```

`HERMES_WEBHOOK_SECRET` 可选；设置后会给请求体签名。

### 3）接到 Codex 上

Codex 配置写在：

```text
~/.codex/config.toml
```

#### 推荐配置：只使用 notify

`notify` 是 Codex 官方的通知命令入口，例如 Plan Mode 中途停下来等你继续对话、等待审批、任务完成等需要提醒你的场景。这个工具只需要配置这一项，不需要再额外配置 `hooks.Stop`。

Windows：

```toml
notify = ['C:\Program Files\nodejs\npx.cmd', '-y', 'go-codex-notify@latest']
```

macOS / Linux：

```toml
notify = ["npx", "-y", "go-codex-notify@latest"]
```

`notify` 必须写在用户级 `~/.codex/config.toml` 顶层，不要写到项目内 `.codex/config.toml`。不要再配置 `hooks.Stop`，否则可能在任务结束时收到重复通知。

#### notify 能拿到哪些上下文

Codex 官方 `notify` payload 当前常见字段包括：

- `type`：通知类型，当前主要是 `agent-turn-complete`
- `thread-id`：会话标识，会在通知里显示为“会话”
- `turn-id`：轮次标识，会在通知里显示为“轮次”
- `cwd`：当前工作目录，会在通知里显示为“项目目录”
- `input-messages`：触发本轮的用户输入，会在通知里显示为“用户输入”
- `last-assistant-message`：Codex 最后一条回复，会在通知里显示为“Codex 回应”

`notify` 不保证提供会话显示名称/title，也不提供 hook 里的 `transcript_path`。如果需要完整 transcript 路径，只能用 lifecycle hooks；如果只是为了知道哪个项目、哪个会话、用户问了什么，`notify` 已经够用。

如果你想手动刷新这个稳定路径，可以重新执行一次：

```bash
node scripts/install.js
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
  "bark_server_url": "https://t.011f.com/BGMZbH4PLKPpGZ8vmJLTcb",
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

- Codex 通知入口：使用顶层 `notify = [...]`
- 工具仍兼容 stdin JSON 输入，方便手动测试或已有集成迁移
- 多个通知通道同时配置时，会一起发送
