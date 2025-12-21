# Course Scheduler - AI Coding Agent Instructions

## Architecture Overview

### Stack & Data Flow
- **Frontend**: Vue 3 + Vite + Naive UI + Pinia (state) + Vue Router
- **Backend**: Tauri 2 + Rust + SQLite (rusqlite)
- **Solver Engine**: Python 3.11+ OR-Tools CP-SAT (packaged as sidecar binary)
- **Data Persistence**: SQLite with **dual-table optimistic temp-to-commit pattern**

### Core Pattern: Dual-Table Architecture
All user edits flow through temporary tables before permanent commit:
1. User edits → 100ms debounced auto-save → `*_temp` tables (`src/stores/data.js`)
2. User reviews → commits or reverts
3. Commit: `REPLACE INTO main SELECT * FROM temp` (atomic transaction)
4. Revert: `DELETE FROM temp` + reload from main

**Critical**: Solver writes to temp tables, requires explicit commit to persist results.

**Implementation**: See `src-tauri/src/db_handler.rs` (`commit_data`, `clear_temp_data`, `save_temp_data`).

## Critical Workflows

### Development Setup
```bash
npm install              # Install Node dependencies
uv sync                  # Install Python dependencies via uv
npm run build:solver     # Build solver binary (MUST run before dev/build)
npm run tauri dev        # Start dev server with hot reload
npm run tauri build      # Production build (NSIS installer in target/release/bundle/)
```

**Build Order**: Always `build:solver` → `tauri dev/build`. Solver binary lives at `src-tauri/binaries/solver-<target>.exe`.

### Solver Build Pipeline (`npm run build:solver`)
1. `cd solver && uv run pyinstaller solver.spec` → Creates `solver/dist/solver.exe`
2. `node scripts/prepare-solver.js` → Copies to `src-tauri/binaries/solver-<target>.exe`
3. Tauri bundles as external binary (sidecar) via `tauri.conf.json` → `externalBin`

**Tool**: Uses `uv` (fast Python package manager), NOT `pip`.

### Data Locations (Windows Only)
- **Database**: `%APPDATA%\com.tsxb.course-scheduler\course_scheduler.db`
- **Logs**: `%APPDATA%\com.tsxb.course-scheduler\logs\course-scheduler-YYYY.MM.DD.log`
- **Auto-migration**: Old JSON (`AppData\Local`) migrates to SQLite on first launch

## Coding Conventions

### Identifiers
- **All entity IDs**: Use `uuid.v4()` (frontend) or `uuidv4()` (Python)
- **Never use auto-increment integers** for primary keys

### State Management
- **Single source of truth**: `src/stores/data.js` (Pinia store)
- **Normalized data**: Separate relationship tables (`teacher_courses`, `course_venues`, `teacher_unavailability`, `scheduled_classes`)
- **Computed properties**: Use store's computed for derived relationships (e.g., `venuesByCampus`, `scheduledClassesByTeacher`)

### Business Logic Constraints
- **Venue propagation**: Changing `course.place` auto-updates all `scheduled_classes.venue_id` for that course (see `updateCourse()`)
- **Schedule density**: Manual constraint (not calculated) - max classes per campus/time slot
- **Database normalization**: MUST comply with 3NF - no redundant data, no transitive dependencies

### UI Patterns
- **Component library**: Naive UI (all components prefixed `n-*`)
- **Custom titlebar**: `MainLayout.vue` with `decorations: false` in `tauri.conf.json`
- **Window controls**: Minimize/maximize/close buttons in custom header with `data-tauri-drag-region`

## Key Files & Modules

### Frontend
- **`src/stores/data.js`** (637 lines): Core state, auto-save watcher, CRUD operations, normalized relationship handlers
- **`src/layouts/MainLayout.vue`**: Custom titlebar, menu, window controls, solver/commit/revert actions
- **`src/pages/*.vue`**: Feature pages (CourseManagement, TeacherManagement, VenueManagement, etc.)

