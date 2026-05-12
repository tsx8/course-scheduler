package backend

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	goruntime "runtime"

	wailsruntime "github.com/wailsapp/wails/v2/pkg/runtime"
)

func (a *App) RunSolver() (AllData, error) {
	currentData, err := a.LoadData()
	if err != nil {
		return AllData{}, err
	}

	solverInput := currentData
	stagedSchedules := make([]ScheduledClass, 0)
	lockedScheduleIDs := make(map[string]struct{})
	lockedActiveSchedules := make([]ScheduledClass, 0)
	for _, schedule := range currentData.ScheduledClasses {
		if schedule.IsStaged {
			stagedSchedules = append(stagedSchedules, schedule)
			continue
		}
		if schedule.IsLocked {
			schedule.IsStaged = false
			schedule.StagedOrder = 0
			lockedActiveSchedules = append(lockedActiveSchedules, schedule)
			lockedScheduleIDs[schedule.ID] = struct{}{}
		}
	}
	solverInput.ScheduledClasses = lockedActiveSchedules

	inputPath := filepath.Join(a.dataDir, "solver_input.tmp.json")
	outputPath := filepath.Join(a.dataDir, "solver_output.tmp.json")
	inputContent, err := json.Marshal(solverInput)
	if err != nil {
		return AllData{}, fmt.Errorf("marshal solver input: %w", err)
	}
	if err := os.WriteFile(inputPath, inputContent, 0o644); err != nil {
		return AllData{}, fmt.Errorf("write solver input: %w", err)
	}
	defer func() { _ = os.Remove(inputPath) }()
	defer func() { _ = os.Remove(outputPath) }()

	solverPath, err := a.resolveSolverPath()
	if err != nil {
		return AllData{}, err
	}

	command := exec.Command(solverPath, inputPath, outputPath)
	configureSolverCommand(command)
	output, err := command.CombinedOutput()
	if err != nil {
		return AllData{}, fmt.Errorf("solver failed: %w\n%s", err, string(output))
	}

	content, err := os.ReadFile(outputPath)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return AllData{}, fmt.Errorf("solver completed without writing output\n%s", string(output))
		}
		return AllData{}, fmt.Errorf("read solver output: %w", err)
	}

	var solvedData AllData
	if err := json.Unmarshal(content, &solvedData); err != nil {
		return AllData{}, fmt.Errorf("parse solver output: %w", err)
	}
	for index := range solvedData.ScheduledClasses {
		if _, ok := lockedScheduleIDs[solvedData.ScheduledClasses[index].ID]; ok {
			solvedData.ScheduledClasses[index].IsLocked = true
		} else {
			solvedData.ScheduledClasses[index].IsLocked = false
		}
		solvedData.ScheduledClasses[index].IsStaged = false
		solvedData.ScheduledClasses[index].StagedOrder = 0
	}
	solvedData.ScheduledClasses = append(solvedData.ScheduledClasses, stagedSchedules...)
	if err := a.SaveTempData(solvedData); err != nil {
		return AllData{}, err
	}

	return solvedData, nil
}

func (a *App) FinalizeAndClose(save bool) error {
	hasChanges, err := a.HasUnsavedChanges()
	if err != nil {
		return err
	}

	if save {
		if err := a.CommitData(); err != nil {
			return err
		}
	} else if hasChanges {
		if err := a.ClearTempData(); err != nil {
			return err
		}
	}

	a.closeMutex.Lock()
	a.isClosingUnconditionally = true
	a.closeMutex.Unlock()

	runtimeWailsQuit(a)
	return nil
}

func (a *App) resolveSolverPath() (string, error) {
	binaryName := solverBinaryName()
	diskPath := filepath.Join("solver", "dist", binaryName)
	if info, err := os.Stat(diskPath); err == nil && !info.IsDir() {
		absolutePath, absErr := filepath.Abs(diskPath)
		if absErr == nil {
			return absolutePath, nil
		}
		return diskPath, nil
	}

	if a.solverFS == nil {
		return "", fmt.Errorf("solver binary missing; run uv --directory solver run pyinstaller solver.spec first")
	}

	embeddedPath := filepath.ToSlash(filepath.Join("solver", "dist", binaryName))
	reader, err := a.solverFS.Open(embeddedPath)
	if err != nil {
		return "", fmt.Errorf("solver binary missing; run uv --directory solver run pyinstaller solver.spec first")
	}
	defer func() { _ = reader.Close() }()

	targetPath := filepath.Join(a.dataDir, binaryName)
	writer, err := os.Create(targetPath)
	if err != nil {
		return "", fmt.Errorf("create extracted solver: %w", err)
	}
	defer func() { _ = writer.Close() }()

	if _, err := io.Copy(writer, reader); err != nil {
		return "", fmt.Errorf("extract solver binary: %w", err)
	}
	if err := writer.Close(); err != nil {
		return "", fmt.Errorf("close extracted solver: %w", err)
	}
	writer = nil
	if err := os.Chmod(targetPath, 0o755); err != nil && goruntime.GOOS != "windows" {
		return "", fmt.Errorf("set solver permissions: %w", err)
	}

	return targetPath, nil
}

func solverBinaryName() string {
	if goruntime.GOOS == "windows" {
		return "solver.exe"
	}
	return "solver"
}

func runtimeWailsQuit(app *App) {
	if app != nil && app.ctx != nil {
		wailsruntime.Quit(app.ctx)
	}
}
