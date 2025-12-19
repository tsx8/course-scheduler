# Course Scheduler - AI Coding Agent Instructions

## Architecture Overview

This is a **Tauri desktop app** with a hybrid multi-language stack:
- **Frontend**: Vue 3 + Vite + Naive UI + Pinia (state management)
- **Backend**: Rust (Tauri framework for native OS integration)
- **Solver**: Python (OR-Tools CP-SAT) compiled to standalone binary via PyInstaller

### Key Architectural Pattern: "Optimistic Temp-to-Commit"
All edits go to `data.tmp.json` (auto-saved via debounced watcher in `src/stores/data.js`). The user explicitly commits/reverts via UI actions. This enables:
- Instant undo without complex state management
- Safe experimentation with the constraint solver
- Clean separation between working state and committed state

## Critical Developer Workflows

### Building for Production
```powershell
npm run tauri build
```
This automatically:
1. Builds the Python solver via `npm run build:solver` (uses `uv` + PyInstaller)
2. Copies solver binary to `src-tauri/binaries/solver-<target>` via `scripts/prepare-solver.js`
3. Bundles Vite frontend + Rust backend + solver into single executable

### Dev Environment Setup
1. Install Rust, Node.js, and Python 3.11+
2. Install `uv` (Python package manager): `pip install uv`
3. Run `npm install` for frontend deps
4. Run `npm run build:solver` before first dev run
5. Run `npm run tauri dev` (auto-runs solver build + Vite dev server)

### Solver Development Cycle
- Edit `solver/solver.py` (OR-Tools constraint model)
- Rebuild: `cd solver && uv run pyinstaller solver.spec && cd .. && node scripts/prepare-solver.js`
- Test in app: trigger "自动排课" (Auto Schedule) button in UI

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

## Python Solver Integration

### Invocation Flow
1. User clicks "自动排课" → `MainLayout.vue` calls `invoke('run_solver')`
2. Rust (`src-tauri/src/main.rs:run_solver`) spawns solver as sidecar process
3. Solver reads input JSON, runs CP-SAT, writes output JSON
4. Rust parses output, writes to `data.tmp.json`, returns to frontend
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
- **Revert**: `revertChanges()` clears temp, reloads from `data.json`
- **Commit**: `commitChanges()` renames `data.tmp.json` → `data.json`

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

**File Operations** (`src-tauri/src/file_handler.rs`):
- `load_data()`: Loads temp if exists, else data.json
- `save_temp_data(content)`: Debounced auto-save target
- `commit_data()`: Rename temp → data
- `clear_temp_data()`: Delete temp (for revert)
- `import_data(file_path)`: Load external JSON into temp
- `export_data(file_path)`: Save current state to file

**Solver** (`src-tauri/src/main.rs`):
- `run_solver()`: Spawns Python binary, handles I/O, returns solved AllData
- `finalize_and_close(save)`: Commit/discard on app exit

## Testing the Solver

Manual test workflow:
1. Create test data via UI (teachers, courses, venues)
2. Click "自动排课" and wait for solution
3. Check console for solver stats (objective value, class assignments)
4. If no solution: relax constraints (increase max_teaching_hours, add venues, reduce unavailable slots)
5. Revert if unsatisfied, iterate on solver weights in `solver.py:422`

## Common Gotchas

- **Solver timeout**: 60s limit in `solver.py:437`. Increase for complex schedules.
- **OR-Tools binary paths**: Windows paths differ; `solver.spec` detects `.pyd` files.
- **ID consistency**: All IDs are UUIDs generated via `uuid.v4()` in JS, must match across entities.
- **Campus hardcoding**: Shahe campus ID is hardcoded in solver (line 172). Update if campus list changes.
- **Build order**: Always `build:solver` before `tauri build/dev` or sidecar will fail.

## File Ownership Guide

- **Solver logic**: Only modify `solver/solver.py`, never touch generated `build/` or `dist/`
- **Data schema changes**: Update BOTH `src-tauri/src/models.rs` AND `solver/solver.py` DataManager
- **UI pages**: Independent, no shared state beyond Pinia store
- **Tauri commands**: Add to `file_handler.rs` or `main.rs`, register in `main.rs:163`

## Next Steps: PostgreSQL Migration

**Current State**: All data persists in flat JSON files (`data.json` / `data.tmp.json`)

**Planned Migration**: Complete replacement of file-based storage with PostgreSQL database

### Critical Requirement
**Keep the `AllData` data model entirely unchanged** - the frontend, solver, and Rust types must continue to work with the exact same structure. Only the persistence layer changes.

### Migration Strategy
- **Schema Design**: Map `AllData` structure to relational tables:
  - Core tables: `teachers`, `courses`, `campuses`, `venues`, `time_slots`, `days`
  - Junction tables: `teacher_courses`, `course_venues`, `scheduled_classes`, `teacher_unavailable`
- **Temp/Commit Pattern**: Replace file operations with database transactions:
  - Option A: Use database transactions (rollback = revert, commit = commit)
  - Option B: Use session/staging tables that mirror main tables
- **Solver Integration**: Maintain JSON interface for solver:
  - Query DB → serialize to `AllData` JSON → pass to solver → parse result → update DB
  - Solver remains completely unaware of database, continues to work with JSON I/O

### Implementation Path
1. **Database Layer** (Rust):
   - Add `sqlx` or `diesel` to `src-tauri/Cargo.toml`
   - Create `src-tauri/src/db/` module with connection pool and queries
   - Implement serialization: DB rows → `AllData` struct (identical to current JSON deserialization)
2. **Replace File Commands**:
   - `load_data()`: Query all tables → assemble into `AllData`
   - `save_temp_data()`: Write to staging tables or begin transaction
   - `commit_data()`: Commit transaction or copy staging → main tables
   - `clear_temp_data()`: Rollback transaction or clear staging tables
   - Remove `import_data()`/`export_data()` - no longer needed
3. **Update `main.rs`**:
   - Initialize database connection pool in `setup()`
   - Manage `AppState` with DB pool instead of file paths
4. **No Frontend Changes**: `data.js` store continues to call same Tauri commands, receives identical `AllData` responses