### Backend (Rust)
- **`src-tauri/src/main.rs`**: App entry point, logging init (tracing + daily rotation), solver invocation, window lifecycle
- **`src-tauri/src/db_handler.rs`** (915 lines): SQLite CRUD, temp-to-commit logic, JSON migration, query builders
- **`src-tauri/src/models.rs`**: `AllData` struct and all entity models (shared with frontend JSON)
- **`src-tauri/src/import_export.rs`**: JSON/DB import/export, legacy format converter
- **`src-tauri/schema.sql`**: Complete schema for main + temp tables (11 tables each)

### Solver
- **`solver/solver.py`** (501 lines): OR-Tools CP-SAT model, hard/soft constraints, JSON I/O
- **DO NOT modify solver on `main` branch** - changes require deep constraint knowledge

## Database Schema

### Main Tables (Committed State)
`time_slots`, `days`, `campuses`, `venues`, `courses`, `teachers`, `scheduled_classes`, `teacher_courses`, `course_venues`, `teacher_unavailability`, `schedule_density`

### Temp Tables (Working State)
Identical structure with `_temp` suffix (e.g., `teachers_temp`)

### Foreign Keys
- Enabled: `PRAGMA foreign_keys = ON`
- Cascade deletes: `ON DELETE CASCADE` on all FKs
- Example: Deleting campus → deletes venues → deletes course_venues

## Tauri Commands (Frontend API)

### Data Operations
- **`load_data()`**: Returns temp data if exists, else main tables (called on app init)
- **`save_temp_data(content: AllData)`**: Writes to temp tables (debounced auto-save)
- **`commit_data()`**: Atomic copy temp → main, clears temp, emits `commit-completed` event
- **`clear_temp_data()`**: Revert by deleting temp data
- **`has_unsaved_changes()`**: Returns `true` if temp tables contain data

### Solver
- **`run_solver()`**: Spawns sidecar process, pipes AllData as JSON stdin, returns solved schedule

### Import/Export
- **`export_json_file(path)`**: Export AllData to JSON
- **`export_db_file(path)`**: Copy database file
- **`import_json_file(path)`**: Load and normalize JSON to temp tables
- **`import_db_file(path)`**: Replace database file

## Platform Constraints

**Windows 10+ ONLY**: Project optimized for Windows, no macOS/Linux support planned. Ignore cross-platform concerns.

---

## Future Implementation Roadmap

### Database Evolution Requirements

**Critical**: All new entities MUST comply with **Third Normal Form (3NF)**:
- No transitive dependencies
- All non-key attributes depend ONLY on the primary key
- No redundant data across tables (except for denormalized snapshots)
- Use separate relationship tables for many-to-many relationships

**Database Objects Strategy** (Triggers, Procedures, Functions):
- **ONLY use when they provide genuine business value** - avoid implementation for demonstration purposes
- **Triggers**: For automatic data integrity enforcement (audit logs, cascading updates)
- **Stored Procedures**: For complex multi-step transactions requiring atomicity (batch operations)
- **Functions**: For reusable calculated values accessed frequently in queries (statistics, aggregations)
- **Do NOT create** if simple Rust/TypeScript logic suffices - prefer application-layer logic for flexibility

### Phase 1: Version Management (Git-like Snapshots)

**Goal**: Enable schedule versioning with snapshot capability for history tracking and rollback.

#### New Entities (3NF Compliant)

1. **`versions` Table** - Version metadata
   ```sql
   CREATE TABLE versions (
       id TEXT PRIMARY KEY,              -- UUID
       parent_id TEXT,                   -- FK to versions(id) for version tree
       name TEXT NOT NULL,               -- User-friendly version name
       commit_msg TEXT,                  -- Description of changes
       created_at DATETIME NOT NULL,     -- Timestamp
       created_by TEXT NOT NULL,         -- FK to users(id)
       FOREIGN KEY (parent_id) REFERENCES versions(id) ON DELETE SET NULL,
       FOREIGN KEY (created_by) REFERENCES users(id)
   );
   ```

