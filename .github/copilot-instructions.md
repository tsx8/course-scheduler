# Course Scheduler - AI Coding Agent Instructions

## Architecture Overview

**Tech Stack**: Tauri 2 (Rust backend) + Vue 3 (frontend) + Python solver  
**Platform**: Windows-only desktop application  
**Domain**: Automated course scheduling using constraint satisfaction (Google OR-Tools)

### Three-Layer Architecture

1. **Frontend (Vue 3)**: SPA with Naive UI components, Pinia stores, auto-save every 100ms
2. **Backend (Rust/Tauri)**: SQLite persistence, RBAC auth, audit logging, subprocess management
3. **Solver (Python)**: Standalone CP-SAT constraint solver binary (PyInstaller-packaged)

## Critical Data Flow: Optimistic Temp-to-Commit Pattern

**Dual-table SQLite architecture** - every entity has main + temp tables:
- Edit in UI → Auto-save to `*_temp` tables (100ms debounce in `src/stores/data.js`)
- User clicks "提交更改" → `commit_data` Rust command → Copy temp → main tables
- User clicks "撤销更改" → `clear_temp_data` → Reload from main tables

**Example**: `teachers` (committed) vs `teachers_temp` (working state)

```rust
// src-tauri/src/db_handler.rs
pub fn commit_data(conn: &Connection) -> CommandResult<()> {
    // Transaction: copy all *_temp tables to main tables
    // ...14 table pairs synchronized
}
```

**Why this matters**: Always check if changes go to temp or main tables. Solver writes to temp. Frontend reads merged view (main + temp overrides).

## Key Development Workflows

### Build and Run
```bash
# One-time setup
npm install && uv sync

# Build solver binary (required before dev/build)
npm run build:solver  # Runs: uv pyinstaller + prepare-solver.js

# Development
npm run tauri dev      # Vite dev server + Rust hot-reload

# Production
npm run tauri build    # Output: src-tauri/target/release/bundle/nsis/
```

### Solver Integration
The Python solver is a **sidecar binary** (not a library):
- Build: `solver/solver.spec` → PyInstaller → `scripts/prepare-solver.js` → `src-tauri/binaries/solver-{target}.exe`
- Runtime: Tauri spawns subprocess via `shell` plugin, pipes JSON via stdin/stdout
- See `src-tauri/src/main.rs` for invocation pattern

```rust
// DO NOT call solver as library - it's a spawned process
sidecar.args(&["--input", &json_str])
       .current_dir(&temp_dir)
       .spawn()?
```

### Database Migrations
Schema is in `src-tauri/schema.sql` - executed on first run:
```rust
// src-tauri/src/db_handler.rs
pub fn init_database(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
}
```

**Adding tables**: Edit schema.sql → Both main + temp variants → Update `commit_data`/`clear_temp_data` logic

## Project-Specific Conventions

### Rust Backend Patterns

**Command Structure** (see `src-tauri/src/main.rs`):
```rust
#[command]
fn my_command(state: State<AppState>) -> Result<T, String> {
    let db = state.db.lock().unwrap();  // Mutex guard
    // ... db operations in transaction
    Ok(result)
}
```

**Audit Logging** (Feature: 001-rbac-audit-system):
- Every state-changing operation must call `audit::create_audit_log_entry`.
- **Dual-table Audit**: Audit logs also follow the temp-to-commit pattern. Logs for uncommitted changes are stored in `audit_logs_temp` and moved to `audit_logs` upon `commit_data`.
- **Rich Tracking**: Frontend uses `jsComputeDiff` in `src/stores/data.js` to generate detailed field-level changes for audit logs.
- Frontend calls `record_audit_log` command after mutations.
- Audit failures are logged but DO NOT block operations

**Error Handling**:
- Return `Result<T, String>` for Tauri commands (String = user-facing message)
- Use `tracing::error!` for internal errors, `warn!` for recoverable issues
- Frontend displays error strings via `message.error()`

### Frontend Patterns

**State Management** (Pinia stores):
- `src/stores/data.js`: ALL application data + auto-save logic
- `src/stores/auth.js`: Session, roles (Scheduler vs Teacher), RBAC checks

**RBAC Guards** in `src/router/index.js`:
```javascript
meta: { requiresScheduler: true }  // Blocks Teacher role
```

**Tauri IPC** - Always use `invoke()` from `@tauri-apps/api/core`:
```javascript
const result = await invoke('command_name', { param: value });
```

**Auto-Import**: Naive UI composables (`useMessage`, `useDialog`) and Vue APIs auto-imported via unplugin-auto-import

### Naming Conventions

