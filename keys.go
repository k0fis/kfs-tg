package main

import tea "charm.land/bubbletea/v2"

type Mode int

const (
	ModeNormal Mode = iota
	ModeInsert
)

type Action int

const (
	ActionNone Action = iota
	ActionQuit
	ActionMoveUp
	ActionMoveDown
	ActionMoveLeft
	ActionMoveRight
	ActionEnter
	ActionEnterInsert
	ActionExitInsert
	ActionSendMessage
	ActionNewLine
	ActionSearch
	ActionReply
	ActionForward
	ActionDelete
	ActionEditMsg
	ActionOpenMedia
	ActionSearchChats
	ActionSearchMessages
	ActionOpenChat
	ActionGoTop
	ActionGoBottom
	ActionPageDown
	ActionPageUp
	ActionRefresh
	ActionHelp
	ActionChar
	ActionBackspace
	ActionCursorLeft
	ActionCursorRight
)

func MapKey(msg tea.KeyPressMsg, mode Mode) Action {
	if mode == ModeInsert {
		return mapInsert(msg)
	}
	return mapNormal(msg)
}

func mapNormal(msg tea.KeyPressMsg) Action {
	key := msg.String()
	switch key {
	case "q":
		return ActionQuit
	case "j", "down":
		return ActionMoveDown
	case "k", "up":
		return ActionMoveUp
	case "h", "left":
		return ActionMoveLeft
	case "l", "right":
		return ActionMoveRight
	case "enter":
		return ActionEnter
	case "i":
		return ActionEnterInsert
	case "/":
		return ActionSearch
	case "r":
		return ActionReply
	case "f":
		return ActionForward
	case "e":
		return ActionEditMsg
	case "o":
		return ActionOpenMedia
	case "d":
		return ActionDelete
	case "g":
		return ActionGoTop
	case "G":
		return ActionGoBottom
	case "?":
		return ActionHelp
	case "ctrl+r":
		return ActionRefresh
	case "ctrl+f":
		return ActionSearchChats
	case "ctrl+s":
		return ActionSearchMessages
	case "ctrl+o":
		return ActionOpenChat
	case "ctrl+d":
		return ActionPageDown
	case "ctrl+u":
		return ActionPageUp
	}
	return ActionNone
}

func mapInsert(msg tea.KeyPressMsg) Action {
	key := msg.String()
	switch key {
	case "esc", "ctrl+c":
		return ActionExitInsert
	case "enter":
		return ActionSendMessage
	case "alt+enter", "shift+enter", "ctrl+j", "ctrl+n":
		return ActionNewLine
	case "left":
		return ActionCursorLeft
	case "right":
		return ActionCursorRight
	case "backspace":
		return ActionBackspace
	}
	// Any printable character
	if len(key) == 1 || (len(msg.Text) > 0) {
		return ActionChar
	}
	return ActionNone
}
