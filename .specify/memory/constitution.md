<!--
Sync Impact Report - Version 1.0.0
=========================================
Version Change: Initial → 1.0.0
Rationale: Initial constitution establishment for Course Scheduler project

Principles Defined:
- I. Data Model Integrity (NEW) - Core principle ensuring data model stability
- II. Standalone Deployment (NEW) - Zero-dependency end-user deployment
- III. User-Centric Operations (NEW) - GUI-first, non-technical user friendly
- IV. Solver Integration (NEW) - Python solver as embedded component
- V. Version Management & Revertibility (NEW) - Temp/commit workflow pattern

Templates Status:
✅ plan-template.md - Updated constitution check references
✅ spec-template.md - User story structure aligns with GUI operations
✅ tasks-template.md - Task organization supports feature independence

Follow-up Items: None - all placeholders resolved

Last Updated: 2025-12-19
-->

# Course Scheduler Constitution

## Core Principles

### I. Data Model Integrity (NON-NEGOTIABLE)

The data model (`AllData` structure) is the foundation of this project and MUST remain stable across all changes.

- **Immutable Contract**: The `AllData` schema (defined in `src-tauri/src/models.rs`) serves as the contract between frontend (Vue/Pinia), backend (Rust), and solver (Python)
- **Schema Changes Require Triple-Update**: Any modification to data entities (teachers, courses, campuses, venues, time slots, days, schedules) MUST be reflected in:
  1. Rust models (`src-tauri/src/models.rs`)
  2. Python solver dataclass (`solver/solver.py` DataManager)
  3. Frontend store types (`src/stores/data.js`)
- **Backward Compatibility**: When migrating persistence layers (e.g., JSON → PostgreSQL), the `AllData` interface MUST NOT change
- **Validation**: All data mutations must preserve referential integrity (course venues, teacher schedules, campus associations)

**Rationale**: The data model spans three languages and multiple subsystems; breaking changes cascade catastrophically across the entire application.

### II. Standalone Deployment

End users MUST be able to use the application without installing any external dependencies.

- **Single Executable**: The build output is a single installer/executable containing:
  - Tauri application (Rust backend + Vue frontend)
  - Embedded Python solver binary (compiled via PyInstaller)
  - All runtime dependencies bundled
- **No Runtime Prerequisites**: Users do NOT need Python, Node.js, or SQL databases installed
- **Embedded Solver**: The OR-Tools Python solver is deployed as a sidecar binary (managed via Tauri `externalBin` configuration)
- **File-Based Persistence**: Default storage uses JSON files (`data.json`, `data.tmp.json`) requiring no database setup

**Rationale**: Target users are academic affairs staff with minimal technical expertise; reducing friction is critical for adoption.

### III. User-Centric Operations

The application prioritizes GUI-based workflows for non-technical users.

- **Direct Manipulation**: Core operations (CRUD on courses, teachers, venues) use GUI controls (forms, drag-drop, clicks) instead of command-line or text editing
- **Visual Feedback**: Data views (timetables, schedules) use tables, calendars, and interactive components (Naive UI)
- **Forgiving UX**: The temp/commit pattern (Principle V) enables experimentation without fear of data loss
- **Export/Import**: Results can be exported to common formats (CSV, Excel) for external use

**Rationale**: Academic staff need productivity tools, not developer tools. The UI must match spreadsheet-like familiarity.

### IV. Solver Integration

The automatic scheduling feature is powered by a Python-based constraint solver that operates as an independent subprocess.

- **Separation of Concerns**: The solver (`solver/solver.py`) is a standalone component communicating via JSON I/O
- **Constraint Model Transparency**: The CP-SAT model encodes domain rules (teacher availability, venue capacity, campus restrictions) that MUST align with UI-level validations
- **Sidecar Architecture**: Rust spawns the solver as a child process, passing input via stdin/file and reading output from stdout/file
- **Build Integration**: Solver compilation (`npm run build:solver`) is part of the CI/CD pipeline, ensuring binary availability before Tauri build

**Rationale**: OR-Tools is Python-native; embedding via PyInstaller is more practical than Rust bindings while maintaining clean separation.

### V. Version Management & Revertibility

