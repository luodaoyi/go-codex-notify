# go-codex-notify

一个给 OpenAI Codex `notify` 使用的 Telegram 通知程序。

它的目标很简单：

- Codex 任务完成后，自动给 Telegram 发一条消息
- 不走 PowerShell，不踩编码和转义坑
- 可以直接通过 `npx` 使用
- 只需要配置 Telegram Bot Token 和 Chat ID

## 推荐用法

### 方式一：直接用 npx

```bash
npx go-codex-notify
```

第一次运行时会自动下载当前平台对应的二进制，然后执行。

### 方式二：先全局安装

```bash
npm install -g go-codex-notify
```

安装后可直接运行：

```bash
go-codex-notify
```

---

## 功能

程序会自动收集并发送这些信息：

- 当前时间
- 当前机器名
- 当前目录名
- 当前完整路径
- Git 根目录
- Git 分支
- 最近一次 commit
- 工作区是否有未提交改动
- 如果 Codex 通过标准输入传入了通知 payload，也会尽量带上其中的信息

---

## 配置方式

程序按下面的优先级读取配置：

1. 环境变量
2. 配置文件

### 方式一：环境变量

需要两个环境变量：

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

设置完成后，重新打开终端。

### 方式二：配置文件

默认配置文件路径：

```text
%USERPROFILE%\.codex\notify-telegram.json
```

示例内容：

```json
{
  "bot_token": "123456789:xxxxxx",
  "chat_id": "123456789"
}
```

如果你想使用其他路径，可以设置环境变量：

- `CODEX_NOTIFY_CONFIG`

例如：

```powershell
$env:CODEX_NOTIFY_CONFIG = "C:\Users\tb16p\.codex\notify-telegram.json"
```

---

## 如何获取 Telegram Bot Token

1. 在 Telegram 里找到 **@BotFather**
2. 发送 `/newbot`
3. 按提示创建 bot
4. 创建完成后，BotFather 会返回一串 token

形如：

```text
123456789:AAxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

这就是 `TELEGRAM_BOT_TOKEN`。

---

## 如何获取 Telegram Chat ID

### 私聊

1. 先给你的 bot 发一条消息
2. 打开：

```text
https://api.telegram.org/bot<你的BotToken>/getUpdates
```

3. 在返回 JSON 里找到：

- `message.chat.id`

这就是你的私聊 Chat ID。

### 群组

1. 把 bot 拉进群
2. 在群里发一条消息
3. 再打开：

```text
https://api.telegram.org/bot<你的BotToken>/getUpdates
```

4. 找到：

- `message.chat.id`

群组 ID 通常长这样：

```text
-100xxxxxxxxxx
```

---

## Codex 配置

### 直接用 npx（推荐）

在 `~/.codex/config.toml` 里写：

```toml
notify = ["npx", "-y", "go-codex-notify"]
```

### 如果你已经全局安装

```toml
notify = ["go-codex-notify"]
```

---

## 本地开发编译

### 本机直接编译

```bash
go build -o notify-telegram .
```

### 编译 Windows 版本

```bash
GOOS=windows GOARCH=amd64 go build -o notify-telegram.exe .
```

如果你是在 Windows 上直接编译，通常直接运行：

```powershell
go build -o notify-telegram.exe .
```

就够了。

---

## 本地运行

### 直接运行 Go 程序

```bash
go run .
```

### 运行编译后的二进制

```bash
./notify-telegram
```

Windows：

```powershell
.\notify-telegram.exe
```

### 测试 npm 包入口

```bash
node scripts/install.js
node bin/cli.js
```

如果没有提供配置，会报错提示缺少 `TELEGRAM_BOT_TOKEN` 或 `TELEGRAM_CHAT_ID`。

### 模拟 Codex 输入

可以手工喂一个 JSON 测试：

```bash
echo '{"client":"codex-tui","task":"修复登录流程","status":"completed","message":"老板可以回来看看了"}' | go run .
```

---

## 消息示例

程序发到 Telegram 的消息大概像这样：

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
