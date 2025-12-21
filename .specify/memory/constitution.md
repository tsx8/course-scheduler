<!--
Sync Impact Report - Version 2.2.0
=========================================
Version Change: 2.1.0 → 2.2.0
Rationale: MINOR version bump - Added Principle VII requiring 3NF normalization for all data models.

Principles Modified: None
Principles Added:
- VII. Relational Data Normalization: Mandatory 3NF compliance for all database relationships.

Principles Removed: None

Templates Status:
✅ plan-template.md - Added 3NF compliance check
✅ tasks-template.md - Added normalization verification tasks
✅ spec-template.md - No changes needed

Follow-up Items: None

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
- **Manual Constraint Definition**: Schedule density MUST be treated as a manual constraint specified by staff (maximum classes per campus per time slot), NOT an auto-calculated value derived from venue counts
- **Single Rule Definition**: Constraints should be defined once and referenced by all layers, not duplicated in code

**Rationale**: Inconsistent constraint enforcement leads to user confusion ("why did the UI allow me to do X if the solver rejects it?"). Alignment eliminates contradictory behaviors. Explicitly defining manual constraints like schedule density prevents the system from making incorrect assumptions about capacity.

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

### VII. Relational Data Normalization

All database tables and relationships MUST be normalized to the Third Normal Form (3NF).

- **3NF Compliance**: Every table MUST be in 3NF to prevent data redundancy and update anomalies.
- **No Transitive Dependencies**: Non-key attributes MUST NOT depend on other non-key attributes; they must depend only on the primary key.
- **Atomic Values**: Each column MUST contain only atomic (indivisible) values; no multi-valued attributes or nested tables.
- **Primary Key Dependency**: Every non-key attribute MUST depend on the primary key, the whole primary key, and nothing but the primary key.
- **Exception Documentation**: Any intentional deviation from 3NF (e.g., for performance optimization) MUST be explicitly documented and justified in the data model specification.

**Rationale**: Normalization prevents data redundancy and update anomalies. In a complex scheduling system, ensuring 3NF simplifies the logic for cascading updates and maintains data integrity across the dual-table (temp/main) architecture.

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
- [ ] Constitution Check section completed for all seven principles
- [ ] Schema changes documented with triple-update plan (Principle I)
- [ ] Referential integrity impact analyzed (Principle II)
- [ ] Constraint alignment verified across layers (Principle III)
- [ ] State source identified (Principle IV)
- [ ] Component boundaries documented (Principle V)
- [ ] Temp/commit workflow impact assessed (Principle VI)
- [ ] **3NF Normalization**: Data model changes verified for 3NF compliance (Principle VII)

**Post-Implementation**:
- Code reviews MUST verify adherence to Principle I (cross-language schema sync) for data model changes
- Constraint changes MUST be validated in both UI and solver (Principle III)
- State mutations MUST flow through centralized store (Principle IV)
- Database schema changes MUST be verified against 3NF rules (Principle VII)

### Documentation References

- **Implementation Details**: `.github/copilot-instructions.md` (architecture, tech stack, workflows)
- **Build Instructions**: `README.md` (setup, dependencies, commands)
- **Feature Specifications**: `specs/[###-feature]/spec.md` (requirements, acceptance criteria)

**Version**: 2.2.0 | **Ratified**: 2025-12-19 | **Last Amended**: 2025-12-21
