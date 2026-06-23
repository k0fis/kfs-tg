package main

import (
	"context"
	"fmt"
	"math/rand"
	"path/filepath"
	"time"

	"github.com/gotd/td/session"
	"github.com/gotd/td/telegram"
	"github.com/gotd/td/telegram/auth"
	"github.com/gotd/td/tg"

	tea "charm.land/bubbletea/v2"
)

// TelegramClient wraps gotd/td for use with bubbletea.
type TelegramClient struct {
	client  *telegram.Client
	api     *tg.Client
	cfg     *Config
	events  chan tea.Msg
	ctx     context.Context
	cancel  context.CancelFunc
	selfID  int64

	// Auth: channel for interactive input from TUI
	authInput chan string
}

func NewTelegramClient(cfg *Config, events chan tea.Msg) *TelegramClient {
	return &TelegramClient{
		cfg:       cfg,
		events:    events,
		authInput: make(chan string, 1),
	}
}

// Start connects to Telegram and begins receiving updates.
// Blocks until ctx is cancelled — run in a goroutine.
func (tc *TelegramClient) Start(ctx context.Context) error {
	tc.ctx, tc.cancel = context.WithCancel(ctx)

	sessionPath := filepath.Join(DataDir(), "session.json")
	storage := &session.FileStorage{Path: sessionPath}

	tc.client = telegram.NewClient(
		tc.cfg.General.ApiID,
		tc.cfg.General.ApiHash,
		telegram.Options{
			SessionStorage: storage,
			UpdateHandler:  telegram.UpdateHandlerFunc(tc.handleUpdates),
		},
	)

	return tc.client.Run(tc.ctx, func(ctx context.Context) error {
		tc.api = tc.client.API()

		// Check auth status
		status, err := tc.client.Auth().Status(ctx)
		if err != nil {
			return fmt.Errorf("auth status: %w", err)
		}

		if !status.Authorized {
			// Need to authenticate
			tc.events <- MsgNeedAuth{State: "phone"}
			if err := tc.authenticate(ctx); err != nil {
				tc.events <- MsgError{Err: fmt.Sprintf("auth failed: %v", err)}
				return err
			}
		}

		// Get self
		self, err := tc.client.Self(ctx)
		if err == nil {
			tc.selfID = self.ID
		}

		tc.events <- MsgAuthReady{}

		// Load initial chats
		tc.loadChats(ctx)

		// Block until cancelled (updates come via handler)
		<-ctx.Done()
		return nil
	})
}

func (tc *TelegramClient) Stop() {
	if tc.cancel != nil {
		tc.cancel()
	}
}

// --- Authentication ---

type authFlow struct {
	tc *TelegramClient
}

func (a *authFlow) Phone(_ context.Context) (string, error) {
	a.tc.events <- MsgNeedAuth{State: "phone"}
	// Block until TUI sends input
	phone := <-a.tc.authInput
	return phone, nil
}

func (a *authFlow) Code(_ context.Context, _ *tg.AuthSentCode) (string, error) {
	a.tc.events <- MsgNeedAuth{State: "code"}
	code := <-a.tc.authInput
	return code, nil
}

func (a *authFlow) Password(_ context.Context) (string, error) {
	a.tc.events <- MsgNeedAuth{State: "password"}
	password := <-a.tc.authInput
	return password, nil
}

func (a *authFlow) AcceptTermsOfService(_ context.Context, tos tg.HelpTermsOfService) error {
	return nil
}

func (a *authFlow) SignUp(_ context.Context) (auth.UserInfo, error) {
	return auth.UserInfo{}, fmt.Errorf("sign up not supported")
}

func (tc *TelegramClient) authenticate(ctx context.Context) error {
	flow := auth.NewFlow(&authFlow{tc: tc}, auth.SendCodeOptions{})
	return flow.Run(ctx, tc.client.Auth())
}

// SubmitAuth sends user input to the blocked auth callback.
func (tc *TelegramClient) SubmitAuth(input string) {
	tc.authInput <- input
}

// --- Load data ---

func (tc *TelegramClient) loadChats(ctx context.Context) {
	result, err := tc.api.MessagesGetDialogs(ctx, &tg.MessagesGetDialogsRequest{
		OffsetPeer: &tg.InputPeerEmpty{},
		Limit:      30,
	})
	if err != nil {
		tc.events <- MsgError{Err: fmt.Sprintf("load chats: %v", err)}
		return
	}

	chats := tc.parseDialogs(result)
	tc.events <- MsgChatsLoaded{Chats: chats}
}

