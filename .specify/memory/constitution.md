<!--
Sync Impact Report - Version 2.3.0
=========================================
Version Change: 2.2.0 → 2.3.0
Rationale: MINOR version bump - Updated placeholder values with concrete project context from README.md and copilot-instructions.md. Added project-specific details throughout all principles and governance sections.

Principles Modified:
- I. Cross-Language Schema Stability: Added explicit quad-update rule (Rust, Python, JavaScript, SQLite schema)
- III. Constraint Validation Alignment: Enhanced with specific Course Scheduler constraints (hard/soft constraints)
- VI. State Isolation & Revertibility: Added specific implementation details for dual-table architecture

Principles Added: None
Principles Removed: None

Templates Status:
✅ plan-template.md - Already aligned with constitution checks
✅ tasks-template.md - Already includes Course Scheduler-specific task patterns
✅ spec-template.md - No changes needed (priority-based user stories)
✅ checklist-template.md - No constitution references
✅ agent-file-template.md - No constitution references

Follow-up Items: None

Last Updated: 2025-12-22
-->

# Course Scheduler Constitution

## Project Identity

**Name**: Course Scheduler (课程排课系统)  
**Purpose**: Intelligent course scheduling desktop application using constraint optimization  
**Platform**: Windows 10+ desktop application (Tauri 2 framework)  
**Repository**: [tsx8/course-scheduler](https://github.com/tsx8/course-scheduler)

## Core Principles

### I. Cross-Language Schema Stability

The shared data model MUST remain consistent across all language boundaries.

- **Synchronized Definitions**: Any data structure used across multiple languages (Rust, Python, JavaScript, SQLite) MUST have identical field names, types, and semantics
- **Quad-Update Rule**: Changes to shared entities require synchronous updates in all four implementations:
  1. **Rust Backend**: `src-tauri/src/models.rs` (`AllData` struct and entity models)
  2. **SQLite Schema**: `src-tauri/schema.sql` (main tables + `_temp` tables, indices, foreign keys)
  3. **Python Solver**: `solver/solver.py` (`DataManager` class and constraint model input)
  4. **Frontend Store**: `src/stores/data.js` (Pinia reactive refs and computed properties)
- **Explicit Migration**: Schema changes must include migration logic in `db_handler.rs`; breaking changes require version negotiation
- **Validation at Boundaries**: Each language boundary MUST validate incoming data structure integrity (Serde deserialization, JSON schema validation)

**Rationale**: Multi-language architectures fail catastrophically when shared data contracts drift. The Course Scheduler's Rust-Python-JavaScript architecture with SQLite persistence has four synchronization points. Preventing desynchronization is cheaper than debugging cross-language data corruption in production.

**Course Scheduler Entities**: `time_slots`, `days`, `campuses`, `venues`, `courses`, `teachers`, `scheduled_classes`, `teacher_courses`, `course_venues`, `teacher_unavailability`, `schedule_density`

### II. Referential Integrity Enforcement

Data relationships MUST be enforced programmatically, not assumed.

- **Foreign Key Constraints**: SQLite `PRAGMA foreign_keys = ON` must be enabled in all database connections
- **Cascading Deletes**: Use `ON DELETE CASCADE` to maintain relationship integrity:
  - Deleting `campus` → cascades to `venues` → cascades to `course_venues` and `scheduled_classes`
  - Deleting `teacher` → cascades to `teacher_courses`, `teacher_unavailability`, `scheduled_classes`
  - Deleting `course` → cascades to `course_venues`, `teacher_courses`, `scheduled_classes`
- **Venue Propagation**: Changes to `course.place` (venue assignment) MUST automatically update all `scheduled_classes.venue_id` for that course via `updateCourse()` in Pinia store
- **Orphan Prevention**: Database operations MUST either cascade to dependents or block if dependencies exist (transaction rollback)
- **Automatic Cleanup**: When a reference becomes invalid, the system MUST either repair or remove the broken link—never leave orphaned data

**Rationale**: Broken references create silent data corruption that manifests as user-visible bugs (e.g., scheduled classes referencing deleted venues). Proactive enforcement via database constraints and application logic prevents data inconsistency states.

### III. Constraint Validation Alignment

Business constraints MUST be enforced consistently across all system layers.

- **UI Pre-Validation**: User interface MUST prevent invalid operations before submission (e.g., disable time slots marked as teacher unavailable, show venue capacity warnings)
- **Backend Enforcement**: Tauri commands MUST re-validate all constraints regardless of UI state (defense in depth)
- **Solver Consistency**: OR-Tools CP-SAT model MUST encode the same business rules as UI validations

**Hard Constraints** (MUST be satisfied, implemented in `solver/solver.py` lines 116-208):
- Teacher conflict: One teacher, one location at a time
- Venue conflict: One course per venue per time slot
- Teacher workload: Total hours ≤ `max_hours_per_week`
- Teacher availability: No classes during marked unavailable time slots
- Campus exclusivity: `is_only_shahe` teachers only teach at Shahe campus
- Cross-campus requirement: Non-exclusive teachers MUST teach at both campuses
- Daily campus consistency: Teachers can only be at one campus per day
- Work day limit: Teachers with ≥4 work days do not get additional days

**Soft Constraints** (optimization goals, lines 210-340):
- Approach schedule density target per campus/time slot (manual constraint, not auto-calculated)
- Minimize single-class days (days with only one class)
- Minimize gaps between consecutive classes
- Minimize scattered work days (non-consecutive days)
- Maximize course offerings
- Balance course distribution across campuses

- **Manual Constraint Definition**: Schedule density MUST be stored in `schedule_density` table as user-specified target (max classes per campus/time slot), NOT derived from venue counts
- **Single Rule Definition**: Constraint logic defined once in solver, validated in UI, enforced in backend

**Rationale**: Inconsistent constraint enforcement leads to user confusion ("why did the UI allow me to schedule a class if the solver rejects it?"). Three-layer alignment (UI + Backend + Solver) eliminates contradictory behaviors. Explicitly treating schedule density as manual prevents incorrect capacity assumptions.

### IV. Single Source of Truth

Each piece of application state MUST have exactly one authoritative location.

- **State Authority**: `src/stores/data.js` (Pinia store) is the single source of truth for frontend state
- **Database Authority**: Main tables in SQLite (`course_scheduler.db` at `%APPDATA%\com.tsxb.course-scheduler\`) are the single source of truth for persisted data
- **Unidirectional State Flow**: Derived state must be computed from the source via computed properties (e.g., `venuesByCampus`, `scheduledClassesByTeacher`)
- **No Duplicate Storage**: The same logical data MUST NOT exist in multiple independent stores
- **Centralized Mutations**: All state changes flow through Pinia store actions with 100ms debounced auto-save to `*_temp` tables
- **Read-Only Derivations**: Computed values are read-only projections; mutations go through store actions only

**Rationale**: Duplicate state inevitably desynchronizes. The dual-table architecture (main + temp) maintains a single source of truth with explicit commit boundaries. Computed properties ensure derived data stays synchronized without manual cache invalidation.

### V. Separation of Concerns

System components MUST have clear boundaries and minimal coupling.

- **Component Independence**: Each subsystem is replaceable without rewriting others:
  - **Frontend**: Vue 3 + Vite + Naive UI + Pinia + Vue Router (`src/`)
  - **Backend**: Tauri 2 + Rust + SQLite (`src-tauri/src/`)
  - **Solver**: Python 3.11+ + OR-Tools CP-SAT (sidecar binary `solver/`)
- **Interface Contracts**: Communication uses well-defined, versioned JSON schemas:
  - Tauri commands: `load_data()`, `save_temp_data(AllData)`, `commit_data()`, `run_solver()`
  - Solver I/O: JSON stdin/stdout with `AllData` structure
- **Functional Isolation**: Solver cannot access database directly; must go through Rust backend serialization
- **Dependency Inversion**: Frontend depends on Tauri command abstractions, not Rust implementation details

**Rationale**: Tight coupling creates brittle systems where changes cascade unpredictably (e.g., changing SQLite schema breaks solver). Clear boundaries enable independent evolution (can replace OR-Tools with another solver without touching frontend/backend).

### VI. State Isolation & Revertibility

User-facing state changes MUST be isolated from committed data until explicitly confirmed.

- **Dual-Table Architecture**: All tables exist in pairs (e.g., `teachers` + `teachers_temp`):
  - **Main tables**: Committed, permanent data
  - **Temp tables**: Working state with `_temp` suffix
- **Optimistic Temporary State**: All user edits write to temp tables immediately via 100ms debounced auto-save (`save_temp_data` command)
- **Explicit Commit Boundary**: Promotion from temp to main requires user clicking "提交更改" button:
  - Backend executes: `REPLACE INTO main_table SELECT * FROM temp_table`
  - Atomic transaction ensures all-or-nothing
  - Emits `commit-completed` event to frontend
  - Clears temp tables after successful commit
- **Instant Rollback**: User clicks "撤销更改" to discard all temp data:
  - Backend executes: `DELETE FROM *_temp` tables
  - Frontend reloads from main tables
- **Transaction Safety**: SQLite ACID properties ensure temp and main state never mix
- **Solver Integration**: Solver writes results to temp tables, requiring explicit commit to persist

**Rationale**: Users need confidence to experiment without fear of data loss. The dual-table pattern enables safe exploration (run solver multiple times, adjust manually) and instant rollback. Critical for constraint-based workflows where trial-and-error is common.

**Implementation**: See `src-tauri/src/db_handler.rs` (`commit_data`, `clear_temp_data`, `save_temp_data`, `has_unsaved_changes`)

### VII. Relational Data Normalization

All database tables and relationships MUST be normalized to the Third Normal Form (3NF).

- **3NF Compliance**: Every table MUST be in 3NF to prevent data redundancy and update anomalies
- **No Transitive Dependencies**: Non-key attributes MUST NOT depend on other non-key attributes; they depend only on the primary key
- **Atomic Values**: Each column MUST contain only atomic (indivisible) values; no multi-valued attributes or nested tables
- **Primary Key Dependency**: Every non-key attribute MUST depend on the primary key, the whole primary key, and nothing but the primary key
- **Many-to-Many Relationships**: Use junction tables without redundant attributes:
  - `course_venues`: Only stores `course_id` + `venue_id` (no redundant `campus_id`)
  - `teacher_courses`: Only stores `teacher_id` + `course_id` (no redundant course names)
- **No Redundant Storage**: `campus_id` stored only in `venues` table; queries use JOIN to retrieve
- **Exception Documentation**: Any intentional deviation from 3NF (e.g., `version_scheduled_classes` denormalized snapshots for historical integrity) MUST be explicitly documented with justification

**Rationale**: Normalization prevents data redundancy and update anomalies. In the Course Scheduler's complex relationship graph (11 main tables + 11 temp tables), 3NF compliance simplifies cascading update logic and maintains data integrity across the dual-table architecture. Flat, normalized structures also optimize frontend state management and serialization performance.

**Examples**:
- ✅ **Correct**: `scheduled_classes` has `venue_id` FK; `campus_id` retrieved via `JOIN venues`
- ❌ **Violation**: Storing `teacher_name` in `scheduled_classes` (already in `teachers` table)

## Governance

### Constitution Scope

This constitution defines **programming principles** that govern code quality, maintainability, and correctness for the Course Scheduler project. It does NOT prescribe:
- Technology stack choices (documented in `.github/copilot-instructions.md`)
- Development workflows and build processes (documented in `.github/copilot-instructions.md` and `README.md`)
- UI/UX design guidelines (handled in feature specifications)
- Constraint solver algorithm details (implemented in `solver/solver.py`, requires domain expertise to modify)

### Amendment Process

1. **Proposed Change**: Document the principle change, affected code, and migration path
2. **Version Bump Rules**:
   - **MAJOR** (X.0.0): Add/remove/redefine principles; backward-incompatible governance changes
   - **MINOR** (x.Y.0): Expand existing principles with new requirements; add clarifying sections or project-specific details
   - **PATCH** (x.y.Z): Fix typos; reword for clarity without semantic change
3. **Validation**: Update all template constitution checks in `.specify/templates/` to reflect new/changed principles
4. **Propagation**: Review existing codebase for compliance; document technical debt if immediate compliance is not feasible

### Compliance Verification

**Pre-Implementation** (in `specs/[###-feature]/plan.md`):
- [ ] Constitution Check section completed for all seven principles
- [ ] **Principle I**: Schema changes documented with quad-update plan (Rust models, SQLite schema, Python DataManager, JavaScript store)
- [ ] **Principle II**: Referential integrity impact analyzed (foreign key cascades, venue propagation logic)
- [ ] **Principle III**: Constraint alignment verified across layers (UI validation, backend enforcement, solver hard/soft constraints)
- [ ] **Principle IV**: State source identified (Pinia store for frontend, main tables for database)
- [ ] **Principle V**: Component boundaries documented (Tauri command contracts, JSON schema versioning)
- [ ] **Principle VI**: Temp/commit workflow impact assessed (writes to temp tables, commit/revert UI)
- [ ] **Principle VII**: Data model changes verified for 3NF compliance (no transitive dependencies, junction tables for many-to-many)

**Post-Implementation**:
- Code reviews MUST verify adherence to Principle I (quad-update for schema changes: `models.rs`, `schema.sql`, `solver.py`, `data.js`)
- Constraint changes MUST be validated in all three layers (UI prevents, backend validates, solver enforces)
- State mutations MUST flow through Pinia store actions with debounced auto-save to temp tables
- Database schema changes MUST be verified against 3NF rules (no redundant attributes, junction tables only)
- Foreign key constraints MUST be tested for proper cascading behavior

### Documentation References

- **Architecture & Implementation**: `.github/copilot-instructions.md` (stack, data flow, coding conventions, file structure, future roadmap)
- **User Guide & Build Instructions**: `README.md` (features, setup, usage workflows, database design, troubleshooting)
- **Feature Specifications**: `specs/[###-feature]/spec.md` (user stories, acceptance criteria, functional requirements)
- **Database Schema**: `src-tauri/schema.sql` (table definitions, foreign keys, indices)
- **Constraint Model**: `solver/solver.py` (hard constraints, soft constraints, objective function)

### Version History

- **2.3.0** (2025-12-22): Updated with concrete Course Scheduler project details from README and copilot-instructions
- **2.2.0** (2025-12-21): Added Principle VII (3NF normalization requirement)
- **2.1.0** (2025-12-20): Expanded governance section with compliance verification checklist
- **2.0.0** (2025-12-19): Initial ratification with seven core principles

**Version**: 2.3.0 | **Ratified**: 2025-12-19 | **Last Amended**: 2025-12-22
