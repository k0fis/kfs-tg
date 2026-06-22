package main

import (
	tea "charm.land/bubbletea/v2"
)

// -- Custom messages from Telegram --

type MsgAuthReady struct{}
type MsgChatsLoaded struct{ Chats []Chat }
type MsgMessagesLoaded struct{ Messages []Message }
type MsgNewMessage struct{ Message Message }
type MsgError struct{ Err string }

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.msgView.SetWidth(m.width - m.config.UI.ChatListWidth - 4)
		m.msgView.SetHeight(m.height - 6)
		return m, nil

	case tea.KeyPressMsg:
		return m.handleKey(msg)

	case MsgAuthReady:
		m.screen = ScreenMain
		m.status = "Ready"
		return m, m.waitForTgEvent()

	case MsgChatsLoaded:
		m.chats = msg.Chats
		return m, m.waitForTgEvent()

	case MsgMessagesLoaded:
		m.messages = msg.Messages
		m.updateMsgView()
		return m, m.waitForTgEvent()

	case MsgNewMessage:
		if len(m.chats) > 0 && m.chats[m.chatCursor].ID == msg.Message.ChatID {
			m.messages = append(m.messages, msg.Message)
			m.updateMsgView()
		}
		return m, m.waitForTgEvent()

	case MsgError:
		m.status = "Error: " + msg.Err
		return m, m.waitForTgEvent()
	}

	return m, nil
}

func (m Model) handleKey(msg tea.KeyPressMsg) (tea.Model, tea.Cmd) {
	action := MapKey(msg, m.mode)

	switch action {
	case ActionQuit:
		return m, tea.Quit

	case ActionMoveDown:
		if m.panel == PanelChatList && m.chatCursor < len(m.chats)-1 {
			m.chatCursor++
		}
	case ActionMoveUp:
		if m.panel == PanelChatList && m.chatCursor > 0 {
			m.chatCursor--
		}
	case ActionMoveRight:
		m.panel = PanelMessages
	case ActionMoveLeft:
		m.panel = PanelChatList

	case ActionEnterInsert:
		m.mode = ModeInsert
		m.input.Focus()
	case ActionExitInsert:
		m.mode = ModeNormal
		m.input.Blur()

	case ActionChar:
		if m.mode == ModeInsert {
			var cmd tea.Cmd
			m.input, cmd = m.input.Update(msg)
			return m, cmd
		}
	case ActionBackspace:
		if m.mode == ModeInsert {
			var cmd tea.Cmd
			m.input, cmd = m.input.Update(msg)
			return m, cmd
		}
	case ActionSendMessage:
		if m.mode == ModeInsert {
			// TODO: send via telegram
			m.input.Reset()
			m.mode = ModeNormal
			m.input.Blur()
		}

	case ActionPageDown:
		m.msgView.PageDown()
	case ActionPageUp:
		m.msgView.PageUp()
	}

	return m, nil
}

func (m *Model) updateMsgView() {
	var content string
	for _, msg := range m.messages {
		ts := msg.Timestamp.Format("15:04")
		if msg.IsOutgoing {
			content += ts + " > " + msg.Text + "\n"
		} else {
			content += ts + " " + msg.SenderName + ": " + msg.Text + "\n"
		}
	}
	m.msgView.SetContent(content)
	m.msgView.GotoBottom()
}
