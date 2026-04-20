package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestReadPayloadReturnsEmptyOnTTYLikeInput(t *testing.T) {
	oldStdin := os.Stdin
	defer func() { os.Stdin = oldStdin }()

	f, err := os.OpenFile("/dev/null", os.O_RDONLY, 0)
	if err != nil {
		t.Fatalf("open /dev/null: %v", err)
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
