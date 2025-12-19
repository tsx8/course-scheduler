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
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (course_id, campus_id, venue_id),
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE,
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
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (course_id, campus_id, venue_id),
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE,
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
