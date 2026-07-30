# codex-notify

用 Rust 编写的 Codex 多渠道通知工具。npm 主包是 `@asural/codex-notify`，全局命令和原生程序都叫 `codex-notify`（Windows 为 `codex-notify.exe`）。

支持 Telegram、Bark、OpeniLink Hub 和 Hermes Webhook。配置多个渠道时会同时发送。

## 安装

需要 Node.js 18 或更高版本：

```bash
npm install -g @asural/codex-notify
codex-notify
```

npm 只负责安装适合当前系统的原生 Rust 程序。后续运行不依赖 `npx`。

无参数启动时会进入 TUI。若当前环境不是交互终端，可显式运行：

```bash
codex-notify ui
```

## TUI 配置与使用

首次运行 `codex-notify` 后：

1. 进入“通知配置”。
2. 填写至少一个通知渠道；不使用的渠道保持为空。
3. 将“通知范围”设为“仅主代理”（推荐）或“主代理 + SubAgent”。
4. 按 `Ctrl+S` 保存；非编辑状态下也可以按 `s`。
5. 新开 Codex 会话，运行 `/hooks`，检查并信任 `codex-notify` Hook。

保存配置时会同时完成三件事：

1. 将当前原生程序复制到 `~/.codex/bin/codex-notify`，Windows 下为 `~/.codex/bin/codex-notify.exe`。
2. 创建或合并 `~/.codex/hooks.json`：仅主代理模式注册 `Stop`，全部代理模式同时注册 `Stop` 和 `SubagentStop`。
3. 如果 `~/.codex/config.toml` 中仍有本工具旧版本写入的 `notify`，自动移除；其它 Codex 配置和第三方 Hooks 不会被删除。

TUI 可以直接填写并写入 JSON：

- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`
- `BARK_SERVER_URL`
- `HERMES_WEBHOOK_URL`
- “通知范围”：选择“仅主代理”或“主代理 + SubAgent”

保存后会自动创建或更新 `~/.codex/codex-notify.json`，并同步 EXE 和 Hooks。Bot Token 在界面中以圆点掩码显示；未在表单中展示的 OpeniLink 配置和 `HERMES_WEBHOOK_SECRET` 会原样保留。

Codex 会审核新的命令 Hook。安装或 Hook 命令变化后，必须新开一个 Codex 会话并运行 `/hooks`，确认命令指向 `~/.codex/bin/codex-notify` 后选择信任。未经信任的 Hook 不会运行。

配置页快捷键：

- `↑`、`↓` 或 `Tab`：切换字段
- `Enter`：开始或结束编辑；在 SubAgent 开关上切换状态
- `Backspace`：删除一个字符
- `Ctrl+U` 或 `Delete`：清空当前字段
- `Ctrl+S`：保存；非编辑状态下也可按 `s`
- `Esc`：放弃尚未保存的修改并返回主菜单

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

可用 `CODEX_NOTIFY_CONFIG` 指定其它配置文件：

```powershell
$env:CODEX_NOTIFY_CONFIG = 'D:\configs\codex-notify.json'
codex-notify ui
```

```bash
CODEX_NOTIFY_CONFIG=/path/to/codex-notify.json codex-notify ui
```

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

TUI 编辑和保存的是 JSON 文件；环境变量只在当前进程运行时覆盖配置，不会被 TUI 自动写入文件。

`ignore_subagent_notifications` 默认为 `false`。设为 `true` 时安装器只注册官方 `Stop` Hook；设为 `false` 时同时注册 `Stop` 和 `SubagentStop`。切换后在 TUI 保存，或再次运行 `codex-notify install` 使 Hooks 生效。

## 命令

```text
codex-notify                  启动 TUI；非交互环境下处理 stdin
codex-notify ui               显式启动 TUI
codex-notify install|apply    复制 EXE、配置 Hooks 并迁移旧 notify
codex-notify status           查看 EXE、Stop 和 SubagentStop Hook 状态
codex-notify uninstall|remove 移除本工具写入的 Hooks 和旧 notify
codex-notify hook             处理 Codex Hook stdin
codex-notify notify [JSON]    手动处理通知 payload，也可从 stdin 读取
codex-notify --version        显示版本
```

`uninstall` 只移除命令指向本工具 EXE 的 `Stop`/`SubagentStop` Hook，以及本工具旧版本写入的 `notify`；不会删除其它 Codex 配置或第三方 Hooks。

手动测试：

```bash
codex-notify notify '{"type":"agent-turn-complete","last-assistant-message":"测试通知"}'
```

## 通知内容

所有渠道的可见正文只包含“用户输入”和“Codex 回应”。Bark 使用原生 `markdown` 字段发送并保留正文中的基础 Markdown；Telegram 会把 Codex 输出的 CommonMark 转换为 `MarkdownV2`，支持标题、列表、链接、粗体、斜体、删除线、引用、行内代码和代码块，并自动转义普通文本中的特殊字符。

```markdown
**用户输入**