- **Tables**: Snake_case (e.g., `teacher_courses`, `schedule_density`)
- **Rust**: Snake_case functions, PascalCase structs/enums
- **JavaScript**: camelCase variables, PascalCase components
- **Commands**: Rust snake_case → JS camelCase (e.g., `load_data` → `invoke('load_data')`)

## Integration Points

### Solver Input/Output Format
See `solver/solver.py` for JSON schema (Strict 3NF Structure):
```json
{
  "time": [{"id": "uuid", "value": "周一1-2节", "corresponding_hours": 2}],
  "day": [{"id": "uuid", "value": "周一"}],
  "campuses": [{"id": "uuid", "name": "沙河校区"}],
  "venues": [{"id": "uuid", "campus_id": "...", "name": "..."}],
  "courses": [{"id": "uuid", "name": "..."}],
  "teachers": [{"id": "uuid", "name": "...", "max_teaching_hours": 10}],
  "course_venues": [{"course_id": "...", "venue_id": "..."}],
  "teacher_courses": [{"teacher_id": "...", "course_id": "..."}],
  "teacher_unavailability": [{"teacher_id": "...", "day_id": "...", "time_id": "..."}],
  "scheduled_classes": [
    {"id": "...", "teacher_id": "...", "course_id": "...", "venue_id": "...", "day_id": "...", "time_id": "..."}
  ],
  "schedule_density": [{"campus_id": "...", "day_id": "...", "time_id": "...", "count": 3}]
}
```

**Output**: Array of `scheduled_classes` entries written to temp tables.

### File Locations (Windows)
- Database: `%APPDATA%\Roaming\com.tsxb.course-scheduler\course_scheduler.db`
- Logs: `%APPDATA%\Roaming\com.tsxb.course-scheduler\logs\` (daily rotation, 30-day retention)
- Test with: `echo $env:APPDATA` in PowerShell

### External Dependencies
- **OR-Tools**: Bundled in solver binary (PyInstaller collects `.pyd` via `solver/solver.spec`)
- **SQLite**: Embedded via `rusqlite` (bundled feature)
- **Naive UI**: Auto-imported components via unplugin-vue-components

## Common Pitfalls

1. **Forgetting to rebuild solver**: Changes to solver.py won't apply until `npm run build:solver`
2. **Mixing temp/main tables**: Always verify which table a function reads/writes
3. **Session expiry**: Auth checks expect `sessionId` - restore from localStorage on mount
4. **Windows paths**: Use `PathBuf` and `std::path`, never hardcode `\` separators
5. **Foreign keys**: Enabled by default (`PRAGMA foreign_keys = ON`) - cascades matter
6. **Log file deletion**: App recreates logs if externally deleted (see `src-tauri/src/main.rs`)

## Testing Approach

**No automated tests currently** - manual testing workflow:
1. Run `npm run tauri dev`
2. Login as admin (default: `admin` / `Admin123!`)
3. Seed data via UI or import JSON
4. Test solver with "自动排课" button
5. Check logs in `%APPDATA%\...\logs\` for errors

**When adding features**: Manually verify temp/commit/revert cycle works correctly.

## Feature Flags and Current State

- ✅ SQLite migration complete (replaced file-based JSON)
- ✅ RBAC + Audit system (Feature: 001-rbac-audit-system)
- ✅ Normalized 3NF database schema
- ✅ Single-session enforcement (File lock on login via `fs2` replaced process mutex)
- ✅ Strict 3NF Data Format (Legacy JSON support removed)
- 🚧 Cross-platform NOT supported (Windows-only by design)

## Planned Improvements

### Session Management Refactoring

- **Current**: Session-based locking (file lock prevents concurrent logins)
- **Target**: Advanced per-account session control
- **Benefits**: Support multiple concurrent users, better scalability

### Export Functionality

- **Selective export**: Allow filtering data before export
- **Format expansion**: Add CSV/Excel output alongside SQLite/JSON`
- **User experience**: Progress indicators for large exports

## When Working on Features

1. **Database changes**: Update schema.sql → Migrate temp table logic → Test commit/revert
2. **UI changes**: Check if Scheduler vs Teacher permissions matter (see `src/stores/auth.js`)
3. **Solver changes**: Modify `solver/solver.py` → Rebuild → Test via UI
4. **RBAC changes**: Update audit logs + session handling + router guards
5. **Session refactor**: Consider multi-user concurrency and session timeout handling

## References

- Tauri docs: https://tauri.app/v2/
- OR-Tools CP-SAT: https://developers.google.com/optimization/cp/cp_solver
- Schema definition: `src-tauri/schema.sql`
- Main README: `README.md`
