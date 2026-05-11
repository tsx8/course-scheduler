set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

alias b := build
alias d := dev
alias f := format
alias l := lint
alias p := package

# List available project commands.
default:
    @& "{{ just_executable() }}" --list

# Install or upgrade just through uv.
install-just:
    uv tool install --upgrade rust-just

# Install local development dependencies.
install: install-frontend sync-solver

# Install dependencies exactly as CI does.
install-ci: install-frontend-ci sync-solver-frozen

# Install frontend dependencies.
install-frontend:
    npm --prefix frontend install

# Install frontend dependencies from the lockfile.
install-frontend-ci:
    npm --prefix frontend ci

# Sync solver dependencies, including development tools.
sync-solver:
    uv --directory solver sync --all-groups

# Sync solver dependencies from the lockfile.
sync-solver-frozen:
    uv --directory solver sync --frozen --all-groups

# Run all configured linters and format checks.
lint: lint-frontend lint-go lint-solver check-solver-format

# Lint frontend code and checked config files.
lint-frontend:
    npm --prefix frontend run lint

# Lint Go code with the version pinned in go.mod.
lint-go:
    go tool golangci-lint run

# Lint solver Python code.
lint-solver:
    uv --directory solver run ruff check .

# Check solver Python formatting.
check-solver-format:
    uv --directory solver run ruff format --check .

# Format frontend/config, Go, and solver files.
format: format-frontend format-go format-solver

# Format frontend-managed files.
format-frontend:
    npm --prefix frontend run format

# Format Go packages.
format-go:
    go fmt ./...

# Format solver Python files.
format-solver:
    uv --directory solver run ruff format .

# Build frontend, solver sidecar, and Go backend.
build: build-frontend build-solver build-go

# Build the frontend bundle.
build-frontend:
    npm --prefix frontend run build

# Build the Python solver sidecar.
build-solver:
    uv --directory solver run pyinstaller solver.spec

# Build the Go backend.
build-go:
    go build ./...

# Run the Wails development app.
dev:
    wails dev

# Build the Windows installer.
package:
    wails build -nsis

# Build the unsigned macOS Apple Silicon app bundle.
package-macos:
    wails build -platform darwin/arm64

# Run the normal local verification suite.
verify: lint build
