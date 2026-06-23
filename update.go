package main

import (
	"time"

	tea "charm.land/bubbletea/v2"
)

// -- Custom messages from Telegram --

type MsgAuthReady struct{}
type MsgNeedAuth struct{ State string }
type MsgChatsLoaded struct{ Chats []Chat }
type MsgMessagesLoaded struct{ Messages []Message }
type MsgNewMessage struct{ Message Message }
type MsgEditedMessage struct{ Message Message }
type MsgDeletedMessages struct{ MessageIDs []int64 }
type MsgError struct{ Err string }

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		msgWidth := m.width - m.config.UI.ChatListWidth - 6
		m.msgView.SetWidth(msgWidth)
		m.msgView.SetHeight(m.height - 10)
		m.input.SetWidth(msgWidth)
		return m, nil

	case tea.KeyPressMsg:
		return m.handleKey(msg)

	case MsgNeedAuth:
		m.screen = ScreenLogin
		m.authState = msg.State
		m.authInput = ""
		m.status = "Enter " + msg.State
		return m, m.waitForTgEvent()

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

	case MsgEditedMessage:
		for i, existing := range m.messages {
			if existing.ID == msg.Message.ID {
				m.messages[i] = msg.Message
				m.updateMsgView()
				break
			}
		}
		return m, m.waitForTgEvent()

	case MsgDeletedMessages:
		filtered := m.messages[:0]
		for _, existing := range m.messages {
			deleted := false
			for _, id := range msg.MessageIDs {
				if existing.ID == id {
					deleted = true
					break
				}
			}
			if !deleted {
				filtered = append(filtered, existing)
			}
		}
		m.messages = filtered
		m.updateMsgView()
		return m, m.waitForTgEvent()

	case MsgError:
		m.status = "Error: " + msg.Err
		return m, m.waitForTgEvent()
	}

	return m, nil
}

func (m Model) handleKey(msg tea.KeyPressMsg) (tea.Model, tea.Cmd) {
	// Login screen has its own key handling
	if m.screen == ScreenLogin {
		return m.handleLoginKey(msg)
	}

	action := MapKey(msg, m.mode)

	switch action {
	case ActionQuit:
		m.tg.Stop()
		return m, tea.Quit

	case ActionMoveDown:
		if m.panel == PanelChatList && m.chatCursor < len(m.chats)-1 {
			m.chatCursor++
		}
	case ActionMoveUp:
		if m.panel == PanelChatList && m.chatCursor > 0 {
			m.chatCursor--
		}
	case ActionMoveRight, ActionEnter:
		if m.panel == PanelChatList && len(m.chats) > 0 {
			// Load messages for selected chat
			m.panel = PanelMessages
			chat := m.chats[m.chatCursor]
			go m.tg.LoadMessages(chat.ID, chat.AccessHash, chat.IsChannel)
		} else {
			m.panel = PanelMessages
		}
	case ActionMoveLeft:
		m.panel = PanelChatList

	case ActionEnterInsert:
		m.mode = ModeInsert
		m.input.Focus()
	case ActionExitInsert:
		m.mode = ModeNormal
		m.input.Blur()

	case ActionSendMessage:
		if m.mode == ModeInsert && len(m.chats) > 0 {
			text := m.input.Value()
			if text != "" {
				chat := m.chats[m.chatCursor]
				go m.tg.SendMessage(chat.ID, chat.AccessHash, chat.IsChannel, text)
				// Optimistic: add message locally
				m.messages = append(m.messages, Message{
					Text:       text,
					Timestamp:  time.Now(),
					IsOutgoing: true,
				})
				m.updateMsgView()
				m.input.Reset()
			}
			m.mode = ModeNormal
			m.input.Blur()
		}
	case ActionNewLine:
		if m.mode == ModeInsert {
			var cmd tea.Cmd
			m.input, cmd = m.input.Update(msg)
			return m, cmd
		}

	case ActionChar, ActionBackspace, ActionCursorLeft, ActionCursorRight:
		if m.mode == ModeInsert {
			var cmd tea.Cmd
			m.input, cmd = m.input.Update(msg)
			return m, cmd
		}

	case ActionPageDown:
		m.msgView.PageDown()
	case ActionPageUp:
		m.msgView.PageUp()
	case ActionRefresh:
		go m.tg.loadChats(m.tg.ctx)
		m.status = "Refreshing..."
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

func (m Model) handleLoginKey(msg tea.KeyPressMsg) (tea.Model, tea.Cmd) {
	key := msg.String()

	switch key {
	case "ctrl+c":
		return m, tea.Quit
	case "enter":
		if m.authInput != "" {
			input := m.authInput
			m.authInput = ""
			m.status = "Verifying..."
			go m.tg.SubmitAuth(input)
			return m, m.waitForTgEvent()
		}
	case "backspace":
		if len(m.authInput) > 0 {
			m.authInput = m.authInput[:len(m.authInput)-1]
		}
	default:
		if len(key) == 1 {
			m.authInput += key
		} else if len(msg.Text) > 0 {
			m.authInput += msg.Text
		}
	}

	return m, nil
}
