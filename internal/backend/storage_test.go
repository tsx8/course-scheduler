package backend

import (
	"database/sql"
	"testing"

	_ "modernc.org/sqlite"
)

func TestInitDatabaseMigratesSchedulePlanIDBeforeIndexes(t *testing.T) {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatalf("open sqlite database: %v", err)
	}
	defer func() { _ = db.Close() }()

	_, err = db.Exec(`
CREATE TABLE scheduled_classes (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL
);

CREATE TABLE scheduled_classes_temp (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL
);

INSERT INTO scheduled_classes (id, teacher_id, course_id, day_id, time_id, campus_id, venue_id)
VALUES ('class-1', 'teacher-1', 'course-1', 'day-1', 'time-1', 'campus-1', 'venue-1');
`)
	if err != nil {
		t.Fatalf("seed legacy schema: %v", err)
	}

	if err := initDatabase(db); err != nil {
		t.Fatalf("migrate legacy database: %v", err)
	}

	for _, table := range []string{"scheduled_classes", "scheduled_classes_temp"} {
		hasColumn, err := columnExists(db, table, "schedule_plan_id")
		if err != nil {
			t.Fatalf("inspect %s: %v", table, err)
		}
		if !hasColumn {
			t.Fatalf("%s missing schedule_plan_id after migration", table)
		}
	}

	var planID string
	if err := db.QueryRow("SELECT schedule_plan_id FROM scheduled_classes WHERE id = 'class-1'").Scan(&planID); err != nil {
		t.Fatalf("load migrated schedule plan id: %v", err)
	}
	if planID != defaultSchedulePlanID {
		t.Fatalf("migrated schedule plan id = %q, want %q", planID, defaultSchedulePlanID)
	}

	for _, indexName := range []string{"idx_scheduled_classes_schedule_plan_id", "idx_scheduled_classes_temp_schedule_plan_id"} {
		var count int
		if err := db.QueryRow("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?", indexName).Scan(&count); err != nil {
			t.Fatalf("inspect index %s: %v", indexName, err)
		}
		if count != 1 {
			t.Fatalf("index %s count = %d, want 1", indexName, count)
		}
	}
}
