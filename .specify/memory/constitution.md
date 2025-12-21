<!--
Sync Impact Report - Version 2.0.0
=========================================
Version Change: 1.0.0 → 2.0.0
Rationale: MAJOR version bump - Complete restructure to focus on programming principles rather than implementation details. Removed technology stack specifics (moved to copilot-instructions.md).

Principles Modified:
- I. Data Model Integrity → Cross-Language Schema Stability (renamed, clarified synchronization requirements)
- II. Standalone Deployment → REMOVED (implementation detail, not a programming principle)
- III. User-Centric Operations → REMOVED (UI/UX guideline, not a programming principle)
- IV. Solver Integration → REMOVED (architectural detail, not a programming principle)
- V. Version Management & Revertibility → State Isolation & Revertibility (renamed, clarified as general pattern)

Principles Added:
- II. Referential Integrity Enforcement (NEW) - Cascading updates and orphan prevention
- III. Constraint Validation Alignment (NEW) - Multi-layer validation consistency
- IV. Single Source of Truth (NEW) - Authoritative state management
- V. Separation of Concerns (NEW) - Clear component boundaries
- VI. State Isolation & Revertibility (from old V, renamed)

Section Changes:
- REMOVED: "Deployment Requirements" (moved to copilot-instructions.md)
- REMOVED: "Development Workflow" (moved to copilot-instructions.md)
- REMOVED: "Technology Stack Constraints" (moved to copilot-instructions.md)
- UPDATED: "Governance" section clarified to reference implementation docs

Templates Status:
✅ plan-template.md - Constitution check now references programming principles only
✅ spec-template.md - No changes needed (user story structure unaffected)
✅ tasks-template.md - No changes needed (task organization unaffected)

Follow-up Items: None - all principles now properly scoped to programming practices

Last Updated: 2025-12-21
-->

# Course Scheduler Constitution

## Core Principles

### I. Cross-Language Schema Stability

The shared data model MUST remain consistent across all language boundaries.

- **Synchronized Definitions**: Any data structure used across multiple languages (Rust, Python, JavaScript) MUST have identical field names, types, and semantics
- **Triple-Update Rule**: Changes to shared entities require synchronous updates in all three language implementations:
  1. Backend serialization layer (Rust)
  2. Computation layer (Python)
  3. Frontend state layer (JavaScript)
- **Explicit Migration**: Schema changes must include migration logic; breaking changes are forbidden without version negotiation
- **Validation at Boundaries**: Each language boundary MUST validate incoming data structure integrity

**Rationale**: Multi-language architectures fail catastrophically when shared data contracts drift. Preventing desynchronization is cheaper than debugging cross-language data corruption.

### II. Referential Integrity Enforcement

Data relationships MUST be enforced programmatically, not assumed.

- **Cascading Updates**: Changes to referenced entities (e.g., venue assignments) MUST automatically propagate to dependent data (e.g., scheduled classes)
- **Orphan Prevention**: Deletions of referenced entities MUST either cascade to dependents or block if dependencies exist
- **Relationship Validation**: Foreign key relationships must be verified before mutation operations
- **Automatic Cleanup**: When a reference becomes invalid, the system MUST either repair or remove the broken link—never leave orphaned data

**Rationale**: Broken references create silent data corruption that manifests as user-visible bugs. Proactive enforcement prevents data inconsistency states.

### III. Constraint Validation Alignment

Business constraints MUST be enforced consistently across all system layers.

- **UI Pre-Validation**: User interface MUST prevent invalid operations before submission (e.g., disable unavailable time slots)
- **Backend Enforcement**: Server/backend MUST re-validate all constraints regardless of UI state (defense in depth)
- **Solver Consistency**: Constraint solver MUST encode the same business rules as UI validations (e.g., teacher availability, venue capacity)
- **Single Rule Definition**: Constraints should be defined once and referenced by all layers, not duplicated in code

