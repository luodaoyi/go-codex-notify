# Codex Notify

Codex Notify 是 Codex 插件，支持 Telegram、Bark、OpeniLink Hub 和 Hermes Webhook。配置多个渠道时会同时发送。

## 安装

需要 Node.js 18 或以上版本，并确保 Codex 能在 PATH 中找到 Node.js。可在支持 npm 插件源的 Codex CLI 中运行以下命令，也可在 Codex 桌面端的插件设置中添加同一个市场。

~~~sh
codex plugin marketplace add luodaoyi/go-codex-notify
codex plugin add codex-notify@codex-notify
~~~

配置渠道后，启动新的 Codex 会话，在 `/hooks` 中检查并信任插件的 `Stop` 和 `SubagentStop`。通知正文会发送到配置的外部渠道。

### 从旧版迁移

如果旧版曾将命令注册到 `~/.codex/hooks.json` 或 `~/.codex/config.toml`，请在启用插件前运行一次旧版卸载命令，以免两个 Hook 重复发送。

~~~sh
npm exec --yes --package=go-codex-notify@1.3.24 -- codex-notify uninstall
~~~

该命令移除旧版写入的 Hook 和 `notify` 项。原有渠道配置 `~/.codex/codex-notify.json` 会继续用于插件。

### 更新

在 Codex 的插件市场中刷新并更新，或运行以下命令。更新后启动新的会话，检查 `/hooks` 的信任状态。

~~~sh
codex plugin marketplace upgrade codex-notify
~~~

## 通知渠道

| 渠道 | 配置 | 说明 |
| --- | --- | --- |
| Telegram | `TELEGRAM_BOT_TOKEN`、`TELEGRAM_CHAT_ID` | 两项都填写后才会启用 |
| Bark | `BARK_SERVER_URL` | 完整的 HTTP POST 地址，例如 Bark 服务地址加设备 key |
| Hermes Webhook | `HERMES_WEBHOOK_URL` | 可选 `HERMES_WEBHOOK_SECRET` 用于 HMAC-SHA256 签名 |
| OpeniLink Hub | `OPENILINK_HUB_URL`、`OPENILINK_HUB_TOKEN` | Token 作为 Bearer Token 发送 |

空配置不会启用对应渠道。

### JSON 配置

默认路径是 `~/.codex/codex-notify.json`。示例：

```json
{
  "bot_token": "123456:ABC...",
  "chat_id": "-1001234567890",
  "openilink_hub_url": "https://hub.example.com/api/notify",
  "openilink_hub_token": "openilink-token",
  "hermes_webhook_url": "https://hooks.example.com/codex",
  "hermes_webhook_secret": "optional-signing-secret",
  "bark_server_url": "https://bark.example.com/device-key",
  "ignore_subagent_notifications": true
}
```

只保留实际使用的字段即可。不要将包含 Token 的配置文件提交到 Git。

可用 `CODEX_NOTIFY_CONFIG` 指定其它配置文件。通过环境变量配置时，请把变量设置在启动 Codex 的环境里。

### 环境变量

所有渠道都可以只用环境变量配置。非空环境变量优先于 JSON 文件中的同名配置：

```powershell
$env:TELEGRAM_BOT_TOKEN = '123456:ABC...'
$env:TELEGRAM_CHAT_ID = '-1001234567890'
$env:BARK_SERVER_URL = 'https://bark.example.com/device-key'
$env:HERMES_WEBHOOK_URL = 'https://hooks.example.com/codex'
```

```bash
export TELEGRAM_BOT_TOKEN='123456:ABC...'
export TELEGRAM_CHAT_ID='-1001234567890'
export BARK_SERVER_URL='https://bark.example.com/device-key'
export HERMES_WEBHOOK_URL='https://hooks.example.com/codex'
```

`ignore_subagent_notifications` 默认为 `false`。设为 `true` 时只发主代理通知。

## 通知内容

所有渠道都展示“用户输入”和“Codex 回应”中本轮可用的内容。轮次结束时显示“Codex 本轮更新”，本轮没有可见内容时跳过通知。Bark 使用原生 `markdown` 字段发送并保留正文中的基础 Markdown；Telegram 会把 Codex 输出的 CommonMark 转换为 `MarkdownV2`，支持标题、列表、链接、粗体、斜体、删除线、引用、行内代码和代码块，并自动转义普通文本中的特殊字符。

```markdown
**用户输入**

<本轮用户消息>

---

**Codex 回应**

<本轮最终回应>
```

会话 ID、轮次 ID、工作目录、模型、权限模式和 transcript 路径不会进入通知，也不会作为 Hermes 的额外字段发送。Codex `Stop` Hook 没有直接提供用户输入时，程序会用 Hook 的轮次 ID 在 transcript 里查找本轮用户消息。交互工具回复、内部 Goal 指令和独立的界面注解会被过滤。

## Hermes Webhook

Hermes 请求只包含 `event_type: "codex_notify"` 和上述 `message`。设置 `HERMES_WEBHOOK_SECRET` 后会发送小写十六进制的 `X-Webhook-Signature`（HMAC-SHA256）。

## 排查和移除

- 收不到通知时，先在新 Codex 会话的 `/hooks` 检查插件 Hook 的信任状态，再检查渠道配置。Telegram 必须同时填写 Bot Token 和 Chat ID，Bark 需要完整的 POST 地址。
- 如果只需要主代理通知，把配置里的 `ignore_subagent_notifications` 设为 `true`。
- 可在 Codex 插件设置中移除，或运行 `codex plugin remove codex-notify@codex-notify`。

## 开发

需要 Rust 1.80+ 和 Node.js 18+：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
npm test
```

## 自动发布

推送 `vMAJOR.MINOR.PATCH` 标签会为 Windows、Linux、macOS 的 x64 和 ARM64 构建二进制文件。发布流程把它们装入 `@asural/codex-notify`，并在 GitHub 创建版本说明。

npm 插件包使用 GitHub OIDC 受信任发布。

- GitHub organization/user：`luodaoyi`
- Repository：`go-codex-notify`
- Workflow filename：`release.yml`
- Environment：留空

workflow 已声明 `id-token: write` 和 `contents: write`，并通过 `npm publish --provenance` 发布插件包。已存在的同版本包会自动跳过。

## 许可证

MIT
