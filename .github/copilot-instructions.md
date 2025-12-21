# Course Scheduler - AI Coding Agent Instructions

## Architecture & Data Flow
- **Stack**: Tauri (Rust) + Vue 3 + Python (OR-Tools) + SQLite.
- **Dual-Table Pattern**: All edits are saved to `*_temp` tables via 100ms debounced auto-save in `src/stores/data.js`.
- **Commit/Revert**: Users must explicitly commit (copy temp → main) or revert (clear temp).
- **Solver**: Sidecar binary (`solver/solver.py`). Rust pipes JSON to/from it. **DO NOT modify solver logic on main branch.**

## Critical Workflows
- **Build Solver**: `npm run build:solver` (uses `uv` + PyInstaller) is required before `tauri dev/build`.
- **Database**: Located in `%APPDATA%\Roaming\com.tsxb.course-scheduler\course_scheduler.db`.
- **Logs**: Located in `%APPDATA%\Roaming\com.tsxb.course-scheduler\logs\`.

## Conventions & Constraints
- **Platform**: Windows 10+ ONLY. Ignore macOS/Linux compatibility.
- **IDs**: Use `uuid.v4()` for all entity IDs in JS.
- **UI**: Naive UI components. Custom titlebar in `src/layouts/MainLayout.vue` (`decorations: false`).
- **State**: Pinia store (`src/stores/data.js`) is the single source of truth.
- **Venue Propagation**: Updating a course's venue automatically updates all teacher schedules (see `updateCourse` in `src/stores/data.js`).
- **Schedule Density**: This is a **manual constraint** (specified by staff), NOT an auto-calculated value. It defines the maximum number of classes allowed per campus per time slot.

## Key Files & Directories
- `src-tauri/src/db_handler.rs`: SQLite logic and temp-to-commit implementation.
- `src-tauri/src/models.rs`: Shared data structures (Rust).
- `src-tauri/schema.sql`: Database schema for both main and temp tables.
- `src/stores/data.js`: Frontend state, auto-save watcher, and business logic.
- `solver/solver.py`: Constraint solver (Collaborator only).

## Database Schema
- **Main tables**: `teachers`, `courses`, `campuses`, `venues`, `time_slots`, `days`, `scheduled_classes`, `teacher_courses`, `course_venues`, `teacher_unavailability`, `schedule_density`.
- **Temp tables**: Identical structure with `_temp` suffix.
- **Foreign Keys**: Enabled via `PRAGMA foreign_keys = ON` and `ON DELETE CASCADE`.

## Tauri Commands
- `load_data()`: Loads temp if exists, else main tables.
- `save_temp_data(content)`: Writes current state to temp tables.
- `commit_data()`: Copies temp tables to main tables.
- `clear_temp_data()`: Deletes all temp table data.
- `run_solver()`: Executes Python sidecar and returns solved data.