**Rationale**: Inconsistent constraint enforcement leads to user confusion ("why did the UI allow me to do X if the solver rejects it?"). Alignment eliminates contradictory behaviors.

### IV. Single Source of Truth

Each piece of application state MUST have exactly one authoritative location.

- **Unidirectional State Flow**: Derived state must be computed from the source, never stored redundantly
- **No Duplicate Storage**: The same logical data MUST NOT exist in multiple independent stores
- **Centralized Mutations**: State changes must flow through a single entry point (e.g., Pinia store actions)
- **Read-Only Derivations**: Computed values must be read-only projections of authoritative state

**Rationale**: Duplicate state inevitably desynchronizes. Single source of truth eliminates entire classes of synchronization bugs.

### V. Separation of Concerns

System components MUST have clear boundaries and minimal coupling.

- **Component Independence**: Each subsystem (UI, backend, computation) should be replaceable without rewriting others
- **Interface Contracts**: Communication between components must use well-defined, versioned interfaces (e.g., JSON schemas)
- **Functional Isolation**: A component's internal implementation MUST NOT leak into other components' logic
- **Dependency Inversion**: High-level components depend on abstractions, not concrete implementations

**Rationale**: Tight coupling creates brittle systems where changes cascade unpredictably. Clear boundaries enable independent evolution and testing.

### VI. State Isolation & Revertibility

User-facing state changes MUST be isolated from committed data until explicitly confirmed.

- **Optimistic Temporary State**: All user edits write to temporary storage (temp tables, temp files) immediately
- **Explicit Commit Boundary**: Promotion from temporary to permanent storage requires explicit user action
- **Instant Rollback**: Users can discard all temporary changes and restore the last committed state at any time
- **Transaction Safety**: Temporary state and permanent state must never mix—operations are atomic (all-or-nothing)

**Rationale**: Users need confidence to experiment without fear of data loss. Separating working state from committed state enables safe exploration and reduces user errors.

## Governance

### Constitution Scope

This constitution defines **programming principles** that govern code quality, maintainability, and correctness. It does NOT prescribe:
- Technology stack choices (see `.github/copilot-instructions.md` for implementation details)
- Development workflows (see `.github/copilot-instructions.md` for build processes)
- UI/UX guidelines (handled in design specifications)

### Amendment Process

1. **Proposed Change**: Document the principle change, affected code, and migration path
2. **Version Bump Rules**:
   - **MAJOR** (X.0.0): Add/remove/redefine principles; backward-incompatible changes
   - **MINOR** (x.Y.0): Expand existing principles; add clarifying sections
   - **PATCH** (x.y.Z): Fix typos; reword without semantic change
3. **Validation**: Update all template constitution checks to reflect new/changed principles
4. **Propagation**: Review existing code for compliance; document technical debt if non-compliant

### Compliance Verification

**Pre-Implementation** (in `specs/[###-feature]/plan.md`):
- [ ] Constitution Check section completed for all six principles
- [ ] Schema changes documented with triple-update plan (Principle I)
- [ ] Referential integrity impact analyzed (Principle II)
- [ ] Constraint alignment verified across layers (Principle III)
- [ ] State source identified (Principle IV)
- [ ] Component boundaries documented (Principle V)
- [ ] Temp/commit workflow impact assessed (Principle VI)

**Post-Implementation**:
- Code reviews MUST verify adherence to Principle I (cross-language schema sync) for data model changes
- Constraint changes MUST be validated in both UI and solver (Principle III)
- State mutations MUST flow through centralized store (Principle IV)

### Documentation References

- **Implementation Details**: `.github/copilot-instructions.md` (architecture, tech stack, workflows)
- **Build Instructions**: `README.md` (setup, dependencies, commands)
- **Feature Specifications**: `specs/[###-feature]/spec.md` (requirements, acceptance criteria)

**Version**: 2.0.0 | **Ratified**: 2025-12-19 | **Last Amended**: 2025-12-21
