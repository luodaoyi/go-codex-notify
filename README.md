# codex-notify

Codex 多渠道通知工具。项目名 `codex-notify`，npm 主包为
`@asural/codex-notify`；仓库仍为 <https://github.com/luodaoyi/go-codex-notify>。

## 安装与启动

```bash
npm install -g @asural/codex-notify
codex-notify
```

全局命令和原生程序名为 `codex-notify`（Windows 为 `codex-notify.exe`）。无参数且在交互终端启动时会进入 TUI；选择安装项后会自动完成：

- 将当前 EXE 复制到 `~/.codex/bin/codex-notify`（Windows 为 `~/.codex/bin/codex-notify.exe`）；
- 保留 `~/.codex/config.toml` 其它内容，只更新顶层 `notify` 为该 EXE 的绝对路径。

脚本可使用 `codex-notify install`（或 `apply`）；`status` 查看状态，`uninstall` 移除本工具写入的配置。`codex-notify notify [JSON]` 处理手动 payload，也可从 stdin 读取 JSON。

## 配置

默认配置文件：`~/.codex/codex-notify.json`，也可用 `CODEX_NOTIFY_CONFIG` 指定。环境变量优先于配置文件：

| 渠道 | 必填配置 |
| --- | --- |
| Telegram | `TELEGRAM_BOT_TOKEN`、`TELEGRAM_CHAT_ID` |
| OpeniLink Hub | `OPENILINK_HUB_URL`、`OPENILINK_HUB_TOKEN`（Bearer） |
| Bark | `BARK_SERVER_URL`（完整 POST 地址） |
| Hermes Webhook | `HERMES_WEBHOOK_URL`；可选 `HERMES_WEBHOOK_SECRET` |

配置文件字段对应为 `bot_token`、`chat_id`、`openilink_hub_url`、`openilink_hub_token`、`bark_server_url`、`hermes_webhook_url`、`hermes_webhook_secret`。配置多个渠道会同时发送。

## Payload 与 Hermes 兼容

Codex 顶层 `notify = [".../codex-notify"]` 会传入 JSON；常用字段：`type`/`event`、`thread-id`、`turn-id`、`cwd`、`input-messages`、`last-assistant-message`，并兼容下划线和驼峰别名。非 JSON 文本也可直接作为消息。

Hermes 请求为 JSON，固定 `event_type: "codex_notify"` 和渲染后的 `message`；有值时附带 `client`、`hook_event_name`、`session_id`、`turn_id`、`cwd`、`transcript_path`、`model`、`permission_mode`、`last_assistant_message`、`input_messages`、`tool_name`、`tool_use_id`、`goal`。设置 secret 时发送 `X-Webhook-Signature`（HMAC-SHA256 小写十六进制）。没有值的字段会省略。

## 开发

需要 Rust 1.80+、Node.js 18+：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
npm test
```

## 自动发布

推送符合 `vX.Y.Z`（下一版可用 `v1.3.14`）的 tag，或手动运行 `release` workflow（`workflow_dispatch` 输入版本），会构建 Windows/Linux/macOS 的 x64 与 arm64 六个平台，创建或更新 GitHub Release，然后发布六个平台 npm 包及主包 `@asural/codex-notify`。

发布前，在 npm owner `asural` 下创建可写入 `@asural` 公共包的 granular access token；若发布策略强制 2FA，还需允许该 token 绕过发布时的 2FA。在 GitHub 仓库 `Settings > Secrets and variables > Actions` 中把它保存为 `NPM_TOKEN`。workflow 已声明 `contents:write` 与 `id-token:write`；仓库或组织策略也必须允许 Actions 写入 contents。

配置完成后发布是自动的；已成功发布的同版本子包会在 workflow 重跑时跳过。

## 许可证

MIT
