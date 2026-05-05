# go-codex-notify

[![npm version](https://img.shields.io/npm/v/go-codex-notify?logo=npm&label=npm)](https://www.npmjs.com/package/go-codex-notify)
[![npm downloads](https://img.shields.io/npm/dm/go-codex-notify?logo=npm&label=downloads%2Fmonth)](https://www.npmjs.com/package/go-codex-notify)
[![GitHub release](https://img.shields.io/github/v/release/luodaoyi/go-codex-notify?logo=github)](https://github.com/luodaoyi/go-codex-notify/releases)
[![release downloads](https://img.shields.io/github/downloads/luodaoyi/go-codex-notify/total?logo=github&label=release%20downloads)](https://github.com/luodaoyi/go-codex-notify/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/luodaoyi/go-codex-notify/ci.yml?branch=main&logo=githubactions)](https://github.com/luodaoyi/go-codex-notify/actions/workflows/ci.yml)
[![node](https://img.shields.io/node/v/go-codex-notify?logo=nodedotjs)](https://www.npmjs.com/package/go-codex-notify)
[![license](https://img.shields.io/github/license/luodaoyi/go-codex-notify)](./LICENSE)

这是一个给 Codex 发完成通知的小工具。

你把它接到 Codex 的完成钩子上后，任务停下来的时候，它会自动把结果发到你常用的地方，比如 Telegram、OpeniLink Hub，或者你自己的 Hermes Webhook。

它是通知型 hook：成功时不会向 stdout 输出内容，避免 Codex 把通知工具的输出误当成下一轮上下文。

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

Codex 配置写在：

```text
~/.codex/config.toml
```

#### 新版 Codex hooks：macOS / Linux

直接使用原生命令即可，不需要再用 shell 包一层：

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

如果你已经全局安装了：

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

#### 新版 Codex hooks：Windows

Windows 上建议写 `npx.cmd` 的完整路径。因为路径里有空格，推荐用 TOML 单引号包住整条命令，然后在命令内部给路径加双引号：

```toml
[features]
codex_hooks = true

[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = '"C:\Program Files\nodejs\npx.cmd" -y go-codex-notify'
timeout = 30
statusMessage = "Sending notification"
```

注意不要写成 `[[hooks]]`，否则 Codex 会报：`invalid type: sequence, expected struct HooksToml in hooks`。`hooks` 本身是普通表，只有 `hooks.Stop` 和 `hooks.Stop.hooks` 是数组表。

新版 Stop hook 会通过 stdin 传入 JSON，例如 `session_id`、`turn_id`、`transcript_path`、`cwd`、`model`、`permission_mode` 和 `last_assistant_message`。`go-codex-notify` 会原样读取这些字段；成功时 stdout 为空，只在失败时向 stderr 写错误并返回非零退出码。

如果你全局安装后有固定的可执行文件路径，也可以这样写：

```toml
[features]
codex_hooks = true

[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = '"C:\Users\你的用户名\AppData\Roaming\npm\go-codex-notify.cmd"'
timeout = 30
statusMessage = "Sending notification"
```

#### 旧版 Codex notify

如果你的 Codex 还不支持 hooks，才用旧的 `notify` 写法：

```toml
notify = ["npx", "-y", "go-codex-notify"]
```

Windows 旧版 `notify` 可以写成：

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

- 新版 Codex：优先使用 `[features] codex_hooks = true` 和 `[[hooks.Stop]]` / `[[hooks.Stop.hooks]]`
- 旧版 Codex：继续使用 `notify = [...]`
- 多个通知通道同时配置时，会一起发送