func (tc *TelegramClient) LoadMessages(chatID int64, accessHash int64, isChannel bool) {
	ctx := tc.ctx
	if ctx == nil {
		return
	}

	var peer tg.InputPeerClass
	if isChannel {
		peer = &tg.InputPeerChannel{ChannelID: chatID, AccessHash: accessHash}
	} else if chatID > 0 {
		peer = &tg.InputPeerUser{UserID: chatID, AccessHash: accessHash}
	} else {
		peer = &tg.InputPeerChat{ChatID: -chatID}
	}

	result, err := tc.api.MessagesGetHistory(ctx, &tg.MessagesGetHistoryRequest{
		Peer:  peer,
		Limit: 50,
	})
	if err != nil {
		tc.events <- MsgError{Err: fmt.Sprintf("load messages: %v", err)}
		return
	}

	messages := tc.parseMessages(result)
	tc.events <- MsgMessagesLoaded{Messages: messages}
}

func (tc *TelegramClient) SendMessage(chatID int64, accessHash int64, isChannel bool, text string) {
	ctx := tc.ctx
	if ctx == nil {
		return
	}

	var peer tg.InputPeerClass
	if isChannel {
		peer = &tg.InputPeerChannel{ChannelID: chatID, AccessHash: accessHash}
	} else if chatID > 0 {
		peer = &tg.InputPeerUser{UserID: chatID, AccessHash: accessHash}
	} else {
		peer = &tg.InputPeerChat{ChatID: -chatID}
	}

	_, err := tc.api.MessagesSendMessage(ctx, &tg.MessagesSendMessageRequest{
		Peer:     peer,
		Message:  text,
		RandomID: rand.Int63(),
	})
	if err != nil {
		tc.events <- MsgError{Err: fmt.Sprintf("send: %v", err)}
	}
}

// --- Update handler ---

func (tc *TelegramClient) handleUpdates(ctx context.Context, u tg.UpdatesClass) error {
	switch upd := u.(type) {
	case *tg.Updates:
		for _, update := range upd.Updates {
			tc.handleSingleUpdate(update, upd.Users)
		}
	case *tg.UpdatesCombined:
		for _, update := range upd.Updates {
			tc.handleSingleUpdate(update, upd.Users)
		}
	case *tg.UpdateShort:
		tc.handleSingleUpdate(upd.Update, nil)
	case *tg.UpdateShortMessage:
		tc.events <- MsgNewMessage{Message: Message{
			ID:         int64(upd.ID),
			ChatID:     int64(upd.UserID),
			SenderName: fmt.Sprintf("user:%d", upd.UserID),
			Text:       upd.Message,
			Timestamp:  time.Unix(int64(upd.Date), 0),
			IsOutgoing: upd.Out,
		}}
	case *tg.UpdateShortChatMessage:
		tc.events <- MsgNewMessage{Message: Message{
			ID:         int64(upd.ID),
			ChatID:     int64(-upd.ChatID),
			SenderName: fmt.Sprintf("user:%d", upd.FromID),
			Text:       upd.Message,
			Timestamp:  time.Unix(int64(upd.Date), 0),
			IsOutgoing: upd.Out,
		}}
	}
	return nil
}

func (tc *TelegramClient) handleSingleUpdate(u tg.UpdateClass, users []tg.UserClass) {
	switch upd := u.(type) {
	case *tg.UpdateNewMessage:
		if msg, ok := tc.convertMessage(upd.Message, users); ok {
			tc.events <- MsgNewMessage{Message: msg}
		}
	case *tg.UpdateEditMessage:
		if msg, ok := tc.convertMessage(upd.Message, users); ok {
			tc.events <- MsgEditedMessage{Message: msg}
		}
	case *tg.UpdateDeleteMessages:
		tc.events <- MsgDeletedMessages{MessageIDs: int64Slice(upd.Messages)}
	}
}

// --- Parsers ---

