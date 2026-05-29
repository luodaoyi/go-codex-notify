package main

import (
	"bytes"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

type Config struct {
	BotToken            string `json:"bot_token"`
	ChatID              string `json:"chat_id"`
	OpeniLinkHubURL     string `json:"openilink_hub_url"`
	OpeniLinkHubToken   string `json:"openilink_hub_token"`
	HermesWebhookURL    string `json:"hermes_webhook_url"`
	HermesWebhookSecret string `json:"hermes_webhook_secret"`
	BarkServerURL       string `json:"bark_server_url"`
}

type NotifyPayload struct {
	Client               string                 `json:"client,omitempty"`
	Task                 string                 `json:"task,omitempty"`
	Status               string                 `json:"status,omitempty"`
	Message              string                 `json:"message,omitempty"`
	Event                string                 `json:"event,omitempty"`
	HookEventName        string                 `json:"hook_event_name,omitempty"`
	SessionID            string                 `json:"session_id,omitempty"`
	TurnID               string                 `json:"turn_id,omitempty"`
	CWD                  string                 `json:"cwd,omitempty"`
	TranscriptPath       string                 `json:"transcript_path,omitempty"`
	Model                string                 `json:"model,omitempty"`
	PermissionMode       string                 `json:"permission_mode,omitempty"`
	LastAssistantMessage string                 `json:"last_assistant_message,omitempty"`
	InputMessages        []string               `json:"input_messages,omitempty"`
	ToolName             string                 `json:"tool_name,omitempty"`
	ToolUseID            string                 `json:"tool_use_id,omitempty"`
	Goal                 GoalContext            `json:"goal,omitempty"`
	Raw                  map[string]interface{} `json:"-"`
}

type GoalContext struct {
	Objective   string `json:"objective,omitempty"`
	Status      string `json:"status,omitempty"`
	TokenBudget string `json:"token_budget,omitempty"`
	TokensUsed  string `json:"tokens_used,omitempty"`
	TimeUsed    string `json:"time_used,omitempty"`
	CreatedAt   string `json:"created_at,omitempty"`
	UpdatedAt   string `json:"updated_at,omitempty"`
	ThreadID    string `json:"thread_id,omitempty"`
	TurnID      string `json:"turn_id,omitempty"`
}

type TelegramRequest struct {
	ChatID string `json:"chat_id"`
	Text   string `json:"text"`
}

type OpeniLinkRequest struct {
	Content string `json:"content"`
}

type BarkRequest struct {
	Title string `json:"title,omitempty"`
	Body  string `json:"body"`
	Group string `json:"group,omitempty"`
}

type HermesWebhookRequest struct {
	EventType            string      `json:"event_type"`
	Message              string      `json:"message"`
	Client               string      `json:"client,omitempty"`
	HookEventName        string      `json:"hook_event_name,omitempty"`
	SessionID            string      `json:"session_id,omitempty"`
	TurnID               string      `json:"turn_id,omitempty"`
	CWD                  string      `json:"cwd,omitempty"`
	TranscriptPath       string      `json:"transcript_path,omitempty"`
	Model                string      `json:"model,omitempty"`
	PermissionMode       string      `json:"permission_mode,omitempty"`
	LastAssistantMessage string      `json:"last_assistant_message,omitempty"`
	InputMessages        []string    `json:"input_messages,omitempty"`
	ToolName             string      `json:"tool_name,omitempty"`
	ToolUseID            string      `json:"tool_use_id,omitempty"`
	Goal                 GoalContext `json:"goal,omitempty"`
}

func main() {
	cfg, err := loadConfig()
	if err != nil {
		fatalf("load config failed: %v", err)
	}

	payload, rawInput := readPayload()
	if err := enrichGoalFromTranscript(&payload); err != nil {
		fatalf("load goal context failed: %v", err)
	}
	msg := buildMessage(payload, rawInput)
	if err := sendNotifications(cfg, msg, payload); err != nil {
		fatalf("send notification failed: %v", err)
	}
}

func loadConfig() (Config, error) {
	cfg := Config{
		BotToken:            strings.TrimSpace(os.Getenv("TELEGRAM_BOT_TOKEN")),
		ChatID:              strings.TrimSpace(os.Getenv("TELEGRAM_CHAT_ID")),
		OpeniLinkHubURL:     strings.TrimSpace(os.Getenv("OPENILINK_HUB_URL")),
		OpeniLinkHubToken:   strings.TrimSpace(os.Getenv("OPENILINK_HUB_TOKEN")),
		HermesWebhookURL:    strings.TrimSpace(os.Getenv("HERMES_WEBHOOK_URL")),
		HermesWebhookSecret: strings.TrimSpace(os.Getenv("HERMES_WEBHOOK_SECRET")),
		BarkServerURL:       strings.TrimSpace(os.Getenv("BARK_SERVER_URL")),
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
			if cfg.HermesWebhookURL == "" {
				cfg.HermesWebhookURL = strings.TrimSpace(fileCfg.HermesWebhookURL)
			}
			if cfg.HermesWebhookSecret == "" {
				cfg.HermesWebhookSecret = strings.TrimSpace(fileCfg.HermesWebhookSecret)
			}
			if cfg.BarkServerURL == "" {
				cfg.BarkServerURL = strings.TrimSpace(fileCfg.BarkServerURL)
			}
		} else if !errors.Is(err, os.ErrNotExist) {
			return Config{}, fmt.Errorf("read config file %s: %w", configPath, err)
		}
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
	if err == nil && (stat.Mode()&os.ModeCharDevice) == 0 {
		b, readErr := io.ReadAll(os.Stdin)
		if readErr == nil && len(bytes.TrimSpace(b)) > 0 {
			return parsePayloadBytes(b)
		}
	}

	if argPayload := payloadFromArgs(os.Args[1:]); argPayload != "" {
		return parsePayloadBytes([]byte(argPayload))
	}

	return NotifyPayload{}, ""
}

func payloadFromArgs(args []string) string {
	if len(args) == 0 {
		return ""
	}
	first := strings.TrimSpace(args[0])
	if first == "" || strings.HasPrefix(first, "-") {
		return ""
	}
	if strings.HasPrefix(first, "{") || strings.HasPrefix(first, "[") || len(args) == 1 {
		return strings.TrimSpace(strings.Join(args, " "))
	}
	return ""
}

func parsePayloadBytes(b []byte) (NotifyPayload, string) {
	rawInput := strings.TrimSpace(string(b))

	var raw map[string]interface{}
	if err := json.Unmarshal(b, &raw); err != nil {
		return NotifyPayload{Message: rawInput}, rawInput
	}

	payload := NotifyPayload{Raw: raw}
	payload.Client = firstString(raw, "client")
	payload.Task = firstString(raw, "task", "title", "session", "thread", "objective")
	payload.Status = firstString(raw, "status", "state", "result")
	payload.Message = firstString(raw, "message", "summary", "text", "body", "reason")
	payload.Event = firstString(raw, "event", "type", "kind", "hook_event_name", "hookEventName", "hook-event-name")
	payload.HookEventName = firstString(raw, "hook_event_name", "hookEventName", "hook-event-name")
	payload.SessionID = firstString(raw, "session_id", "sessionId", "session-id", "thread-id", "thread_id", "threadId")
	payload.TurnID = firstString(raw, "turn_id", "turnId", "turn-id")
	payload.CWD = firstString(raw, "cwd")
	payload.TranscriptPath = firstString(raw, "transcript_path", "transcriptPath", "transcript-path")
	payload.Model = firstString(raw, "model")
	payload.PermissionMode = firstString(raw, "permission_mode", "permissionMode", "permission-mode")
	payload.LastAssistantMessage = firstString(raw, "last_assistant_message", "lastAssistantMessage", "last-assistant-message")
	payload.InputMessages = firstStringSlice(raw, "input_messages", "inputMessages", "input-messages")
	payload.ToolName = firstString(raw, "tool_name", "toolName", "tool-name")
	payload.ToolUseID = firstString(raw, "tool_use_id", "toolUseId", "tool-use-id")
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
			case json.Number:
				return t.String()
			case float64:
				return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.0f", t), "0"), ".")
			case bool:
				if t {
					return "true"
				}
				return "false"
			case fmt.Stringer:
				return strings.TrimSpace(t.String())
			}
		}
	}
	return ""
}

