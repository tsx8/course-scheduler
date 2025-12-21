# Course Scheduler - AI Coding Agent Instructions

## Architecture Overview

This is a **Tauri desktop app** with a hybrid multi-language stack:
- **Frontend**: Vue 3 + Vite + Naive UI + Pinia (state management)
- **Backend**: Rust (Tauri framework for native OS integration)
- **Solver**: Python (OR-Tools CP-SAT) compiled to standalone binary via PyInstaller
- **Database**: SQLite (embedded, zero-config persistence)

### Key Architectural Pattern: "Optimistic Temp-to-Commit" with Dual Table Schema
All edits are saved to **temporary tables** (`*_temp` suffix in SQLite) via auto-save watcher in `src/stores/data.js` (100ms debounce). The user explicitly commits/reverts via UI actions:
- **Main tables**: Committed, permanent data (e.g., `teachers`, `courses`, `campuses`)
- **Temp tables**: Working state (e.g., `teachers_temp`, `courses_temp`)
- **Auto-save**: Every edit → temp tables (see `save_temp_data` command)
- **Commit**: Copies all temp tables → main tables (see `commit_data` command)
- **Revert**: Clears temp tables, reloads from main tables (see `clear_temp_data`)

This enables:
- Instant undo without complex state management
- Safe experimentation with the constraint solver
- Clean separation between working state and committed state
- Transactional safety via SQLite ACID guarantees

## Project Scope & Constraints

### Platform Support
**Windows 10+ ONLY** - Despite Tauri 2's cross-platform capabilities, this project targets Windows exclusively for optimal GUI experience. Do not invest time in macOS/Linux compatibility.

### Solver Development
**OFF-LIMITS on main branch** - All solver improvements (`solver/solver.py`, constraint logic, optimization weights) are handled by a dedicated collaborator. Do not modify solver code unless explicitly working on a solver-specific branch.

### Testing Strategy
**Manual testing only** - No automated test suite due to rapid development timeline. Focus on implementation and manual verification rather than test infrastructure.

## Critical Developer Workflows

### Building for Production
```powershell
npm run tauri build
```
This automatically:
1. Builds the Python solver via `npm run build:solver` (uses `uv` + PyInstaller)
2. Copies solver binary to `src-tauri/binaries/solver-<target>` via `scripts/prepare-solver.js`
3. Bundles Vite frontend + Rust backend + solver + SQLite into single executable

