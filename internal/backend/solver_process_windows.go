package backend

import (
	"os/exec"
	"syscall"
)

func configureSolverCommand(command *exec.Cmd) {
	command.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
}
