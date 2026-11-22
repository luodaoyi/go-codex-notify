# Codex Notify

Codex Notify 是 Codex 插件，支持 Telegram、Bark、OpeniLink Hub 和 Hermes Webhook。配置多个渠道时会同时发送。

## 安装

需要 Node.js 18 或以上版本，并确保 Codex 能在 PATH 中找到 Node.js。可在支持 npm 插件源的 Codex CLI 中运行以下命令，也可在 Codex 桌面端的插件设置中添加同一个市场。

~~~sh
codex plugin marketplace add luodaoyi/go-codex-notify
codex plugin add codex-notify@codex-notify
~~~

在下面的本地设置页保存渠道后，启动新的 Codex 会话，在 `/hooks` 中检查并信任插件的 `Stop` 和 `SubagentStop`。通知正文会发送到配置的外部渠道。

### 从旧版迁移

如果旧版曾将命令注册到 `~/.codex/hooks.json` 或 `~/.codex/config.toml`，请在启用插件前运行一次旧版卸载命令，以免两个 Hook 重复发送。

~~~sh
npm exec --yes --package=go-codex-notify@1.3.24 -- codex-notify uninstall
~~~

该命令移除旧版写入的 Hook 和 `notify` 项。保留旧渠道配置，首次打开本地设置页后用「导入旧配置」迁移。

#### 交给 Codex 的迁移提示词

复制下面这段到需要升级的机器上的 Codex 会话。

~~~text
请把当前机器的旧版 go-codex-notify 迁移到 Codex Notify 插件，并验证安装结果。

1. 检查当前 Codex 使用的配置目录、旧版安装来源及 Hook 的命令归属。保留已有的 codex-notify.json、CODEX_NOTIFY_CONFIG 指向的文件、渠道环境变量及其他工具的配置。检查和汇报时只说明配置是否存在，不回显 Key、Token 或带密钥的 URL。
2. 确认 Codex 能在 PATH 中找到 Node.js 18 或以上版本。首次安装时运行：
   codex plugin marketplace add luodaoyi/go-codex-notify
   codex plugin add codex-notify@codex-notify
   如果已装插件，运行 codex plugin marketplace upgrade codex-notify。确认插件版本至少为 1.3.26。
3. 在信任新 Hook 前，运行旧版的 codex-notify uninstall。旧命令不可用时，运行：
   npm exec --yes --package=go-codex-notify@1.3.24 -- codex-notify uninstall
   再用原包管理器卸载已确认的旧全局包，并仅在确认归属后清理旧安装器复制的程序。遇到 config.toml 顶层的 notify，先确认它确实调用旧 Codex Notify。其他工具的 notify 需要保留。
4. 在与 Codex 相同的用户和 CODEX_HOME 下，运行：
   npm exec --yes --package=@asural/codex-notify@latest -- codex-notify-settings
   请用户在本机浏览器打开终端生成的链接，并点击「导入旧配置」。如果插件已有设置，核对页面中的渠道即可。缺少旧配置时，请用户直接在页面填写 URL 和 Key。不要在对话里索取或转发设置链接、URL 或密钥。
5. 新开 Codex 会话，在 /hooks 中检查 Codex Notify 的 Stop 和 SubagentStop。按当前环境的授权流程信任这两项。
6. 汇报插件版本、旧 Hook 是否已移除、设置是否已导入、新 Hook 是否已信任。向真实渠道发测试消息前先取得授权，并说明实际完成了哪些验证。
~~~

### 更新

在 Codex 的插件市场中刷新并更新，或运行以下命令。更新后启动新的会话，检查 `/hooks` 的信任状态。

~~~sh
codex plugin marketplace upgrade codex-notify
~~~

## 本地设置页

在运行 Codex 的同一台机器、同一系统用户和同一 `CODEX_HOME` 下打开终端，运行：

~~~sh
npm exec --yes --package=@asural/codex-notify@latest -- codex-notify-settings
~~~

在本机浏览器打开终端生成的 `127.0.0.1` 链接，填写要使用的渠道并保存。链接包含本次设置会话的访问凭据，请勿分享。完成后按 `Ctrl+C` 关闭设置服务。清空某渠道的字段再保存可停用该渠道。

| 渠道 | 设置项 |
| --- | --- |
| Telegram | Bot Token、Chat ID |
| Bark | 完整推送 URL，包含设备 Key |
| Hermes Webhook | Webhook URL、可选的签名密钥 |
| OpeniLink Hub | Hub URL、Hub Token |

勾选「只推送主代理通知」可关闭子代理通知。

设置保存在 Windows 凭据管理器、macOS 钥匙串或 Linux Secret Service 中。Linux 需要可访问且已解锁的 Secret Service 会话。自定义 `CODEX_HOME` 的设置与默认配置目录分开保存。Windows 凭据库会拒绝超过单条凭据容量的配置，页面会显示错误。

首次配置时可点击「导入旧配置」。导入会一次性读取旧 `~/.codex/codex-notify.json` 或 `CODEX_NOTIFY_CONFIG` 指定的文件，并采用设置进程中非空的旧渠道环境变量覆盖同名值。导入保留旧文件。保存或导入后，Hook 只从系统凭据库读取渠道设置，后续修改通过此页面完成。尚未保存设置时，Hook 会提示先打开设置页。

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

Hermes 请求只包含 `event_type: "codex_notify"` 和上述 `message`。填写 Hermes 签名密钥后会发送小写十六进制的 `X-Webhook-Signature`（HMAC-SHA256）。

## 排查和移除

- 收不到通知时，先在新 Codex 会话的 `/hooks` 检查插件 Hook 的信任状态，再检查渠道配置。Telegram 必须同时填写 Bot Token 和 Chat ID，Bark 需要完整的 POST 地址。
- 如果只需要主代理通知，在设置页勾选「只推送主代理通知」。
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
