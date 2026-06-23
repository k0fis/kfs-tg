package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
)

type Config struct {
	General GeneralConfig `toml:"general"`
	UI      UIConfig      `toml:"ui"`
}

type GeneralConfig struct {
	ApiID   int    `toml:"api_id"`
	ApiHash string `toml:"api_hash"`
}

// Compile-time defaults (injected via -ldflags "-X main.defaultApiID=..." in CI)
var (
	defaultApiID   = ""
	defaultApiHash = ""
)

func (g *GeneralConfig) EffectiveApiID() int {
	if g.ApiID != 0 {
		return g.ApiID
	}
	if defaultApiID != "" {
		var id int
		fmt.Sscanf(defaultApiID, "%d", &id)
		return id
	}
	if v := os.Getenv("KFS_TG_API_ID"); v != "" {
		var id int
		fmt.Sscanf(v, "%d", &id)
		return id
	}
	return 0
}

func (g *GeneralConfig) EffectiveApiHash() string {
	if g.ApiHash != "" {
		return g.ApiHash
	}
	if defaultApiHash != "" {
		return defaultApiHash
	}
	return os.Getenv("KFS_TG_API_HASH")
}

type UIConfig struct {
	ChatListWidth  int  `toml:"chat_list_width"`
	ShowTimestamps bool `toml:"show_timestamps"`
	Notifications  bool `toml:"notifications"`
}

func LoadConfig(path string) (*Config, error) {
	cfg := &Config{
		UI: UIConfig{
			ChatListWidth:  20,
			ShowTimestamps: true,
			Notifications:  true,
		},
	}

	if path == "" {
		path = defaultConfigPath()
	}

	if _, err := os.Stat(path); err != nil {
		return cfg, nil // no config file = use defaults
	}

	if _, err := toml.DecodeFile(path, cfg); err != nil {
		return nil, err
	}

	return cfg, nil
}

func defaultConfigPath() string {
	dir, _ := os.UserConfigDir()
	return filepath.Join(dir, "kfs-tg", "config.toml")
}

func DataDir() string {
	dir, _ := os.UserCacheDir()
	p := filepath.Join(dir, "kfs-tg")
	os.MkdirAll(p, 0700)
	return p
}
