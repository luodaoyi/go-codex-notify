package main

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestReadPayloadReturnsEmptyOnTTYLikeInput(t *testing.T) {
	oldStdin := os.Stdin
	defer func() { os.Stdin = oldStdin }()

	f, err := os.OpenFile(os.DevNull, os.O_RDONLY, 0)
	if err != nil {
		t.Fatalf("open null device: %v", err)
	}
	defer f.Close()
	os.Stdin = f

	payload, raw := readPayload()
	if raw != "" {
		t.Fatalf("expected empty raw input, got %q", raw)
	}
	if payload.Client != "" || payload.Task != "" || payload.Status != "" || payload.Message != "" || payload.Event != "" || payload.Raw != nil {
		t.Fatalf("expected empty payload, got %#v", payload)
	}
}

func TestLoadConfigReadsJsonFile(t *testing.T) {
	t.Setenv("TELEGRAM_BOT_TOKEN", "")
	t.Setenv("TELEGRAM_CHAT_ID", "")
	t.Setenv("OPENILINK_HUB_URL", "")
	t.Setenv("OPENILINK_HUB_TOKEN", "")
	t.Setenv("HERMES_WEBHOOK_URL", "")
	t.Setenv("HERMES_WEBHOOK_SECRET", "")

	dir := t.TempDir()
	cfgPath := filepath.Join(dir, "notify-telegram.json")
	content := []byte(`{"bot_token":"bot-from-file","chat_id":"chat-from-file"}`)
	if err := os.WriteFile(cfgPath, content, 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}
	t.Setenv("CODEX_NOTIFY_CONFIG", cfgPath)

	cfg, err := loadConfig()
	if err != nil {
		t.Fatalf("loadConfig returned error: %v", err)
	}
	if cfg.BotToken != "bot-from-file" {
		t.Fatalf("unexpected bot token: %q", cfg.BotToken)
	}
	if cfg.ChatID != "chat-from-file" {
		t.Fatalf("unexpected chat id: %q", cfg.ChatID)
	}
}

func TestLoadConfigReadsHermesWebhookJsonFile(t *testing.T) {
	t.Setenv("TELEGRAM_BOT_TOKEN", "")
	t.Setenv("TELEGRAM_CHAT_ID", "")
	t.Setenv("OPENILINK_HUB_URL", "")
	t.Setenv("OPENILINK_HUB_TOKEN", "")
	t.Setenv("HERMES_WEBHOOK_URL", "")
	t.Setenv("HERMES_WEBHOOK_SECRET", "")

	dir := t.TempDir()
	cfgPath := filepath.Join(dir, "notify-telegram.json")
	content := []byte(`{"hermes_webhook_url":"https://example.test/webhooks/codex-notify","hermes_webhook_secret":"secret-from-file"}`)
	if err := os.WriteFile(cfgPath, content, 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}
	t.Setenv("CODEX_NOTIFY_CONFIG", cfgPath)

	cfg, err := loadConfig()
	if err != nil {
		t.Fatalf("loadConfig returned error: %v", err)
	}
	if cfg.HermesWebhookURL != "https://example.test/webhooks/codex-notify" {
		t.Fatalf("unexpected hermes webhook url: %q", cfg.HermesWebhookURL)
	}
	if cfg.HermesWebhookSecret != "secret-from-file" {
		t.Fatalf("unexpected hermes webhook secret: %q", cfg.HermesWebhookSecret)
	}
}

func TestBuildMessageIncludesCodexStopHookContext(t *testing.T) {
	payload := NotifyPayload{
		Client:               "codex-tui",
		Event:                "Stop",
		SessionID:            "session-123",
		TurnID:               "turn-456",
		HookEventName:        "Stop",
		CWD:                  "/tmp/example-project",
		TranscriptPath:       `/tmp/codex/transcript.jsonl`,
		Model:                "gpt-5.1-codex-max",
		PermissionMode:       "full-auto",
		ToolName:             "shell_command",
		LastAssistantMessage: "已完成修复，并通过 go test ./...。",
	}

	msg := buildMessage(payload, "")
	for _, want := range []string{
		"客户端：codex-tui",
		"会话：session-123",
		"轮次：turn-456",
		"项目目录：/tmp/example-project",
		"模型：gpt-5.1-codex-max",
		"权限模式：full-auto",
		`转写记录：/tmp/codex/transcript.jsonl`,
		"工具：shell_command",
		"Codex 回应：已完成修复，并通过 go test ./...。",
	} {
		if !strings.Contains(msg, want) {
			t.Fatalf("message missing %q:\n%s", want, msg)
		}
	}
	for _, unwanted := range []string{"Hook：", "时间：", "机器：", "目录名：", "完整路径：", "Git 根目录：", "Git 分支：", "最近提交：", "工作区状态：", "任务：", "原始输入："} {
		if strings.Contains(msg, unwanted) {
			t.Fatalf("message should not include %q for parsed lifecycle payload:\n%s", unwanted, msg)
		}
	}
}

