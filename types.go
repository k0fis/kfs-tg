package main

import "time"

type Chat struct {
	ID          int64
	Title       string
	UnreadCount int
	LastMessage string
	LastDate    time.Time
	Kind        ChatKind
}

type ChatKind int

const (
	ChatPrivate ChatKind = iota
	ChatGroup
	ChatSupergroup
	ChatChannel
)

type Message struct {
	ID         int64
	ChatID     int64
	SenderName string
	Text       string
	Timestamp  time.Time
	IsOutgoing bool
}
