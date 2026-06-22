package main

import (
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
