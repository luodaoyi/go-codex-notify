# go-codex-notify

一个给 OpenAI Codex `notify` 配置用的 Telegram 通知程序。

它适合这样的场景：

- Codex 任务完成后自动提醒
- 不想再走 PowerShell 编码和转义坑
- 想直接执行一个 Go 编译出来的单文件程序
- 想把 Telegram Bot Token / Chat ID 放到环境变量或配置文件里，而不是写死在代码里

## 功能

- 直接作为 Codex `notify` 外部命令执行
- 自动读取当前工作目录
- 自动附带：
  - 当前目录名
  - 当前完整路径
  - 主机名
  - 当前时间
  - Git 根目录
  - Git 分支
  - 最近一次 commit
  - 工作区是否有未提交改动
- 尝试从标准输入读取 Codex 传入的 payload
- Telegram 配置支持：
  - 环境变量
  - JSON 配置文件

## 配置方式

程序按这个优先级读取配置：

1. 环境变量
2. 配置文件

### 方式一：环境变量

设置两个环境变量：

- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

#### PowerShell 临时设置

```powershell
$env:TELEGRAM_BOT_TOKEN = "123456789:xxxxxx"
$env:TELEGRAM_CHAT_ID = "123456789"
```

#### Windows 永久设置

```powershell
setx TELEGRAM_BOT_TOKEN "123456789:xxxxxx"
setx TELEGRAM_CHAT_ID "123456789"
```

设置后重新打开终端。

---

### 方式二：配置文件

默认配置文件路径：

```text
%USERPROFILE%\.codex\notify-telegram.json
```

示例：

```json
{
  "bot_token": "123456789:xxxxxx",
  "chat_id": "123456789"
}
```

如果想改路径，可以设置环境变量：

- `CODEX_NOTIFY_CONFIG`

例如：

```powershell
$env:CODEX_NOTIFY_CONFIG = "C:\\Users\\tb16p\\.codex\\notify-telegram.json"
```

## Codex 配置

把编译后的程序放到例如：

```text
C:\Users\tb16p\.codex\notify-telegram.exe
```

然后在 `~/.codex/config.toml` 里写：

```toml
notify = ["C:\\Users\\tb16p\\.codex\\notify-telegram.exe"]
```

这是最推荐的方式：

- 不走 PowerShell
- 不需要额外转义 Telegram 文本
- 不需要传 Bot Token / Chat ID 参数

## 本地运行

### 直接运行

```bash
go run .
```

如果没有提供配置，会报错提示缺少 `TELEGRAM_BOT_TOKEN` 或 `TELEGRAM_CHAT_ID`。

### 模拟 Codex payload

可以手工往标准输入喂 JSON：

```bash
echo '{"client":"codex-tui","task":"修复登录流程","status":"completed","message":"老板可以回来看看了"}' | go run .
```

## 编译

### 本机编译

```bash
go build -o notify-telegram .
```

### Windows

```bash
GOOS=windows GOARCH=amd64 go build -o notify-telegram-windows-amd64.exe .
```

### macOS Apple Silicon

```bash
GOOS=darwin GOARCH=arm64 go build -o notify-telegram-darwin-arm64 .
```

### Linux amd64

```bash
GOOS=linux GOARCH=amd64 go build -o notify-telegram-linux-amd64 .
```

## GitHub Actions

仓库已经附带多平台构建工作流，会在以下场景触发：

- push 到 `main`
- pull request
- 手动触发 `workflow_dispatch`
- 发布 tag（`v*`）时自动构建 release asset

构建目标：

- Windows amd64
- Windows arm64
- Linux amd64
- Linux arm64
- macOS amd64
- macOS arm64

### 自动 Release（两种方式）

#### 方式一：直接 push tag

只要推送形如 `v*` 的 tag，例如：

```bash
git tag v1.0.0
git push origin v1.0.0
```

现有 `ci.yml` 会自动：

- 构建多平台二进制
- 上传 Actions artifacts
- 创建 GitHub Release
- 把构建产物作为 release assets 附到该 tag

#### 方式二：在 Actions 页面点按钮手动发布

仓库还提供了一个单独的 `release` workflow，会在 GitHub Actions 页面显示 `Run workflow` 按钮。

使用方式：

1. 打开 GitHub 仓库的 **Actions** 页面
2. 选择 **release** workflow
3. 点击 **Run workflow**
4. 输入版本号，例如：`v1.0.0`

这个 workflow 会自动：

- 校验版本号格式
- 创建并推送 tag
- 创建 GitHub Release

随后 `ci.yml` 会因为 tag 被推送而继续自动构建多平台二进制并上传为 release assets。

## Telegram Chat ID 获取方法

### 私聊

先给你的 bot 发一条消息，然后访问：

```text
https://api.telegram.org/bot<你的BotToken>/getUpdates
```

找到返回 JSON 里的：

- `message.chat.id`

### 群组

把 bot 拉进群里，在群里发一条消息，然后再看：

- `message.chat.id`

群通常类似：

```text
-100xxxxxxxxxx
```

## 输出内容示例

程序发送到 Telegram 的消息大概会长这样：

```text
老板，Codex 任务已完成。

时间：2026-04-20 11:20:00
机器：TB16P
目录名：my-project
完整路径：C:\Users\tb16p\code\my-project
客户端：codex-tui
任务：修复登录流程
状态：completed
消息：老板可以回来看看了
Git 根目录：C:\Users\tb16p\code\my-project
Git 分支：main
最近提交：a1b2c3 fix login callback
工作区状态：有未提交改动
```

## 开发

### 运行测试

当前项目没有额外单元测试，先执行：

```bash
go test ./...
```

### 格式化

```bash
gofmt -w .
```

## 许可证

MIT
