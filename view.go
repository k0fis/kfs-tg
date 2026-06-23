package main

import (
	"fmt"
	"strings"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
)

var (
	styleBorderActive = lipgloss.NewStyle().
				Border(lipgloss.RoundedBorder()).
				BorderForeground(lipgloss.Color("2"))

	styleBorderInactive = lipgloss.NewStyle().
				Border(lipgloss.RoundedBorder()).
				BorderForeground(lipgloss.Color("8"))

	styleStatusBar = lipgloss.NewStyle().
			Background(lipgloss.Color("4")).
			Foreground(lipgloss.Color("15")).
			Padding(0, 1)

	styleChatSelected = lipgloss.NewStyle().
				Background(lipgloss.Color("4")).
				Foreground(lipgloss.Color("15"))

	styleChatUnread = lipgloss.NewStyle().
			Foreground(lipgloss.Color("2")).
			Bold(true)
)

func (m Model) View() tea.View {
	if m.width == 0 {
		v := tea.NewView("Loading...")
		v.AltScreen = true
		return v
	}

	var content string
	switch m.screen {
	case ScreenLogin:
		content = m.viewLogin()
	case ScreenMain:
		content = m.viewMain()
	}

	v := tea.NewView(content)
	v.AltScreen = true
	return v
}

func (m Model) viewLogin() string {
	var prompt string
	switch m.authState {
	case "phone":
		prompt = "Enter phone number (e.g. +420123456789):"
	case "code":
		prompt = "Enter verification code:"
	case "password":
		prompt = "Enter 2FA password:"
	default:
		prompt = "Connecting to Telegram..."
	}

	display := m.authInput
	if m.authState == "password" && len(display) > 0 {
		display = string(make([]byte, len(display)))
		for i := range display {
			display = display[:i] + "*" + display[i+1:]
		}
	}

	content := fmt.Sprintf("  kfs-tg\n\n  %s\n\n  > %s█\n\n  %s", prompt, display, m.status)

	return lipgloss.Place(m.width, m.height,
		lipgloss.Center, lipgloss.Center,
		content,
	)
}

func (m Model) viewMain() string {
	// Border adds 2 chars each side (left+right=2, top+bottom=2)
	borderW := 2
	borderH := 2

	listInnerW := m.config.UI.ChatListWidth
	statusHeight := 1

	// Available height for panels (minus status bar)
	availH := m.height - statusHeight

	// Right panel: split into messages + input
	inputInnerH := 3
	msgInnerH := availH - inputInnerH - borderH*2 // two bordered panels stacked
	if msgInnerH < 5 {
		msgInnerH = 5
	}

	// Right panel inner width (total width - left panel - borders)
	msgInnerW := m.width - listInnerW - borderW*2
	if msgInnerW < 20 {
		msgInnerW = 20
	}

	// Left panel inner height
	leftInnerH := availH - borderH
	if leftInnerH < 5 {
		leftInnerH = 5
	}

	// Chat list
	chatContent := m.renderChatList(listInnerW, leftInnerH-2)
	chatContent += "\n" + lipgloss.NewStyle().Foreground(lipgloss.Color("8")).Render("kfs-tg "+version)
	chatStyle := styleBorderInactive
	if m.panel == PanelChatList {
		chatStyle = styleBorderActive
	}
	leftPanel := chatStyle.Width(listInnerW).Height(leftInnerH).Render(chatContent)

	// Messages
	msgContent := m.msgView.View()
	msgStyle := styleBorderInactive
	if m.panel == PanelMessages {
		msgStyle = styleBorderActive
	}
	msgPanel := msgStyle.Width(msgInnerW).Height(msgInnerH).Render(msgContent)

	// Input
	inputContent := m.input.View()
	inputPanel := styleBorderInactive.Width(msgInnerW).Height(inputInnerH).Render(inputContent)

	rightPanel := lipgloss.JoinVertical(lipgloss.Left, msgPanel, inputPanel)

	main := lipgloss.JoinHorizontal(lipgloss.Top, leftPanel, rightPanel)

	// Status bar
	modeStr := "NORMAL"
	if m.mode == ModeInsert {
		modeStr = "INSERT"
	}
	chatName := ""
	if len(m.chats) > 0 {
		chatName = m.chats[m.chatCursor].Title
	}
	status := styleStatusBar.Width(m.width).Render(
		fmt.Sprintf(" [%s] %s  %s", modeStr, chatName, m.status),
	)

	return lipgloss.JoinVertical(lipgloss.Left, main, status)
}

func (m Model) renderChatList(width, height int) string {
	if len(m.chats) == 0 {
		return "No chats loaded"
	}

	var sb strings.Builder
	visible := height - 2 // reserve space for version label
	start := 0
	if m.chatCursor >= visible {
		start = m.chatCursor - visible + 1
	}

	for i := start; i < len(m.chats) && i-start < visible; i++ {
		chat := m.chats[i]
		line := truncate(chat.Title, width)

		if chat.UnreadCount > 0 {
			line = styleChatUnread.Render(fmt.Sprintf("(%d) %s", chat.UnreadCount, line))
		}

		if i == m.chatCursor {
			line = styleChatSelected.Width(width).Render(line)
		}

		sb.WriteString(line)
		sb.WriteByte('\n')
	}

	return sb.String()
}

func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	if max < 4 {
		return s[:max]
	}
	return s[:max-3] + "..."
}
