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
	return lipgloss.Place(m.width, m.height,
		lipgloss.Center, lipgloss.Center,
		"kfs-tg\n\nConnecting to Telegram...\n\n"+m.status,
	)
}

func (m Model) viewMain() string {
	listWidth := m.config.UI.ChatListWidth
	msgWidth := m.width - listWidth - 4 // borders

	// Chat list
	chatContent := m.renderChatList(listWidth-2, m.height-4)
	chatStyle := styleBorderInactive
	if m.panel == PanelChatList {
		chatStyle = styleBorderActive
	}
	leftPanel := chatStyle.Width(listWidth).Height(m.height - 3).Render(chatContent)

	// Messages + input
	msgContent := m.msgView.View()
	inputContent := m.input.View()

	msgStyle := styleBorderInactive
	if m.panel == PanelMessages {
		msgStyle = styleBorderActive
	}
	msgPanel := msgStyle.Width(msgWidth).Height(m.height - 8).Render(msgContent)
	inputPanel := styleBorderInactive.Width(msgWidth).Height(3).Render(inputContent)

	rightPanel := lipgloss.JoinVertical(lipgloss.Left, msgPanel, inputPanel)

	main := lipgloss.JoinHorizontal(lipgloss.Top, leftPanel, rightPanel)

	// Status bar
	modeStr := "NORMAL"
	if m.mode == ModeInsert {
		modeStr = "INSERT"
	}
	status := styleStatusBar.Width(m.width).Render(
		fmt.Sprintf(" [%s] %s", modeStr, m.status),
	)

	return lipgloss.JoinVertical(lipgloss.Left, main, status)
}

func (m Model) renderChatList(width, height int) string {
	if len(m.chats) == 0 {
		return "No chats loaded"
	}

	var sb strings.Builder
	visible := height
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