### Dev Environment Setup
1. Install Rust (via [rustup](https://rustup.rs/)), Node.js 18+, and Python 3.11+
2. Install `uv` (fast Python package manager): `pip install uv`
3. Run `npm install` for frontend deps
4. Run `uv sync` in project root (creates `.venv`, installs OR-Tools + PyInstaller from `pyproject.toml`)
5. Run `npm run build:solver` before first dev run (builds Python binary)
6. Run `npm run tauri dev` (auto-runs solver build + Vite dev server via `beforeDevCommand`)

### Database Location (Windows)
- **Production**: `%APPDATA%\Roaming\com.tsxb.course-scheduler\course_scheduler.db`
- **Logs**: `%APPDATA%\Roaming\com.tsxb.course-scheduler\logs\course-scheduler-YYYY.MM.DD.log`
- **First Launch**: Automatically migrates old `data.json` → SQLite, creates backup as `data.json.backup-YYYYMMDD-HHMMSS`

### Solver Development Cycle (Collaborator Only)
**Do not modify solver code on main branch** - this is handled by a dedicated collaborator.
- Solver code: `solver/solver.py` (OR-Tools constraint model)
- Rebuild process: `cd solver && uv run pyinstaller solver.spec && cd .. && node scripts/prepare-solver.js`
- Test trigger: "自动排课" (Auto Schedule) button in UI
- I/O: Solver uses JSON (Rust serializes DB → JSON → solver → JSON result → DB)

## Data Model (Shared JSON Schema)

The entire app revolves around `AllData` structure defined in **both** `src-tauri/src/models.rs` (Rust) and consumed by `solver/solver.py` (Python). Key entities:

```javascript
{
  "time": [{id, value, corresponding_hours}],        // Time slots (e.g., "8:00-10:00")
  "day": [{id, value}],                              // Days of week
  "campuses": [{id, name, venues[], schedule_density[]}],
  "courses": [{id, name, place: [{campus_id, venue_id}]}],
  "teachers": [{
    id, name, max_teaching_hours, is_only_shahe,
    teaches: [course_ids],
    scheduled: [{id, day_id, time_id, course_id, campus_id, venue_id}],
    unavailable: [{day_id, time_id}]
  }]
}
```

### Key Constraint: Two-Campus Rule
- Teachers with `is_only_shahe: true` can only teach at Shahe campus (hardcoded ID: `138697dc-1591-4c16-b60e-d0057964be56`)
- Others **must** teach at **both** campuses (hard constraint in solver, lines 180-195)
- Solver enforces: teachers can only be at one campus per day (venue-to-campus mapping)
- Teachers already scheduled at 4+ different days cannot receive new days (hard cap)
- **Note**: Campus ID hardcoding is planned for future improvement but not a priority - solver changes are collaborator's responsibility

## Python Solver Integration

### Invocation Flow
1. User clicks "自动排课" → `MainLayout.vue` calls `invoke('run_solver')`
2. Rust (`src-tauri/src/main.rs:run_solver`) spawns solver as sidecar process
3. Solver reads input JSON, runs CP-SAT, writes output JSON
4. Rust parses output, writes to temp tables via `db_handler.rs`, returns to frontend
5. Frontend (`data.js`) replaces store state with solver results

### Solver Architecture (`solver/solver.py`)
- **DataManager** (lines 8-94): Preprocesses input, creates ID-to-index maps, calculates existing schedule metrics
- **Hard Constraints** (lines 116-208): Teachers can't be in two places, venues have capacity, max hours, campus rules
- **Soft Constraints** (lines 210-340): Minimize single-class days, schedule gaps, scattered workdays; maximize course openings
- **Objective Function** (line 422): Multi-weighted optimization balancing 6 factors

### PyInstaller Bundle Requirements
- `solver.spec` manually includes OR-Tools binaries (`_pywrapcp.pyd`, `.libs`)
- Target binary name pattern: `solver-<rust_target>` (e.g., `solver-x86_64-pc-windows-msvc.exe`)
- Deployed to Tauri via `externalBin` in `tauri.conf.json`

## State Management Patterns

### Pinia Store (`src/stores/data.js`)
- **Single source of truth**: `teachers`, `courses`, `campuses`, `time`, `day` refs
- **Auto-save**: Deep watcher triggers `debouncedSave` (100ms) on any mutation
- **Load**: `invoke('load_data')` fetches from SQLite (temp tables if exist, else main tables)
- **Revert**: `revertChanges()` clears temp tables, reloads from main tables
- **Commit**: `commitChanges()` copies temp tables → main tables
- **Event system**: Listens to `commit-completed` event from Rust backend after successful commit
- **Solving state**: `isSolving` ref prevents UI changes during solver execution

### Critical: Venue Update Propagation
When a course's venue changes (lines 150-210 in `data.js`), the store automatically updates all teacher schedules with "smart replacement" logic:
- If a course had venue A at campus X and now has venue B at campus X
- All scheduled classes at (course, campus X, venue A) → venue B
- This prevents orphaned schedule references

## UI/Component Conventions

### Page Structure
All pages in `src/pages/` follow this pattern:
- NDataTable with pagination for list views
- Modal forms for add/edit operations
- Delete requires dialog confirmation
- Icons from `@vicons/ionicons5`

### Custom Window Chrome
`decorations: false` in `tauri.conf.json` → custom titlebar in `MainLayout.vue` with minimize/maximize/close buttons (uses Tauri Window API)

## Tauri Commands Reference

**Database Operations** (`src-tauri/src/db_handler.rs`):
- `load_data()`: Loads temp if exists, else main tables
- `save_temp_data(content)`: Debounced auto-save target (writes to temp tables)
- `commit_data()`: Copy temp tables → main tables
- `clear_temp_data()`: Delete all temp table data (for revert)

**Import/Export** (`src-tauri/src/import_export.rs`):
- `import_json(file_path)`: Load external JSON into temp tables (user reviews then commits)
- `export_json(file_path)`: Export current state to JSON file
- `import_database(file_path)`: Import from another SQLite database into temp tables
- `export_database(file_path)`: Copy entire database file (main + temp tables)

**Solver** (`src-tauri/src/main.rs`):
- `run_solver()`: Spawns Python binary as sidecar, pipes JSON I/O, returns solved AllData
- `finalize_and_close(save)`: Commit/discard on app exit (called from close dialog in `MainLayout.vue`)

**Window Management**:
- Custom titlebar (via `decorations: false`) with minimize/maximize/close in `MainLayout.vue`
- Close button triggers `show-close-dialog` event → displays save prompt → calls `finalize_and_close`

**Utilities**:
- `open_logs_folder()`: Opens log directory in file explorer

## Database Schema Reference

SQLite database uses **dual-table pattern** with main and temp tables:
- **Main tables**: `teachers`, `courses`, `campuses`, `venues`, `time_slots`, `days`, `scheduled_classes`, `teacher_courses`, `course_venues`, `teacher_unavailability`, `schedule_density`
- **Temp tables**: Identical structure with `_temp` suffix
- **Foreign keys**: All relationships enforced with `ON DELETE CASCADE`
- **Schema file**: `src-tauri/schema.sql` - executed on first run to initialize database

Key tables:
- `teachers`: Core teacher info with `is_only_shahe` flag
- `scheduled_classes`: Stores scheduled lessons (teacher, course, day, time, campus, venue)
- `teacher_unavailability`: Stores blackout periods (teacher, day, time)
- `course_venues`: Junction table linking courses to allowed venues
- `schedule_density`: Campus schedule load tracking (campus, day, time, count)

## Import/Export Features

All import operations write to **temp tables only** - user must commit changes:
1. **JSON Import**: Parses `AllData` JSON → writes to temp tables → user reviews → commit
2. **Database Import**: Opens external `.db` file → validates schema → copies to temp tables
3. **JSON Export**: Queries current state → serializes to `AllData` JSON → writes file
4. **Database Export**: Copies entire `.db` file (includes both main and temp tables)

Import/export commands defined in `src-tauri/src/import_export.rs` and invoked from `src/pages/Settings.vue`.

## Migration from JSON (First Launch)

On first run, if database is empty and `data.json` exists:
1. Check `%APPDATA%\Roaming\com.tsxb.course-scheduler\data.json` first
2. Fall back to old location `%APPDATA%\Local\course-scheduler\data.json`
3. Parse JSON → insert into **main tables** (not temp) via `migrate_from_json()`
4. Backup original as `data.json.backup-YYYYMMDD-HHMMSS` in new AppData location
5. Clear temp tables to ensure clean state

## Common Gotchas

- **Platform-specific**: Windows 10+ only - don't add cross-platform code or test on macOS/Linux
- **Solver is off-limits**: Never modify `solver/solver.py` or constraint logic on main branch - collaborator's responsibility
- **No automated tests**: Manual testing only - don't create test suites or CI/CD test pipelines
- **ID consistency**: All IDs are UUIDs generated via `uuid.v4()` in JS, must match across entities
- **Build order**: Always `build:solver` before `tauri build/dev` or sidecar will fail
- **Database locks**: SQLite uses `Mutex<Connection>` - avoid long-running operations in Tauri commands
- **Foreign key constraints**: Enabled in `schema.sql` - deletions cascade automatically
- **PyInstaller caching**: Delete `solver/build/` and `solver/dist/` if solver changes don't take effect
- **Event listeners**: Set up in `data.js` after initialization - ensure `listen()` calls don't duplicate on hot reload

## File Ownership Guide

- **Solver logic**: **OFF-LIMITS** - `solver/solver.py` and related files are collaborator's domain, do not modify on main
- **Database schema**: Update BOTH `src-tauri/schema.sql` AND `src-tauri/src/db_handler.rs` serialization logic
- **Data model changes**: Update `src-tauri/src/models.rs` (solver model sync is collaborator's job)
- **UI pages**: Independent, no shared state beyond Pinia store
- **Tauri commands**: Add to `db_handler.rs`, `import_export.rs`, or `main.rs`, register in `main.rs`

## Logging System

Application uses `tracing` + `tracing-subscriber` with custom log rotation:
- **Location**: `%APPDATA%\Roaming\com.tsxb.course-scheduler\logs\`
- **Format**: `course-scheduler-YYYY.MM.DD.log` (one file per day)
- **Retention**: Keeps minimum 3 most recent files, deletes files older than 30 days
- **Reopening**: Custom `ReopenableLogWriter` detects file deletion and recreates automatically
- **Usage**: Import `tracing::{info, error, warn}` in Rust modules for structured logging
- **Control**: Set `RUST_LOG` environment variable (e.g., `RUST_LOG=debug`) for verbosity
