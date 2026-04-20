package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

type Config struct {
	BotToken         string `json:"bot_token"`
	ChatID           string `json:"chat_id"`
	OpeniLinkHubURL   string `json:"openilink_hub_url"`
	OpeniLinkHubToken string `json:"openilink_hub_token"`
}

type NotifyPayload struct {
	Client  string                 `json:"client,omitempty"`
	Task    string                 `json:"task,omitempty"`
	Status  string                 `json:"status,omitempty"`
	Message string                 `json:"message,omitempty"`
	Event   string                 `json:"event,omitempty"`
	Raw     map[string]interface{} `json:"-"`
}

type TelegramRequest struct {
	ChatID string `json:"chat_id"`
	Text   string `json:"text"`
}

type OpeniLinkRequest struct {
	Content string `json:"content"`
}

func main() {
	cfg, err := loadConfig()
	if err != nil {
		fatalf("load config failed: %v", err)
	}

	payload, rawInput := readPayload()
	msg := buildMessage(payload, rawInput)
	if err := sendNotifications(cfg, msg); err != nil {
		fatalf("send notification failed: %v", err)
	}

	fmt.Println("ok")
}

func loadConfig() (Config, error) {
	cfg := Config{
		BotToken:          strings.TrimSpace(os.Getenv("TELEGRAM_BOT_TOKEN")),
		ChatID:            strings.TrimSpace(os.Getenv("TELEGRAM_CHAT_ID")),
		OpeniLinkHubURL:   strings.TrimSpace(os.Getenv("OPENILINK_HUB_URL")),
		OpeniLinkHubToken: strings.TrimSpace(os.Getenv("OPENILINK_HUB_TOKEN")),
	}

	configPath := strings.TrimSpace(os.Getenv("CODEX_NOTIFY_CONFIG"))
	if configPath == "" {
		home, _ := os.UserHomeDir()
		if home != "" {
			configPath = filepath.Join(home, ".codex", "notify-telegram.json")
		}
	}

	if configPath != "" {
		if fileCfg, err := loadConfigFile(configPath); err == nil {
			if cfg.BotToken == "" {
				cfg.BotToken = strings.TrimSpace(fileCfg.BotToken)
			}
			if cfg.ChatID == "" {
				cfg.ChatID = strings.TrimSpace(fileCfg.ChatID)
			}
			if cfg.OpeniLinkHubURL == "" {
				cfg.OpeniLinkHubURL = strings.TrimSpace(fileCfg.OpeniLinkHubURL)
			}
			if cfg.OpeniLinkHubToken == "" {
				cfg.OpeniLinkHubToken = strings.TrimSpace(fileCfg.OpeniLinkHubToken)
			}
		} else if !errors.Is(err, os.ErrNotExist) {
			return Config{}, fmt.Errorf("read config file %s: %w", configPath, err)
		}
	}

	telegramConfigured := cfg.BotToken != "" && cfg.ChatID != ""
	openiLinkConfigured := cfg.OpeniLinkHubURL != "" && cfg.OpeniLinkHubToken != ""

	if !telegramConfigured && !openiLinkConfigured {
		return Config{}, errors.New("no notification channel configured; set Telegram and/or OpeniLink Hub via env or config file")
	}

	if (cfg.BotToken == "") != (cfg.ChatID == "") {
		return Config{}, errors.New("telegram config incomplete; TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID must be set together")
	}

	if (cfg.OpeniLinkHubURL == "") != (cfg.OpeniLinkHubToken == "") {
		return Config{}, errors.New("openilink hub config incomplete; OPENILINK_HUB_URL and OPENILINK_HUB_TOKEN must be set together")
	}

	return cfg, nil
}

func loadConfigFile(path string) (Config, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return Config{}, err
	}
	var cfg Config
	if err := json.Unmarshal(b, &cfg); err != nil {
		return Config{}, err
	}
	return cfg, nil
}

func readPayload() (NotifyPayload, string) {
	stat, err := os.Stdin.Stat()
	if err != nil {
		return NotifyPayload{}, ""
	}
	if (stat.Mode() & os.ModeCharDevice) != 0 {
		return NotifyPayload{}, ""
	}

	b, err := io.ReadAll(os.Stdin)
	if err != nil || len(bytes.TrimSpace(b)) == 0 {
		return NotifyPayload{}, ""
	}

	rawInput := strings.TrimSpace(string(b))

	var raw map[string]interface{}
	if err := json.Unmarshal(b, &raw); err != nil {
		return NotifyPayload{Message: rawInput}, rawInput
	}

	payload := NotifyPayload{Raw: raw}
	payload.Client = firstString(raw, "client")
	payload.Task = firstString(raw, "task", "title", "session", "thread", "cwd")
	payload.Status = firstString(raw, "status", "state", "result")
	payload.Message = firstString(raw, "message", "summary", "text")
	payload.Event = firstString(raw, "event", "type")
	return payload, rawInput
}