2. **`version_scheduled_classes` Table** - Immutable schedule snapshots
   ```sql
   CREATE TABLE version_scheduled_classes (
       version_id TEXT NOT NULL,         -- FK to versions(id)
       schedule_id TEXT NOT NULL,        -- Original schedule record ID
       teacher_id TEXT NOT NULL,
       course_id TEXT NOT NULL,
       day_id TEXT NOT NULL,
       time_id TEXT NOT NULL,
       campus_id TEXT NOT NULL,
       venue_id TEXT NOT NULL,
       PRIMARY KEY (version_id, schedule_id),
       FOREIGN KEY (version_id) REFERENCES versions(id) ON DELETE CASCADE
   );
   ```
   **Note**: This table stores denormalized snapshots for historical integrity. Data is frozen at commit time.

#### Database Objects (Required for Business Logic)

**Stored Procedure**: `proc_create_version(name, commit_msg, user_id)`
- **Why Procedure**: Ensures **atomic snapshot creation** - if any step fails, entire version rollback occurs
- **Business Value**: Guarantees data consistency when creating immutable snapshots (critical for audit trail)
- **Logic**:
  1. Begin transaction
  2. Insert new version record with `created_at = CURRENT_TIMESTAMP`
  3. Batch copy 50-200+ schedule records from `scheduled_classes_temp` to `version_scheduled_classes`
  4. If any FK violation occurs, rollback entire operation
  5. Return version ID on success
- **Why NOT Rust**: SQLite transaction over network calls is slower; batching within DB is 10-50x faster
- **Location**: `src-tauri/schema_procedures.sql` (or inline in `db_handler.rs` as prepared statement)

**Trigger**: `trg_versions_audit` - Automatic audit logging
- **Why Trigger**: Enforces **mandatory audit trail** - cannot be bypassed by application code
- **Business Value**: Security compliance - every version creation is logged automatically
- **Event**: `AFTER INSERT ON versions`
- **Action**: 
  ```sql
  INSERT INTO audit_logs (id, user_id, action, table_name, record_id, timestamp)
  VALUES (hex(randomblob(16)), NEW.created_by, 'VERSION_CREATED', 'versions', NEW.id, NEW.created_at);
  ```
- **Why NOT Rust**: Application-layer logging can be skipped accidentally; trigger guarantees execution

#### Implementation Notes

- Version creation is **separate from temp-to-main commit** - users can create versions at any point
- Use `parent_id` self-reference to build version tree (enables branching if needed)
- Frontend: Add "Create Version" button in MainLayout.vue, alongside commit/revert

### Phase 2: RBAC & Security

**Goal**: Multi-role authentication with permission-based access control.

#### New Entities (3NF Compliant)

3. **`roles` Table** - Role definitions
   ```sql
   CREATE TABLE roles (
       id TEXT PRIMARY KEY,              -- UUID
       name TEXT NOT NULL UNIQUE,        -- 'Scheduler', 'Teacher', 'Admin'
       description TEXT
   );
   ```

4. **`users` Table** - User accounts
   ```sql
   CREATE TABLE users (
       id TEXT PRIMARY KEY,              -- UUID
       username TEXT NOT NULL UNIQUE,
       password_hash TEXT NOT NULL,      -- bcrypt/argon2 hash
       role_id TEXT NOT NULL,            -- FK to roles(id)
       teacher_id TEXT,                  -- FK to teachers(id), NULL for non-teachers
       created_at DATETIME NOT NULL,
       last_login DATETIME,
       FOREIGN KEY (role_id) REFERENCES roles(id),
       FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE SET NULL
   );
   ```