func TestEnrichGoalFromTranscriptUsesLastGoalUpdatedEvent(t *testing.T) {
	dir := t.TempDir()
	transcriptPath := filepath.Join(dir, "rollout.jsonl")
	content := strings.Join([]string{
		`{"method":"thread/goal/updated","params":{"threadId":"thread-old","turnId":"turn-old","goal":{"objective":"old goal","status":"active"}}}`,
		`{"method":"turn/completed","params":{"threadId":"thread-1"}}`,
		`{"method":"thread/goal/updated","params":{"threadId":"thread-1","turnId":"turn-2","goal":{"threadId":"thread-1","objective":"把 goal 上下文接进通知","status":"active","tokenBudget":200000,"tokensUsed":12345,"timeUsedSeconds":90,"createdAt":1776272400,"updatedAt":1776272490}}}`,
	}, "\n") + "\n"
	if err := os.WriteFile(transcriptPath, []byte(content), 0o600); err != nil {
		t.Fatalf("write transcript: %v", err)
	}

	payload := NotifyPayload{TranscriptPath: transcriptPath}
	if err := enrichGoalFromTranscript(&payload); err != nil {
		t.Fatalf("enrichGoalFromTranscript returned error: %v", err)
	}
	if payload.Goal.Objective != "把 goal 上下文接进通知" {
		t.Fatalf("unexpected objective: %q", payload.Goal.Objective)
	}
	if payload.Goal.ThreadID != "thread-1" || payload.Goal.TurnID != "turn-2" || payload.Goal.Status != "active" {
		t.Fatalf("unexpected goal context: %#v", payload.Goal)
	}

	msg := buildMessage(payload, "")
	for _, want := range []string{
		"目标：把 goal 上下文接进通知",
		"目标状态：active",
		"目标轮次：turn-2",
		"目标线程：thread-1",
	} {
		if !strings.Contains(msg, want) {
			t.Fatalf("message missing %q:\n%s", want, msg)
		}
	}
}

func TestEnrichGoalFromTranscriptIgnoresMissingOrUnrelatedTranscript(t *testing.T) {
	payload := NotifyPayload{TranscriptPath: filepath.Join(t.TempDir(), "missing.jsonl")}
	if err := enrichGoalFromTranscript(&payload); err != nil {
		t.Fatalf("missing transcript should be ignored: %v", err)
	}
	if payload.Goal.Objective != "" {
		t.Fatalf("unexpected goal from missing transcript: %#v", payload.Goal)
	}
}

func TestSendHermesWebhookPostsSignedCodexNotifyPayload(t *testing.T) {
	const secret = "test-secret"
	const text = "Codex finished"

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Fatalf("unexpected method: %s", r.Method)
		}
		if got := r.Header.Get("Content-Type"); got != "application/json; charset=utf-8" {
			t.Fatalf("unexpected content type: %q", got)
		}

		body, err := io.ReadAll(r.Body)
		if err != nil {
			t.Fatalf("read body: %v", err)
		}

		mac := hmac.New(sha256.New, []byte(secret))
		mac.Write(body)
		wantSig := hex.EncodeToString(mac.Sum(nil))
		if got := r.Header.Get("X-Webhook-Signature"); got != wantSig {
			t.Fatalf("unexpected signature: got %q want %q", got, wantSig)
		}

		var payload map[string]interface{}
		if err := json.Unmarshal(body, &payload); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		if payload["event_type"] != "codex_notify" {
			t.Fatalf("unexpected event_type: %q", payload["event_type"])
		}
		if payload["message"] != text {
			t.Fatalf("unexpected message: %q", payload["message"])
		}

		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	cfg := Config{HermesWebhookURL: server.URL, HermesWebhookSecret: secret}
	if err := sendHermesWebhook(cfg, text, NotifyPayload{}); err != nil {
		t.Fatalf("sendHermesWebhook returned error: %v", err)
	}
}

func TestSendHermesWebhookIncludesStructuredCodexContext(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, err := io.ReadAll(r.Body)
		if err != nil {
			t.Fatalf("read body: %v", err)
		}

		var payload map[string]interface{}
		if err := json.Unmarshal(body, &payload); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		if payload["event_type"] != "codex_notify" {
			t.Fatalf("unexpected event_type: %q", payload["event_type"])
		}
		if payload["hook_event_name"] != "Stop" {
			t.Fatalf("unexpected hook_event_name: %q", payload["hook_event_name"])
		}
		if payload["session_id"] != "session-123" {
			t.Fatalf("unexpected session_id: %q", payload["session_id"])
		}
		if payload["model"] != "gpt-5.1-codex-max" {
			t.Fatalf("unexpected model: %q", payload["model"])
		}
		if payload["last_assistant_message"] != "done" {
			t.Fatalf("unexpected last_assistant_message: %q", payload["last_assistant_message"])
		}
		goal, ok := payload["goal"].(map[string]interface{})
		if !ok {
			t.Fatalf("expected structured goal payload, got %#v", payload["goal"])
		}
		if goal["objective"] != "ship it" || goal["status"] != "active" {
			t.Fatalf("unexpected goal payload: %#v", goal)
		}

		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	cfg := Config{HermesWebhookURL: server.URL}
	payload := NotifyPayload{
		HookEventName:        "Stop",
		SessionID:            "session-123",
		Model:                "gpt-5.1-codex-max",
		LastAssistantMessage: "done",
		Goal: GoalContext{
			Objective: "ship it",
			Status:    "active",
		},
	}
	if err := sendHermesWebhook(cfg, "message", payload); err != nil {
		t.Fatalf("sendHermesWebhook returned error: %v", err)
	}
}