All edits follow an "optimistic temp-to-commit" pattern to enable safe experimentation.

- **Temporary State**: Changes are immediately saved to `data.tmp.json` (via debounced auto-save in Pinia store)
- **Explicit Commit**: Users explicitly invoke "Commit" to promote `data.tmp.json` → `data.json`
- **Instant Revert**: Users can discard all temporary changes and reload from `data.json` at any time
- **Solver Integration**: Automatic scheduling operates on temp data; results are written to temp and require commit to persist
- **State Isolation**: The working state (`data.tmp.json`) never affects the last-known-good state (`data.json`) until user confirmation

**Rationale**: Academic scheduling involves trial-and-error; the ability to safely explore alternatives without corrupting production data is essential.

## Deployment Requirements

### Technology Stack Constraints

- **Frontend**: Vue 3, Vite, Naive UI, Pinia (state management)
- **Backend**: Rust (Tauri 2.x framework)
- **Solver**: Python 3.11+ with OR-Tools, compiled via PyInstaller
- **Packaging**: Tauri bundler for Windows (MSI/EXE installers)
- **Build Tools**: npm, uv (Python package manager), Rust toolchain

### Build Pipeline

1. **Solver Compilation**: `npm run build:solver` → PyInstaller → `solver/dist/solver.exe`
2. **Binary Preparation**: `node scripts/prepare-solver.js` → Copy to `src-tauri/binaries/`
3. **Tauri Build**: `npm run tauri build` → Bundle frontend + backend + solver → Installer

### Performance Standards

- **Solver Timeout**: 60 seconds for constraint solving (configurable in `solver.py`)
- **UI Responsiveness**: Main thread never blocked by solver execution (runs as background process)
- **Data Load Time**: < 1 second for typical datasets (100 teachers, 200 courses)
- **Auto-Save Debounce**: 100ms delay to batch rapid edits

## Development Workflow

### Pre-Development Checklist

- [ ] Verify Rust, Node.js, Python 3.11+, and `uv` are installed
- [ ] Run `npm install` to install frontend dependencies
- [ ] Run `npm run build:solver` to compile Python solver
- [ ] Confirm `src-tauri/binaries/solver-*.exe` exists before `npm run tauri dev`

### Change Management

- **Data Model Changes**: Follow Principle I triple-update requirement; update tests for all three layers
- **Solver Logic Changes**: Edit `solver/solver.py`, rebuild via `npm run build:solver`, test via UI "Auto Schedule" button
- **UI Component Changes**: Leverage Pinia store reactivity; ensure auto-save triggers correctly
- **Tauri Commands**: Add/modify in `src-tauri/src/file_handler.rs` or `main.rs`; register in `main.rs` invocation handler

### Testing Strategy

- **Manual Testing**: Primary validation method due to GUI-heavy nature
- **Solver Validation**: Run solver with sample data, verify constraint satisfaction and objective value
- **Integration Points**: Test temp/commit workflow, solver I/O serialization, file operations
- **Edge Cases**: Empty datasets, maximum capacity schedules, solver timeouts, corrupted JSON recovery

## Governance

This constitution supersedes all other development practices. All code changes, architectural decisions, and feature implementations MUST comply with the five core principles.

### Amendment Process

1. Proposed changes must document impact on existing principles
2. Breaking changes to Principle I (Data Model Integrity) require major version bump (e.g., 1.x.x → 2.0.0)
3. New principles or governance sections require minor version bump (e.g., 1.0.x → 1.1.0)
4. Clarifications and non-semantic edits require patch version bump (e.g., 1.0.0 → 1.0.1)

### Compliance Verification

- All feature implementations must reference applicable principles in their specification (`specs/*/spec.md`)
- Code reviews must verify adherence to Principle I (data model consistency) and Principle II (standalone deployment)
- Breaking changes to the `AllData` schema require review of all dependent files (Rust models, Python DataManager, frontend stores)

### Runtime Guidance

For agent-specific implementation instructions, refer to `.github/copilot-instructions.md` (contains architecture details, build workflows, and developer workflows).

**Version**: 1.0.0 | **Ratified**: 2025-12-19 | **Last Amended**: 2025-12-19
