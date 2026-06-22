package main

import (
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

	// Chat list
	chats      []Chat
	chatCursor int

	// Messages
	messages   []Message
	msgView    viewport.Model
	input      textarea.Model

	// Status
	status string

	// TG events channel
	tgEvents chan tea.Msg
}

func NewModel(cfg *Config) Model {
	ti := textarea.New()
	ti.Placeholder = "Type a message..."
	ti.ShowLineNumbers = false

	return Model{
		config:   cfg,
		screen:   ScreenLogin,
		panel:    PanelChatList,
		mode:     ModeNormal,
		status:   "Connecting...",
		msgView:  viewport.New(),
		input:    ti,
		tgEvents: make(chan tea.Msg, 100),
	}
}

func (m Model) Init() tea.Cmd {
	return tea.Batch(
		m.waitForTgEvent(),
	)
}

func (m Model) waitForTgEvent() tea.Cmd {
	return func() tea.Msg {
		return <-m.tgEvents
	}
}
