package main

import (
	"embed"
	"log"

	"course-scheduler/internal/backend"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	windowsoptions "github.com/wailsapp/wails/v2/pkg/options/windows"
)

//go:embed all:frontend/dist
var frontendAssets embed.FS

//go:embed all:solver/dist
var solverAssets embed.FS

func main() {
	app := backend.NewApp(solverAssets)

	err := wails.Run(&options.App{
		Title:            "course-scheduler",
		Width:            800,
		Height:           600,
		MinWidth:         800,
		MinHeight:        600,
		Frameless:        true,
		BackgroundColour: &options.RGBA{R: 0, G: 0, B: 0, A: 0},
		AssetServer: &assetserver.Options{
			Assets: frontendAssets,
		},
		OnStartup:     app.Startup,
		OnShutdown:    app.Shutdown,
		OnBeforeClose: app.BeforeClose,
		Bind: []any{
			app,
		},
		Windows: &windowsoptions.Options{
			WebviewIsTransparent: true,
			WindowIsTranslucent:  true,
		},
	})
	if err != nil {
		log.Fatal(err)
	}
}
