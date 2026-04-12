package backend

import (
	"context"
	"database/sql"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sync"

	"github.com/wailsapp/wails/v2/pkg/runtime"
	_ "modernc.org/sqlite"
)

type App struct {
	ctx                      context.Context
	db                       *sql.DB
	dataDir                  string
	solverFS                 fs.FS
	closeMutex               sync.Mutex
	isClosingUnconditionally bool
}

func NewApp(solverFS fs.FS) *App {
	return &App{solverFS: solverFS}
}

func (a *App) Startup(ctx context.Context) {
	a.ctx = ctx

	dataDir, err := resolveAppDataDir()
	if err != nil {
		panic(fmt.Errorf("resolve app data dir: %w", err))
	}
	if err := os.MkdirAll(dataDir, 0o755); err != nil {
		panic(fmt.Errorf("create app data dir: %w", err))
	}

	db, err := openDatabase(filepath.Join(dataDir, "course_scheduler.db"))
	if err != nil {
		panic(fmt.Errorf("open database: %w", err))
	}
	if err := initDatabase(db); err != nil {
		panic(fmt.Errorf("init database: %w", err))
	}
	if err := migrateTeacherCampuses(db); err != nil {
		panic(fmt.Errorf("migrate teacher campuses: %w", err))
	}

	a.dataDir = dataDir
	a.db = db
}

func (a *App) Shutdown(ctx context.Context) {
	if a.db != nil {
		_ = a.db.Close()
	}
}

func (a *App) BeforeClose(ctx context.Context) bool {
	a.closeMutex.Lock()
	defer a.closeMutex.Unlock()

	if a.isClosingUnconditionally {
		return false
	}

	runtime.EventsEmit(ctx, "show-close-dialog")
	return true
}

func (a *App) OpenDialog(options OpenDialogOptions) (string, error) {
	defaultDirectory, defaultFilename := splitDefaultPath(options.DefaultPath)
	filters := make([]runtime.FileFilter, 0, len(options.Filters))
	for _, filter := range options.Filters {
		filters = append(filters, runtime.FileFilter{
			DisplayName: filter.DisplayName,
			Pattern:     filter.Pattern,
		})
	}

	dialogOptions := runtime.OpenDialogOptions{
		Title:            options.Title,
		DefaultDirectory: defaultDirectory,
		DefaultFilename:  defaultFilename,
		Filters:          filters,
	}

	if options.Multiple {
		paths, err := runtime.OpenMultipleFilesDialog(a.ctx, dialogOptions)
		if err != nil {
			return "", err
		}
		if len(paths) == 0 {
			return "", nil
		}
		return paths[0], nil
	}

	return runtime.OpenFileDialog(a.ctx, dialogOptions)
}

func (a *App) SaveDialog(options SaveDialogOptions) (string, error) {
	defaultDirectory, defaultFilename := splitDefaultPath(options.DefaultPath)
	filters := make([]runtime.FileFilter, 0, len(options.Filters))
	for _, filter := range options.Filters {
		filters = append(filters, runtime.FileFilter{
			DisplayName: filter.DisplayName,
			Pattern:     filter.Pattern,
		})
	}

	return runtime.SaveFileDialog(a.ctx, runtime.SaveDialogOptions{
		Title:            options.Title,
		DefaultDirectory: defaultDirectory,
		DefaultFilename:  defaultFilename,
		Filters:          filters,
	})
}

func resolveAppDataDir() (string, error) {
	baseDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(baseDir, "com.tsxb.course-scheduler"), nil
}

func openDatabase(dbPath string) (*sql.DB, error) {
	database, err := sql.Open("sqlite", dbPath)
	if err != nil {
		return nil, err
	}
	database.SetMaxOpenConns(1)
	if _, err := database.Exec("PRAGMA foreign_keys = ON"); err != nil {
		_ = database.Close()
		return nil, err
	}
	if _, err := database.Exec("PRAGMA busy_timeout = 5000"); err != nil {
		_ = database.Close()
		return nil, err
	}
	if err := database.Ping(); err != nil {
		_ = database.Close()
		return nil, err
	}
	return database, nil
}

func splitDefaultPath(defaultPath string) (string, string) {
	if defaultPath == "" {
		return "", ""
	}

	cleanedPath := filepath.Clean(defaultPath)
	if filepath.IsAbs(cleanedPath) || filepath.Dir(cleanedPath) != "." {
		return filepath.Dir(cleanedPath), filepath.Base(cleanedPath)
	}

	return "", cleanedPath
}
