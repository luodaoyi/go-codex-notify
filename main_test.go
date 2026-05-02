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

		var payload map[string]string
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
	if err := sendHermesWebhook(cfg, text); err != nil {
		t.Fatalf("sendHermesWebhook returned error: %v", err)
	}
}