5. **`audit_logs` Table** - Security audit trail
   ```sql
   CREATE TABLE audit_logs (
       id TEXT PRIMARY KEY,              -- UUID
       user_id TEXT NOT NULL,            -- FK to users(id)
       action TEXT NOT NULL,             -- 'LOGIN', 'VERSION_CREATED', 'DATA_EXPORTED'
       table_name TEXT,                  -- Target table (if applicable)
       record_id TEXT,                   -- Target record ID
       timestamp DATETIME NOT NULL,
       ip_address TEXT,
       FOREIGN KEY (user_id) REFERENCES users(id)
   );
   ```

#### Role Permissions

| Role       | Permissions                                                                 |
|------------|-----------------------------------------------------------------------------|
| **Scheduler** | Full CRUD on all entities, run solver, create versions, import/export    |
| **Teacher**   | Read-only access to own schedule (`teacher_id` match), export own CSV    |
| **Admin**     | All Scheduler permissions + user management                              |

#### Database Objects (Required for Security)

**Trigger**: `trg_users_password_update` - Password change audit
- **Why Trigger**: Security compliance - every password change MUST be logged
- **Event**: `AFTER UPDATE OF password_hash ON users`
- **Action**: Insert audit record with action `'PASSWORD_CHANGED'`
- **Business Value**: Detect unauthorized password resets, security breach investigation

**Trigger**: `trg_audit_sensitive_operations` - Generic audit trigger
- **Why Trigger**: Centralized security logging cannot be bypassed
- **Events**: `AFTER DELETE ON scheduled_classes`, `AFTER UPDATE ON roles`, etc.
- **Business Value**: Complete audit trail for forensic analysis and compliance

#### Implementation Strategy

- **Authentication**: Rust backend validates credentials, returns JWT token
- **Authorization**: Middleware checks role before Tauri command execution
- **Frontend**: Conditional UI rendering based on user role (hide admin buttons for teachers)
- **Tauri Commands**: Add `user_id` parameter to all write operations for audit logging
- **Note**: Triggers handle audit logging automatically - Rust code only needs to perform business operations

### Phase 3: Statistical Dashboard & PDF Export

**Goal**: Aggregated analytics with chart visualization and PDF reporting.

#### New Entities (3NF Compliant)

6. **`dashboard_queries` Table** - Saved query definitions
   ```sql
   CREATE TABLE dashboard_queries (
       id TEXT PRIMARY KEY,
       name TEXT NOT NULL,
       sql_query TEXT NOT NULL,         -- Parameterized SQL
       chart_type TEXT,                 -- 'bar', 'pie', 'line'
       created_by TEXT NOT NULL,
       FOREIGN KEY (created_by) REFERENCES users(id)
   );
   ```

#### Key Statistics (SQL Functions - Use Judiciously)

**Function**: `fn_get_teacher_workload(teacher_id TEXT) RETURNS INTEGER`
- **Why Function**: Called repeatedly in dashboard queries - caching logic at DB layer improves performance
- **Business Value**: Avoid recalculating same aggregation across multiple API calls
- **Implementation**:
  ```sql
  CREATE FUNCTION fn_get_teacher_workload(teacher_id TEXT) RETURNS INTEGER
  BEGIN
      RETURN (SELECT COALESCE(SUM(ts.corresponding_hours), 0)
              FROM scheduled_classes sc
              JOIN time_slots ts ON sc.time_id = ts.id
              WHERE sc.teacher_id = teacher_id);
  END;
  ```
- **Usage**: `SELECT name, fn_get_teacher_workload(id) FROM teachers;`
- **Alternative**: If only used once, inline the subquery - functions add complexity

**Function**: `fn_get_campus_utilization(campus_id TEXT, day_id TEXT, time_id TEXT) RETURNS REAL`
- **Why Function**: Complex calculation with multiple joins - encapsulate for reusability
- **Business Value**: Consistent utilization formula across all reports
- **Returns**: Percentage (0.0 to 100.0) of venue capacity used

**AVOID**: Table-valued functions in SQLite (limited support). Use views instead:
```sql
CREATE VIEW vw_course_distribution AS
SELECT co.name, c.name as campus_name, COUNT(sc.id) as class_count
FROM courses co
CROSS JOIN campuses c
LEFT JOIN scheduled_classes sc ON co.id = sc.course_id AND c.id = sc.campus_id
GROUP BY co.id, c.id;
```

