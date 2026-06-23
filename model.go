package main

import (
	"context"

	"charm.land/bubbles/v2/textarea"
	"charm.land/bubbles/v2/viewport"
	tea "charm.land/bubbletea/v2"
)

type Screen int

const (
	ScreenLogin Screen = iota
	ScreenMain
)

type Panel int

const (
	PanelChatList Panel = iota
	PanelMessages
)

type Model struct {
	config *Config
	width  int
	height int

	screen Screen
	panel  Panel
	mode   Mode

	// Login
	authInput string
	authState string // "phone", "code", "password"

	// Chat list
	chats      []Chat
	chatCursor int
	searchQuery string // active search filter
	searching   bool   // in search mode

	// Messages
	messages   []Message
	msgCursor  int // selected message index (-1 = none)
	replyTo    int64 // message ID we're replying to (0 = no reply)
	msgView    viewport.Model
	input      textarea.Model

	// Status
	status string

	// Telegram
	tg       *TelegramClient
	tgEvents chan tea.Msg
}

func NewModel(cfg *Config) Model {
	ti := textarea.New()
	ti.Placeholder = "Type a message..."
	ti.ShowLineNumbers = false
	ti.SetHeight(2)

	events := make(chan tea.Msg, 100)
	tgClient := NewTelegramClient(cfg, events)

	return Model{
		config:    cfg,
		screen:    ScreenLogin,
		panel:     PanelChatList,
		mode:      ModeNormal,
		status:    "Connecting...",
		authState: "phone",
		msgCursor: -1,
		msgView:   viewport.New(),
		input:     ti,
		tg:        tgClient,
		tgEvents:  events,
	}
}

func (m Model) Init() tea.Cmd {
	return tea.Batch(
		m.startTelegram(),
		m.waitForTgEvent(),
	)
}

func (m Model) startTelegram() tea.Cmd {
	return func() tea.Msg {
		go func() {
			ctx := context.Background()
			if err := m.tg.Start(ctx); err != nil {
				m.tgEvents <- MsgError{Err: err.Error()}
			}
		}()
		return nil
	}
}

func (m Model) waitForTgEvent() tea.Cmd {
	return func() tea.Msg {
		return <-m.tgEvents
	}
}