<本轮用户消息>

---

**Codex 回应**

<本轮最终回应>
```

会话 ID、轮次 ID、工作目录、模型、权限模式和 transcript 路径不会进入通知，也不会作为 Hermes 的额外字段发送。Codex `Stop` Hook 没有直接提供用户输入时，程序会从 Hook 给出的 transcript 中读取本轮最后一条用户消息。

## Hook 与 Hermes 兼容

Codex Hooks 会将 JSON 写入程序 stdin。本工具识别官方 Hook 字段，并继续兼容旧 `notify` 的字段别名；非 JSON 文本会作为 `Codex 回应` 发送。

Hermes 请求只包含 `event_type: "codex_notify"` 和上述 `message`。设置 `HERMES_WEBHOOK_SECRET` 后会发送小写十六进制的 `X-Webhook-Signature`（HMAC-SHA256）。

## 常见问题

### 运行 `codex-notify` 没有进入 TUI

无参数启动仅在 stdin 和 stdout 都是交互终端时进入 TUI。请在本机终端运行 `codex-notify ui`；CI 或管道环境使用子命令。

### 保存后没有收到通知

先运行 `codex-notify status`，确认 EXE 为当前版本且 `Codex Stop Hook` 已配置；新开 Codex 会话并运行 `/hooks` 确认 Hook 已信任。然后用上面的 `notify` 命令测试渠道地址。Telegram 必须同时配置 Bot Token 和 Chat ID，Bark 必须填写完整 POST 地址。

### SubAgent 仍然发送通知

运行 `codex-notify status`。仅主代理模式下不应显示 `SubagentStop Hook`；如果仍显示，进入 TUI 将通知范围切为“仅主代理”并保存。若 EXE 显示“需要更新”，重新执行“安装 / 更新”。不要继续使用旧的顶层 `notify`，因为官方 `notify` payload 没有稳定的主代理/SubAgent 身份字段。

### 想移除通知 Hook

运行 `codex-notify uninstall`。工具只会移除自己的 Hook 和旧 `notify`；第三方配置会保留。

## 开发

需要 Rust 1.80+ 和 Node.js 18+：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
npm test
```

## 自动发布

推送 `vMAJOR.MINOR.PATCH` tag，或手动运行 `release` workflow 并输入版本，会构建 Windows、Linux、macOS 的 x64/ARM64 六个平台，创建 GitHub Release，并发布六个平台包和主包 `@asural/codex-notify`。

发布使用 npm Trusted Publishing（GitHub OIDC），不需要长期 `NPM_TOKEN`。npm 上的七个包都必须配置同一个 Trusted Publisher：

- GitHub organization/user：`luodaoyi`
- Repository：`go-codex-notify`
- Workflow filename：`release.yml`
- Environment：留空

workflow 已声明 `id-token: write` 和 `contents: write`，并通过 `npm publish --provenance` 发布。已存在的同版本包会自动跳过。

## 许可证

MIT
