package main

import (
	"fmt"
	"os"

	tea "charm.land/bubbletea/v2"
)

var version = "dev"

func main() {
	cfgPath := ""
	for i, arg := range os.Args[1:] {
		switch arg {
		case "--version", "-v":
			fmt.Printf("kfs-tg %s\n", version)
			return
		case "--config", "-c":
			if i+1 < len(os.Args)-1 {
				cfgPath = os.Args[i+2]
			}
		}
	}

	cfg, err := LoadConfig(cfgPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "config error: %v\n", err)
		os.Exit(1)
	}

	m := NewModel(cfg)

	p := tea.NewProgram(m)
	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
