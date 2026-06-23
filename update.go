package main

import (
	"fmt"
	"strings"
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
type MsgChatReadInbox struct{ ChatID int64; UnreadCount int }
type MsgError struct{ Err string }

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		// Right panel inner width = total right width - 2 (border)
		rightW := m.width - m.config.UI.ChatListWidth - 2
		innerW := rightW - 2
		// Messages inner height = total available - input(5) - status(1) - border(2)
		innerH := m.height - 1 - 5 - 2
		if innerW < 20 {
			innerW = 20
		}
		if innerH < 5 {
			innerH = 5
		}
		m.msgView.SetWidth(innerW)
		m.msgView.SetHeight(innerH)
		m.input.SetWidth(innerW)
		m.input.SetHeight(2)
		return m, nil

	case tea.KeyPressMsg:
		return m.handleKey(msg)

	case tea.PasteMsg:
		if m.mode == ModeInsert {
			var cmd tea.Cmd
			m.input, cmd = m.input.Update(msg)
			return m, cmd
		}

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

	case MsgChatReadInbox:
		for i := range m.chats {
			if m.chats[i].ID == msg.ChatID {
				m.chats[i].UnreadCount = msg.UnreadCount
				break
			}
		}
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

	// Search mode handling
	if m.searching {
		return m.handleSearchKey(msg)
	}

	action := MapKey(msg, m.mode)

	switch action {
	case ActionQuit:
		m.tg.Stop()
		return m, tea.Quit

	case ActionMoveDown:
		if m.panel == PanelChatList {
			chats := m.filteredChats()
			if m.chatCursor < len(chats)-1 {
				m.chatCursor++
			}
		} else if m.panel == PanelMessages && len(m.messages) > 0 {
			if m.msgCursor < len(m.messages)-1 {
				m.msgCursor++
			}
			m.updateMsgView()
		}
	case ActionMoveUp:
		if m.panel == PanelChatList && m.chatCursor > 0 {
			m.chatCursor--
		} else if m.panel == PanelMessages && len(m.messages) > 0 {
			if m.msgCursor > 0 {
				m.msgCursor--
			}
			m.updateMsgView()
		}
	case ActionMoveRight, ActionEnter:
		if m.panel == PanelChatList {
			chats := m.filteredChats()
			if len(chats) > 0 && m.chatCursor < len(chats) {
				// Load messages for selected chat
				m.panel = PanelMessages
				m.msgCursor = -1
				m.replyTo = 0
				chat := chats[m.chatCursor]
				go m.tg.LoadMessages(chat.ID, chat.AccessHash, chat.IsChannel)
			}
		} else {
			m.panel = PanelMessages
		}
	case ActionMoveLeft:
		m.panel = PanelChatList
		m.replyTo = 0

	case ActionEnterInsert:
		m.mode = ModeInsert
		m.input.Focus()
	case ActionExitInsert:
		m.mode = ModeNormal
		m.replyTo = 0
		m.input.Blur()

	case ActionReply:
		if m.panel == PanelMessages && m.msgCursor >= 0 && m.msgCursor < len(m.messages) {
			m.replyTo = m.messages[m.msgCursor].ID
			m.mode = ModeInsert
			m.input.Focus()
			m.status = fmt.Sprintf("Reply to: %s", truncate(m.messages[m.msgCursor].Text, 30))
		}

	case ActionSendMessage:
		if m.mode == ModeInsert && len(m.chats) > 0 {
			text := m.input.Value()
			if text != "" {
				chat := m.chats[m.chatCursor]
				if m.replyTo < 0 {
					// Editing existing message
					msgID := -m.replyTo
					go m.tg.EditMessage(chat.ID, chat.AccessHash, chat.IsChannel, msgID, text)
					// Optimistic update
					for i := range m.messages {
						if m.messages[i].ID == msgID {
							m.messages[i].Text = text
							break
						}
					}
				} else {
					go m.tg.SendMessage(chat.ID, chat.AccessHash, chat.IsChannel, text, m.replyTo)
					// Optimistic: add message locally
					m.messages = append(m.messages, Message{
						Text:       text,
						Timestamp:  time.Now(),
						IsOutgoing: true,
					})
				}
				m.updateMsgView()
				m.input.Reset()
				m.replyTo = 0
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

	case ActionDelete:
		if m.panel == PanelMessages && m.msgCursor >= 0 && m.msgCursor < len(m.messages) {
			msg := m.messages[m.msgCursor]
			if msg.IsOutgoing && msg.ID > 0 {
				chat := m.chats[m.chatCursor]
				go m.tg.DeleteMessage(chat.ID, chat.AccessHash, chat.IsChannel, msg.ID)
				// Optimistic remove
				m.messages = append(m.messages[:m.msgCursor], m.messages[m.msgCursor+1:]...)
				if m.msgCursor >= len(m.messages) {
					m.msgCursor = len(m.messages) - 1
				}
				m.updateMsgView()
			} else {
				m.status = "Can only delete own messages"
			}
		}

	case ActionEditMsg:
		if m.panel == PanelMessages && m.msgCursor >= 0 && m.msgCursor < len(m.messages) {
			msg := m.messages[m.msgCursor]
			if msg.IsOutgoing && msg.ID > 0 {
				m.replyTo = -msg.ID // negative = editing (hack to reuse field)
				m.mode = ModeInsert
				m.input.SetValue(msg.Text)
				m.input.Focus()
				m.status = "Editing message..."
			} else {
				m.status = "Can only edit own messages"
			}
		}

	case ActionRefresh:
		go m.tg.loadChats(m.tg.ctx)
		m.status = "Refreshing..."

	case ActionSearch:
		if m.panel == PanelChatList {
			m.searching = true
			m.searchQuery = ""
			m.status = "/"
		}
	}

	return m, nil
}

func (m Model) handleSearchKey(msg tea.KeyPressMsg) (tea.Model, tea.Cmd) {
	key := msg.String()
	switch key {
	case "esc", "ctrl+c":
		m.searching = false
		m.searchQuery = ""
		m.status = ""
	case "enter":
		m.searching = false
		m.status = ""
	case "backspace":
		if len(m.searchQuery) > 0 {
			m.searchQuery = m.searchQuery[:len(m.searchQuery)-1]
			m.status = "/" + m.searchQuery
			m.chatCursor = 0
		} else {
			m.searching = false
			m.status = ""
		}
	default:
		if len(key) == 1 {
			m.searchQuery += key
		} else if len(msg.Text) > 0 {
			m.searchQuery += msg.Text
		}
		m.status = "/" + m.searchQuery
		m.chatCursor = 0
	}
	return m, nil
}

func (m Model) filteredChats() []Chat {
	if m.searchQuery == "" {
		return m.chats
	}
	query := strings.ToLower(m.searchQuery)
	var filtered []Chat
	for _, c := range m.chats {
		if strings.Contains(strings.ToLower(c.Title), query) {
			filtered = append(filtered, c)
		}
	}
	return filtered
}

func (m *Model) updateMsgView() {
	msgWidth := m.width - m.config.UI.ChatListWidth - 8 // usable width inside borders
	if msgWidth < 20 {
		msgWidth = 80
	}

	var content string
	for i, msg := range m.messages {
		ts := msg.Timestamp.Format("15:04")
		var line string
		if msg.IsOutgoing {
			line = ts + " > " + msg.Text
		} else {
			line = ts + " " + msg.SenderName + ": " + msg.Text
		}
		wrapped := wrapText(line, msgWidth)
		if i == m.msgCursor {
			// Highlight selected message
			wrapped = "│ " + strings.ReplaceAll(wrapped, "\n", "\n│ ")
		}
		content += wrapped + "\n"
	}
	m.msgView.SetContent(content)
	if m.msgCursor < 0 {
		m.msgView.GotoBottom()
	}
}

func wrapText(s string, width int) string {
	if width < 10 {
		return s
	}
	runes := []rune(s)
	if len(runes) <= width {
		return s
	}
	var lines []string
	indent := "      "
	first := true
	for len(runes) > 0 {
		w := width
		if !first {
			w = width - len(indent)
		}
		if len(runes) <= w {
			if first {
				lines = append(lines, string(runes))
			} else {
				lines = append(lines, indent+string(runes))
			}
			break
		}
		// Find last space before width
		cut := w
		for cut > w/2 {
			if runes[cut] == ' ' {
				break
			}
			cut--
		}
		if cut <= w/2 {
			cut = w
		}
		if first {
			lines = append(lines, string(runes[:cut]))
			first = false
		} else {
			lines = append(lines, indent+string(runes[:cut]))
		}
		runes = runes[cut:]
		// Skip leading space on next line
		if len(runes) > 0 && runes[0] == ' ' {
			runes = runes[1:]
		}
	}
	return strings.Join(lines, "\n")
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