func firstString(m map[string]interface{}, keys ...string) string {
	for _, k := range keys {
		if v, ok := m[k]; ok {
			switch t := v.(type) {
			case string:
				if strings.TrimSpace(t) != "" {
					return strings.TrimSpace(t)
				}
			case fmt.Stringer:
				return strings.TrimSpace(t.String())
			}
		}
	}
	return ""
}

func buildMessage(payload NotifyPayload, rawInput string) string {
	cwd, _ := os.Getwd()
	folderName := filepath.Base(cwd)
	hostname, _ := os.Hostname()
	now := time.Now().Format("2006-01-02 15:04:05")

	gitRoot := gitOutput("rev-parse", "--show-toplevel")
	gitBranch := gitOutput("rev-parse", "--abbrev-ref", "HEAD")
	gitCommit := gitOutput("log", "-1", "--pretty=format:%h %s")
	gitDirty := gitDirtyState()

	var sb strings.Builder
	sb.WriteString("老板，Codex 任务已完成。\n\n")
	sb.WriteString("时间：" + now + "\n")
	if hostname != "" {
		sb.WriteString("机器：" + hostname + "\n")
	}
	sb.WriteString("目录名：" + folderName + "\n")
	sb.WriteString("完整路径：" + cwd + "\n")

	if payload.Client != "" {
		sb.WriteString("客户端：" + payload.Client + "\n")
	}
	if payload.Event != "" {
		sb.WriteString("事件：" + payload.Event + "\n")
	}
	if payload.Task != "" {
		sb.WriteString("任务：" + payload.Task + "\n")
	}
	if payload.Status != "" {
		sb.WriteString("状态：" + payload.Status + "\n")
	}
	if payload.Message != "" {
		sb.WriteString("消息：" + payload.Message + "\n")
	}

	if gitRoot != "" {
		sb.WriteString("Git 根目录：" + gitRoot + "\n")
	}
	if gitBranch != "" {
		sb.WriteString("Git 分支：" + gitBranch + "\n")
	}
	if gitCommit != "" {
		sb.WriteString("最近提交：" + gitCommit + "\n")
	}
	if gitDirty != "" {
		sb.WriteString("工作区状态：" + gitDirty + "\n")
	}

	if rawInput != "" && payload.Message == "" {
		sb.WriteString("\n原始输入：" + rawInput + "\n")
	}

	return strings.TrimSpace(sb.String())
}

func gitOutput(args ...string) string {
	cmd := exec.Command("git", args...)
	out, err := cmd.Output()
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(out))
}

func gitDirtyState() string {
	cmd := exec.Command("git", "status", "--porcelain")
	out, err := cmd.Output()
	if err != nil {
		return ""
	}
	if len(bytes.TrimSpace(out)) == 0 {
		return "干净"
	}
	return "有未提交改动"
}

func sendTelegram(cfg Config, text string) error {
	url := fmt.Sprintf("https://api.telegram.org/bot%s/sendMessage", cfg.BotToken)
	body, err := json.Marshal(TelegramRequest{ChatID: cfg.ChatID, Text: text})
	if err != nil {
		return err
	}

	resp, err := http.Post(url, "application/json; charset=utf-8", bytes.NewReader(body))
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("telegram api %s: %s", resp.Status, strings.TrimSpace(string(respBody)))
	}
	return nil
}

func sendOpeniLinkHub(cfg Config, text string) error {
	body, err := json.Marshal(OpeniLinkRequest{Content: text})
	if err != nil {
		return err
	}

	req, err := http.NewRequest(http.MethodPost, cfg.OpeniLinkHubURL, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+cfg.OpeniLinkHubToken)
	req.Header.Set("Content-Type", "application/json; charset=utf-8")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("openilink hub api %s: %s", resp.Status, strings.TrimSpace(string(respBody)))
	}
	return nil
}

func sendNotifications(cfg Config, text string) error {
	var errs []string

	if cfg.BotToken != "" && cfg.ChatID != "" {
		if err := sendTelegram(cfg, text); err != nil {
			errs = append(errs, "telegram: "+err.Error())
		}
	}

	if cfg.OpeniLinkHubURL != "" && cfg.OpeniLinkHubToken != "" {
		if err := sendOpeniLinkHub(cfg, text); err != nil {
			errs = append(errs, "openilink hub: "+err.Error())
		}
	}

	if len(errs) > 0 {
		return errors.New(strings.Join(errs, "; "))
	}
	return nil
}

func fatalf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