#### Chart Types & SQL Queries

1. **Teacher Workload Distribution** (Bar Chart)
   ```sql
   SELECT t.name, SUM(ts.corresponding_hours) as total_hours
   FROM teachers t
   JOIN scheduled_classes sc ON t.id = sc.teacher_id
   JOIN time_slots ts ON sc.time_id = ts.id
   GROUP BY t.id ORDER BY total_hours DESC;
   ```

2. **Campus Utilization Heatmap** (Heatmap)
   ```sql
   SELECT c.name, d.value as day, t.value as time, 
          COUNT(sc.id) as class_count
   FROM campuses c
   CROSS JOIN days d
   CROSS JOIN time_slots t
   LEFT JOIN scheduled_classes sc 
       ON c.id = sc.campus_id AND d.id = sc.day_id AND t.id = sc.time_id
   GROUP BY c.id, d.id, t.id;
   ```

3. **Course Popularity** (Pie Chart)
   ```sql
   SELECT co.name, COUNT(DISTINCT tc.teacher_id) as teacher_count
   FROM courses co
   LEFT JOIN teacher_courses tc ON co.id = tc.course_id
   GROUP BY co.id ORDER BY teacher_count DESC;
   ```

#### PDF Export

- **Library**: Use `printpdf` (Rust) or `wkhtmltopdf` (sidecar)
- **Template**: HTML/CSS report generated from Vue component → converted to PDF
- **Sections**: Summary statistics, charts (as PNG), detailed schedule tables

### Implementation Priority

1. **Phase 2 (Security)** - Highest priority, foundational for multi-user access
2. **Phase 1 (Versioning)** - Medium priority, enables safe experimentation
3. **Phase 3 (Dashboard)** - Lower priority, nice-to-have analytics

### Database Migration Strategy

- **New schema file**: `src-tauri/schema_v2.sql` with migration logic
- **Versioned migrations**: Use `refinery` or `diesel-migrations` crate
- **Backward compatibility**: Maintain existing temp-to-commit pattern
- **Data integrity**: Add database version check on app startup

### Database Objects Implementation Checklist

**When to Use Database Objects**:

✅ **Use Triggers When**:
- Enforcing mandatory audit logging (cannot be bypassed)
- Automatic timestamp updates (`updated_at` columns)
- Cascade operations beyond foreign key constraints
- Data validation that MUST happen at DB level

✅ **Use Stored Procedures When**:
- Complex multi-table transactions requiring atomicity
- Batch operations involving 100+ rows (performance critical)
- Operations combining inserts, updates, deletes in specific order

✅ **Use Functions When**:
- Calculation reused in 3+ different queries
- Performance-critical aggregations accessed frequently
- Business logic that MUST stay consistent across all clients

❌ **Avoid When**:
- Simple CRUD operations (use Rust/TypeScript)
- Logic that changes frequently (harder to test/debug in SQL)
- SQLite doesn't support feature (stored procedures have limitations)
- Can achieve same result with foreign keys + application code

**SQLite Limitations**:
- No native stored procedures (workaround: use prepared statements in Rust)
- Limited function support (use Rust UDFs via `rusqlite::functions`)
- Triggers cannot call external code (pure SQL only)

### Additional Entity Examples (Total 12+)

To meet the 12+ entity requirement:

7. **`user_sessions` Table** - Active login sessions (with trigger for auto-expiration)
8. **`export_history` Table** - Track data exports for compliance
9. **`solver_runs` Table** - Log solver execution parameters and results
10. **`campus_holidays` Table** - Campus-specific non-teaching days
11. **`course_prerequisites` Table** - Course dependency relationships
12. **`notification_preferences` Table** - User notification settings
13. **`system_config` Table** - App-wide configuration key-value pairs

All entities follow 3NF principles with proper foreign key relationships and normalized data structures.
