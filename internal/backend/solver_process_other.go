//go:build !windows

package backend

import "os/exec"

func configureSolverCommand(command *exec.Cmd) {}