func (tc *TelegramClient) parseDialogs(result tg.MessagesDialogsClass) []Chat {
	var dialogs *tg.MessagesDialogsSlice
	var dialogsNonSlice *tg.MessagesDialogs

	switch r := result.(type) {
	case *tg.MessagesDialogsSlice:
		dialogs = r
	case *tg.MessagesDialogs:
		dialogsNonSlice = r
	default:
		return nil
	}

	// Build user/chat maps
	var userList []tg.UserClass
	var chatList []tg.ChatClass
	var dialogList []tg.DialogClass
	var messageList []tg.MessageClass

	if dialogs != nil {
		userList = dialogs.Users
		chatList = dialogs.Chats
		dialogList = dialogs.Dialogs
		messageList = dialogs.Messages
	} else if dialogsNonSlice != nil {
		userList = dialogsNonSlice.Users
		chatList = dialogsNonSlice.Chats
		dialogList = dialogsNonSlice.Dialogs
		messageList = dialogsNonSlice.Messages
	}

	users := make(map[int64]*tg.User)
	for _, u := range userList {
		if user, ok := u.(*tg.User); ok {
			users[user.ID] = user
		}
	}
	chats := make(map[int64]*tg.Chat)
	channels := make(map[int64]*tg.Channel)
	for _, c := range chatList {
		switch ch := c.(type) {
		case *tg.Chat:
			chats[ch.ID] = ch
		case *tg.Channel:
			channels[ch.ID] = ch
		}
	}

	// Build last message map
	lastMsgs := make(map[int]string)
	for _, m := range messageList {
		if msg, ok := m.(*tg.Message); ok {
			lastMsgs[msg.ID] = extractText(msg)
		}
	}

	var result2 []Chat
	for _, d := range dialogList {
		dlg, ok := d.(*tg.Dialog)
		if !ok {
			continue
		}

		var chat Chat
		chat.UnreadCount = dlg.UnreadCount

		switch p := dlg.Peer.(type) {
		case *tg.PeerUser:
			if u, ok := users[p.UserID]; ok {
				chat.ID = p.UserID
				chat.Title = userName(u)
				chat.Kind = ChatPrivate
				chat.AccessHash = u.AccessHash
			}
		case *tg.PeerChat:
			if c, ok := chats[p.ChatID]; ok {
				chat.ID = -p.ChatID
				chat.Title = c.Title
				chat.Kind = ChatGroup
			}
		case *tg.PeerChannel:
			if ch, ok := channels[p.ChannelID]; ok {
				chat.ID = p.ChannelID
				chat.Title = ch.Title
				chat.AccessHash = ch.AccessHash
				if ch.Broadcast {
					chat.Kind = ChatChannel
				} else {
					chat.Kind = ChatSupergroup
				}
				chat.IsChannel = true
			}
		}

		if chat.Title == "" {
			continue
		}

		if dlg.TopMessage != 0 {
			chat.LastMessage = lastMsgs[dlg.TopMessage]
		}

		result2 = append(result2, chat)
	}

	return result2
}

func (tc *TelegramClient) parseMessages(result tg.MessagesMessagesClass) []Message {
	var msgList []tg.MessageClass
	var userList []tg.UserClass

	switch r := result.(type) {
	case *tg.MessagesMessages:
		msgList = r.Messages
		userList = r.Users
	case *tg.MessagesMessagesSlice:
		msgList = r.Messages
		userList = r.Users
	case *tg.MessagesChannelMessages:
		msgList = r.Messages
		userList = r.Users
	default:
		return nil
	}

	users := make(map[int64]string)
	for _, u := range userList {
		if user, ok := u.(*tg.User); ok {
			users[user.ID] = userName(user)
		}
	}

	var messages []Message
	for _, m := range msgList {
		if msg, ok := tc.convertMessage(m, userList); ok {
			// Resolve sender name
			if msg.SenderName == "" && msg.senderID != 0 {
				if name, ok := users[msg.senderID]; ok {
					msg.SenderName = name
				}
			}
			messages = append(messages, msg)
		}
	}

	// Sort oldest first
	for i, j := 0, len(messages)-1; i < j; i, j = i+1, j-1 {
		messages[i], messages[j] = messages[j], messages[i]
	}

	return messages
}

func (tc *TelegramClient) convertMessage(m tg.MessageClass, users []tg.UserClass) (Message, bool) {
	msg, ok := m.(*tg.Message)
	if !ok {
		return Message{}, false
	}

	result := Message{
		ID:         int64(msg.ID),
		Text:       extractText(msg),
		Timestamp:  time.Unix(int64(msg.Date), 0),
		IsOutgoing: msg.Out,
	}

	// Chat ID from peer
	switch p := msg.PeerID.(type) {
	case *tg.PeerUser:
		if msg.Out {
			result.ChatID = int64(p.UserID)
		} else {
			result.ChatID = int64(p.UserID)
		}
	case *tg.PeerChat:
		result.ChatID = int64(-p.ChatID)
	case *tg.PeerChannel:
		result.ChatID = int64(p.ChannelID)
	}

	// Sender
	if msg.FromID != nil {
		switch from := msg.FromID.(type) {
		case *tg.PeerUser:
			result.senderID = from.UserID
			if from.UserID == tc.selfID {
				result.IsOutgoing = true
			}
			// Try resolving from provided users
			for _, u := range users {
				if user, ok := u.(*tg.User); ok && user.ID == from.UserID {
					result.SenderName = userName(user)
					break
				}
			}
		}
	}

	return result, true
}

// --- Helpers ---

func extractText(msg *tg.Message) string {
	if msg.Message != "" {
		return msg.Message
	}
	if msg.Media != nil {
		switch msg.Media.(type) {
		case *tg.MessageMediaPhoto:
			return "[Photo]"
		case *tg.MessageMediaDocument:
			return "[File]"
		case *tg.MessageMediaGeo:
			return "[Location]"
		case *tg.MessageMediaContact:
			return "[Contact]"
		}
		return "[Media]"
	}
	return ""
}

func userName(u *tg.User) string {
	if u.LastName != "" {
		return u.FirstName + " " + u.LastName
	}
	return u.FirstName
}

func int64Slice(ids []int) []int64 {
	result := make([]int64, len(ids))
	for i, id := range ids {
		result[i] = int64(id)
	}
	return result
}