func firstStringSlice(m map[string]interface{}, keys ...string) []string {
	for _, k := range keys {
		v, ok := m[k]
		if !ok {
			continue
		}
		switch t := v.(type) {
		case []string:
			return cleanStrings(t)
		case []interface{}:
			var values []string
			for _, item := range t {
				switch s := item.(type) {
				case string:
					values = append(values, s)
				case json.Number:
					values = append(values, s.String())
				case float64:
					values = append(values, strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.0f", s), "0"), "."))
				case bool:
					if s {
						values = append(values, "true")
					} else {
						values = append(values, "false")
					}
				}
			}
			return cleanStrings(values)
		case string:
			return cleanStrings([]string{t})
		}
	}
	return nil
}

func cleanStrings(values []string) []string {
	var cleaned []string
	for _, value := range values {
		if s := strings.TrimSpace(value); s != "" {
			cleaned = append(cleaned, s)
		}
	}
	return cleaned
}

func buildMessage(payload NotifyPayload, rawInput string) string {
	var sb strings.Builder
	sb.WriteString(notificationHeadline(payload) + "\n\n")

	if payload.Client != "" {
		sb.WriteString("客户端：" + payload.Client + "\n")
	}
	if payload.Event != "" && payload.Event != payload.HookEventName {
		sb.WriteString("事件：" + payload.Event + "\n")
	}
	if payload.SessionID != "" {
		sb.WriteString("会话：" + payload.SessionID + "\n")
	}
	if payload.TurnID != "" {
		sb.WriteString("轮次：" + payload.TurnID + "\n")
	}
	if payload.CWD != "" {
		sb.WriteString("项目目录：" + payload.CWD + "\n")
	}
	if payload.Model != "" {
		sb.WriteString("模型：" + payload.Model + "\n")
	}
	if payload.PermissionMode != "" {
		sb.WriteString("权限模式：" + payload.PermissionMode + "\n")
	}
	if payload.TranscriptPath != "" {
		sb.WriteString("转写记录：" + payload.TranscriptPath + "\n")
	}
	if payload.Goal.Objective != "" {
		sb.WriteString("目标：" + payload.Goal.Objective + "\n")
	}
	if payload.Goal.Status != "" {
		sb.WriteString("目标状态：" + payload.Goal.Status + "\n")
	}
	if payload.Goal.TimeUsed != "" {
		sb.WriteString("目标耗时：" + payload.Goal.TimeUsed + "\n")
	}
	if payload.Goal.TokenBudget != "" || payload.Goal.TokensUsed != "" {
		sb.WriteString("目标 Token：" + payload.Goal.TokensUsed)
		if payload.Goal.TokenBudget != "" {
			sb.WriteString(" / " + payload.Goal.TokenBudget)
		}
		sb.WriteString("\n")
	}
	if payload.Goal.TurnID != "" {
		sb.WriteString("目标轮次：" + payload.Goal.TurnID + "\n")
	}
	if payload.Goal.ThreadID != "" {
		sb.WriteString("目标线程：" + payload.Goal.ThreadID + "\n")
	}
	if payload.ToolName != "" {
		sb.WriteString("工具：" + payload.ToolName + "\n")
	}
	if payload.ToolUseID != "" {
		sb.WriteString("工具调用：" + payload.ToolUseID + "\n")
	}
	if payload.Task != "" {
		sb.WriteString("任务：" + payload.Task + "\n")
	}
	if len(payload.InputMessages) > 0 {
		sb.WriteString("用户输入：" + formatInputMessages(payload.InputMessages) + "\n")
	}
	if payload.Status != "" {
		sb.WriteString("状态：" + payload.Status + "\n")
	}
	if payload.Message != "" {
		sb.WriteString("消息：" + payload.Message + "\n")
	}
	if payload.LastAssistantMessage != "" {
		sb.WriteString("Codex 回应：" + payload.LastAssistantMessage + "\n")
	}

	if rawInput != "" && payload.Message == "" && !payload.hasLifecycleContext() {
		sb.WriteString("\n原始输入：" + rawInput + "\n")
	}

	return strings.TrimSpace(sb.String())
}

func formatInputMessages(messages []string) string {
	const maxMessages = 3
	const maxRunes = 500

	display := messages
	if len(display) > maxMessages {
		display = display[:maxMessages]
	}
	result := strings.Join(display, " / ")
	if len(messages) > maxMessages {
		result += fmt.Sprintf(" / ...(+%d)", len(messages)-maxMessages)
	}
	return truncateRunes(result, maxRunes)
}

func truncateRunes(s string, limit int) string {
	if limit <= 0 {
		return ""
	}
	runes := []rune(s)
	if len(runes) <= limit {
		return s
	}
	return string(runes[:limit]) + "..."
}

func notificationHeadline(payload NotifyPayload) string {
	eventText := strings.ToLower(strings.Join([]string{
		payload.HookEventName,
		payload.Event,
		payload.Status,
		payload.Message,
	}, " "))

	if strings.EqualFold(payload.HookEventName, "Stop") ||
		strings.EqualFold(payload.Event, "Stop") ||
		containsAny(eventText, "turn-complete", "turn_complete", "completed", "finished", "done") {
		return "父亲，Codex 任务已完成。"
	}

	if containsAny(eventText, "permission", "approval", "approve", "审批", "批准", "权限") {
		return "父亲，Codex 等待你审批。"
	}

	if containsAny(eventText, "input", "interaction", "question", "prompt", "waiting", "resume", "continue", "继续", "对话", "处理") {
		return "父亲，Codex 需要你继续处理。"
	}

	if strings.EqualFold(payload.PermissionMode, "plan") && payload.HookEventName == "" {
		return "父亲，Codex Plan Mode 需要你处理。"
	}

	if payload.Event != "" || payload.Status != "" || payload.Message != "" {
		return "父亲，Codex 有新的通知。"
	}

	return "父亲，Codex 任务已完成。"
}

func containsAny(s string, needles ...string) bool {
	for _, needle := range needles {
		if strings.Contains(s, needle) {
			return true
		}
	}
	return false
}

func notificationTitle(payload NotifyPayload) string {
	title := strings.TrimPrefix(notificationHeadline(payload), "父亲，")
	return strings.TrimSuffix(title, "。")
}

func (p NotifyPayload) hasLifecycleContext() bool {
	return p.HookEventName != "" || p.SessionID != "" || p.TurnID != "" || p.CWD != "" || p.TranscriptPath != "" || p.Model != "" || p.PermissionMode != "" || p.LastAssistantMessage != "" || len(p.InputMessages) > 0 || p.ToolName != "" || p.ToolUseID != ""
}

func enrichGoalFromTranscript(payload *NotifyPayload) error {
	path := strings.TrimSpace(payload.TranscriptPath)
	if path == "" {
		return nil
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	lines := strings.Split(strings.TrimSpace(string(b)), "\n")
	for i := len(lines) - 1; i >= 0; i-- {
		line := strings.TrimSpace(lines[i])
		if line == "" {
			continue
		}
		var raw map[string]interface{}
		if err := json.Unmarshal([]byte(line), &raw); err != nil {
			continue
		}
		method := firstString(raw, "method", "event", "type")
		if method == "" {
			continue
		}
		if method != "thread/goal/updated" && method != "threadGoalUpdated" {
			continue
		}
		params := raw
		if v, ok := raw["params"].(map[string]interface{}); ok {
			params = v
		}
		goalMap := params
		if v, ok := params["goal"].(map[string]interface{}); ok {
			goalMap = v
		}
		payload.Goal = GoalContext{
			Objective:   firstString(goalMap, "objective"),
			Status:      firstString(goalMap, "status"),
			TokenBudget: firstString(goalMap, "tokenBudget", "token_budget"),
			TokensUsed:  firstString(goalMap, "tokensUsed", "tokens_used"),
			TimeUsed:    firstString(goalMap, "timeUsedSeconds", "time_used_seconds"),
			CreatedAt:   firstString(goalMap, "createdAt", "created_at"),
			UpdatedAt:   firstString(goalMap, "updatedAt", "updated_at"),
			ThreadID:    firstString(goalMap, "threadId", "thread_id"),
			TurnID:      firstString(params, "turnId", "turn_id"),
		}
		if payload.Goal.ThreadID == "" {
			payload.Goal.ThreadID = firstString(params, "threadId", "thread_id")
		}
		if payload.Goal.Objective != "" {
			return nil
		}
	}
	return nil
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

func sendBark(cfg Config, text string, payload NotifyPayload) error {
	body, err := json.Marshal(BarkRequest{
		Title: notificationTitle(payload),
		Body:  text,
		Group: "Codex",
	})
	if err != nil {
		return err
	}

	req, err := http.NewRequest(http.MethodPost, cfg.BarkServerURL, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json; charset=utf-8")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("bark api %s: %s", resp.Status, strings.TrimSpace(string(respBody)))
	}
	return nil
}

func sendHermesWebhook(cfg Config, text string, payload NotifyPayload) error {
	body, err := json.Marshal(HermesWebhookRequest{
		EventType:            "codex_notify",
		Message:              text,
		Client:               payload.Client,
		HookEventName:        payload.HookEventName,
		SessionID:            payload.SessionID,
		TurnID:               payload.TurnID,
		CWD:                  payload.CWD,
		TranscriptPath:       payload.TranscriptPath,
		Model:                payload.Model,
		PermissionMode:       payload.PermissionMode,
		LastAssistantMessage: payload.LastAssistantMessage,
		InputMessages:        payload.InputMessages,
		ToolName:             payload.ToolName,
		ToolUseID:            payload.ToolUseID,
		Goal:                 payload.Goal,
	})
	if err != nil {
		return err
	}

	req, err := http.NewRequest(http.MethodPost, cfg.HermesWebhookURL, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json; charset=utf-8")
	if cfg.HermesWebhookSecret != "" {
		req.Header.Set("X-Webhook-Signature", signHermesWebhook(body, cfg.HermesWebhookSecret))
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("hermes webhook %s: %s", resp.Status, strings.TrimSpace(string(respBody)))
	}
	return nil
}

func signHermesWebhook(body []byte, secret string) string {
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(body)
	return hex.EncodeToString(mac.Sum(nil))
}

func sendNotifications(cfg Config, text string, payload NotifyPayload) error {
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

	if cfg.BarkServerURL != "" {
		if err := sendBark(cfg, text, payload); err != nil {
			errs = append(errs, "bark: "+err.Error())
		}
	}

	if cfg.HermesWebhookURL != "" {
		if err := sendHermesWebhook(cfg, text, payload); err != nil {
			errs = append(errs, "hermes webhook: "+err.Error())
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
