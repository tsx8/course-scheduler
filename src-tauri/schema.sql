-- SQLite Database Schema for Course Scheduler
-- Feature: 001-sqlite-migration
-- This file contains all table definitions for main and temp tables

-- Enable foreign key constraints
PRAGMA foreign_keys = ON;

-- ============================================================================
-- MAIN TABLES (Committed State)
-- ============================================================================

CREATE TABLE IF NOT EXISTS time_slots (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    corresponding_hours INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS days (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS campuses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS venues (
    id TEXT PRIMARY KEY,
    campus_id TEXT NOT NULL,
    name TEXT NOT NULL,
    capacity INTEGER NOT NULL,
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS courses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS course_venues (
    course_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (course_id, venue_id),
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teachers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    max_teaching_hours INTEGER NOT NULL,
    is_only_shahe INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS teacher_courses (
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, course_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teacher_unavailability (
    teacher_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, day_id, time_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheduled_classes (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS schedule_density (
    campus_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (campus_id, day_id, time_id),
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots(id) ON DELETE CASCADE
);

-- ============================================================================
-- RBAC AND AUDIT SYSTEM TABLES (Feature: 001-rbac-audit-system)
-- ============================================================================

-- Roles: Define user role types (main only - static seed data)
CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

-- Users: Authentication and authorization (dual-table pattern)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role_id TEXT NOT NULL,
    teacher_id TEXT,
    created_at TEXT NOT NULL,
    last_login TEXT,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE RESTRICT,
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE SET NULL
);

-- Audit Logs: System activity tracking (main table only - append-only)
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    target_table TEXT,
    target_id TEXT,
    timestamp TEXT NOT NULL,
    change_details TEXT,
    ip_address TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- TEMP TABLES (Working State - Identical Structure)
-- ============================================================================

CREATE TABLE IF NOT EXISTS time_slots_temp (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    corresponding_hours INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS days_temp (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS campuses_temp (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS venues_temp (
    id TEXT PRIMARY KEY,
    campus_id TEXT NOT NULL,
    name TEXT NOT NULL,
    capacity INTEGER NOT NULL,
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS courses_temp (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS course_venues_temp (
    course_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (course_id, venue_id),
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teachers_temp (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    max_teaching_hours INTEGER NOT NULL,
    is_only_shahe INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS teacher_courses_temp (
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, course_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teacher_unavailability_temp (
    teacher_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, day_id, time_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheduled_classes_temp (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS schedule_density_temp (
    campus_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (campus_id, day_id, time_id),
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (day_id) REFERENCES days_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (time_id) REFERENCES time_slots_temp(id) ON DELETE CASCADE
);

-- ============================================================================
-- PERFORMANCE INDEXES
-- ============================================================================

-- Main table indexes
CREATE INDEX IF NOT EXISTS idx_venues_campus_id ON venues(campus_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_course_id ON course_venues(course_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_venue_id ON course_venues(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_teacher_id ON teacher_courses(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_course_id ON teacher_courses(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_teacher_id ON scheduled_classes(teacher_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_course_id ON scheduled_classes(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_venue_id ON scheduled_classes(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_unavailability_teacher_id ON teacher_unavailability(teacher_id);
CREATE INDEX IF NOT EXISTS idx_schedule_density_campus_id ON schedule_density(campus_id);

-- Temp table indexes
CREATE INDEX IF NOT EXISTS idx_venues_temp_campus_id ON venues_temp(campus_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_temp_course_id ON course_venues_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_temp_venue_id ON course_venues_temp(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_temp_teacher_id ON teacher_courses_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_temp_course_id ON teacher_courses_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_teacher_id ON scheduled_classes_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_course_id ON scheduled_classes_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_venue_id ON scheduled_classes_temp(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_unavailability_temp_teacher_id ON teacher_unavailability_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_schedule_density_temp_campus_id ON schedule_density_temp(campus_id);

-- RBAC and Audit system indexes
CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role_id);
CREATE INDEX IF NOT EXISTS idx_users_teacher ON users(teacher_id);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_logs(action_type);
CREATE INDEX IF NOT EXISTS idx_audit_table ON audit_logs(target_table);
