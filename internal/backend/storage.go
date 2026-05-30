package backend

import (
	"database/sql"
	"fmt"
	"strings"

	"github.com/wailsapp/wails/v2/pkg/runtime"
)

type queryExecutor interface {
	Exec(query string, args ...any) (sql.Result, error)
	Query(query string, args ...any) (*sql.Rows, error)
	QueryRow(query string, args ...any) *sql.Row
}

const (
	defaultSchedulePlanID      = "default-schedule-plan"
	defaultSchedulePlanName    = "默认课表"
	activeSchedulePlanStateKey = "active_schedule_plan_id"
)

type scheduledClassLoadScope int

const (
	scheduledClassLoadActive scheduledClassLoadScope = iota
	scheduledClassLoadAll
)

func initDatabase(db *sql.DB) error {
	if _, err := db.Exec("PRAGMA foreign_keys = ON"); err != nil {
		return err
	}
	if _, err := db.Exec(schemaSQL); err != nil {
		return err
	}
	if err := migrateScheduledClassState(db); err != nil {
		return err
	}
	if err := migrateSchedulePlanStorage(db); err != nil {
		return err
	}
	if err := ensureSchedulePlanIndexes(db); err != nil {
		return err
	}
	return nil
}

func (a *App) HasUnsavedChanges() (bool, error) {
	return hasUnsavedChanges(a.db)
}

func (a *App) LoadData() (AllData, error) {
	return loadAllData(a.db)
}

func (a *App) SaveTempData(content AllData) error {
	tx, err := a.db.Begin()
	if err != nil {
		return fmt.Errorf("begin temp transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := writeActiveAllDataToTemp(tx, content); err != nil {
		return err
	}
	return tx.Commit()
}

func (a *App) saveImportedTempData(content AllData) error {
	tx, err := a.db.Begin()
	if err != nil {
		return fmt.Errorf("begin import temp transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := writeAllDataToTables(tx, content, true); err != nil {
		return err
	}
	return tx.Commit()
}

func (a *App) CommitData() error {
	hasChanges, err := hasUnsavedChanges(a.db)
	if err != nil {
		return err
	}
	if !hasChanges {
		return nil
	}

	tx, err := a.db.Begin()
	if err != nil {
		return fmt.Errorf("begin commit transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	content, err := loadAllDataForExport(tx, true)
	if err != nil {
		return err
	}
	if err := writeAllDataToTables(tx, content, false); err != nil {
		return err
	}
	if err := truncateAllTempTables(tx); err != nil {
		return err
	}
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("commit data: %w", err)
	}

	runtimeEventsEmit(a, "commit-completed")
	return nil
}

func (a *App) ClearTempData() error {
	return clearAllTempTables(a.db)
}

func (a *App) ListCommittedTeachers() ([]Teacher, error) {
	return loadTeachers(a.db, false)
}

func hasUnsavedChanges(conn queryExecutor) (bool, error) {
	tempTables := []string{
		"app_metadata_temp",
		"scheduled_classes_temp",
		"schedule_plans_temp",
		"teacher_unavailability_temp",
		"campus_filter_view_courses_temp",
		"campus_filter_view_teachers_temp",
		"campus_filter_view_venues_temp",
		"campus_filter_views_temp",
		"teacher_campuses_temp",
		"teacher_courses_temp",
		"teachers_temp",
		"course_venues_temp",
		"courses_temp",
		"schedule_density_temp",
		"venues_temp",
		"campuses_temp",
		"days_temp",
		"time_slots_temp",
	}

	for _, table := range tempTables {
		var count int
		query := fmt.Sprintf("SELECT COUNT(*) FROM %s", table)
		if err := conn.QueryRow(query).Scan(&count); err != nil {
			return false, fmt.Errorf("count %s: %w", table, err)
		}
		if count > 0 {
			return true, nil
		}
	}

	return false, nil
}

func loadAllData(conn queryExecutor) (AllData, error) {
	hasTempData, err := hasUnsavedChanges(conn)
	if err != nil {
		return AllData{}, fmt.Errorf("check temp data: %w", err)
	}
	return loadAllDataFromConnection(conn, hasTempData)
}

func loadAllDataFromConnection(conn queryExecutor, useTemp bool) (AllData, error) {
	return loadAllDataFromConnectionWithScheduleScope(conn, useTemp, scheduledClassLoadActive)
}

func loadAllDataForExport(conn queryExecutor, useTemp bool) (AllData, error) {
	return loadAllDataFromConnectionWithScheduleScope(conn, useTemp, scheduledClassLoadAll)
}

func loadAllDataFromConnectionWithScheduleScope(conn queryExecutor, useTemp bool, scope scheduledClassLoadScope) (AllData, error) {
	timeSlots, err := loadTimeSlots(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	days, err := loadDays(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	campuses, err := loadCampuses(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	venues, err := loadVenues(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	courses, err := loadCourses(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	teachers, err := loadTeachers(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	courseVenues, err := loadCourseVenues(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	teacherCourses, err := loadTeacherCourses(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	teacherCampuses, err := loadTeacherCampuses(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	schedulePlans, err := loadSchedulePlans(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	activeSchedulePlanID, err := loadActiveSchedulePlanID(conn, useTemp, schedulePlans)
	if err != nil {
		return AllData{}, err
	}
	scheduledClasses, err := loadScheduledClasses(conn, useTemp, activeSchedulePlanID, scope)
	if err != nil {
		return AllData{}, err
	}
	teacherUnavailability, err := loadTeacherUnavailability(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	scheduleDensity, err := loadScheduleDensity(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}
	campusFilterViews, err := loadCampusFilterViews(conn, useTemp)
	if err != nil {
		return AllData{}, err
	}

	return AllData{
		Time:                  timeSlots,
		Day:                   days,
		Campuses:              campuses,
		Venues:                venues,
		Courses:               courses,
		Teachers:              teachers,
		CourseVenues:          courseVenues,
		TeacherCourses:        teacherCourses,
		TeacherCampuses:       teacherCampuses,
		SchedulePlans:         schedulePlans,
		ActiveSchedulePlanID:  activeSchedulePlanID,
		ScheduledClasses:      scheduledClasses,
		TeacherUnavailability: teacherUnavailability,
		ScheduleDensity:       scheduleDensity,
		CampusFilterViews:     campusFilterViews,
	}, nil
}

func loadTimeSlots(conn queryExecutor, useTemp bool) ([]TimeSlot, error) {
	table := tableName("time_slots", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, value, corresponding_hours FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	slots := []TimeSlot{}
	for rows.Next() {
		var slot TimeSlot
		if err := rows.Scan(&slot.ID, &slot.Value, &slot.CorrespondingHours); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		slots = append(slots, slot)
	}
	return slots, rows.Err()
}

func loadDays(conn queryExecutor, useTemp bool) ([]Day, error) {
	table := tableName("days", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, value FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	days := []Day{}
	for rows.Next() {
		var day Day
		if err := rows.Scan(&day.ID, &day.Value); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		days = append(days, day)
	}
	return days, rows.Err()
}

func loadCampuses(conn queryExecutor, useTemp bool) ([]Campus, error) {
	table := tableName("campuses", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, name FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	campuses := []Campus{}
	for rows.Next() {
		var campus Campus
		if err := rows.Scan(&campus.ID, &campus.Name); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		campuses = append(campuses, campus)
	}
	return campuses, rows.Err()
}

func loadVenues(conn queryExecutor, useTemp bool) ([]Venue, error) {
	table := tableName("venues", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, campus_id, name, capacity FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	venues := []Venue{}
	for rows.Next() {
		var venue Venue
		if err := rows.Scan(&venue.ID, &venue.CampusID, &venue.Name, &venue.Capacity); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		venues = append(venues, venue)
	}
	return venues, rows.Err()
}

func loadCourses(conn queryExecutor, useTemp bool) ([]Course, error) {
	table := tableName("courses", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, name FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	courses := []Course{}
	for rows.Next() {
		var course Course
		if err := rows.Scan(&course.ID, &course.Name); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		courses = append(courses, course)
	}
	return courses, rows.Err()
}

func loadCourseVenues(conn queryExecutor, useTemp bool) ([]CourseVenue, error) {
	table := tableName("course_venues", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT course_id, venue_id FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	relations := []CourseVenue{}
	for rows.Next() {
		var relation CourseVenue
		if err := rows.Scan(&relation.CourseID, &relation.VenueID); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		relations = append(relations, relation)
	}
	return relations, rows.Err()
}

func loadTeachers(conn queryExecutor, useTemp bool) ([]Teacher, error) {
	table := tableName("teachers", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, name, max_teaching_hours FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	teachers := []Teacher{}
	for rows.Next() {
		var teacher Teacher
		if err := rows.Scan(&teacher.ID, &teacher.Name, &teacher.MaxTeachingHours); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		teachers = append(teachers, teacher)
	}
	return teachers, rows.Err()
}

func tableExists(conn queryExecutor, table string) (bool, error) {
	var exists bool
	err := conn.QueryRow("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)", table).Scan(&exists)
	if err != nil {
		return false, fmt.Errorf("inspect table %s: %w", table, err)
	}
	return exists, nil
}

func columnExists(conn queryExecutor, table string, column string) (bool, error) {
	rows, err := conn.Query(fmt.Sprintf("PRAGMA table_info(%s)", table))
	if err != nil {
		return false, fmt.Errorf("inspect columns for %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	for rows.Next() {
		var cid int
		var name string
		var valueType string
		var notNull int
		var defaultValue any
		var primaryKey int
		if err := rows.Scan(&cid, &name, &valueType, &notNull, &defaultValue, &primaryKey); err != nil {
			return false, fmt.Errorf("scan columns for %s: %w", table, err)
		}
		if name == column {
			return true, nil
		}
	}
	return false, rows.Err()
}
func boolToInt(value bool) int {
	if value {
		return 1
	}
	return 0
}

func migrateScheduledClassState(conn queryExecutor) error {
	for _, table := range []string{"scheduled_classes", "scheduled_classes_temp"} {
		exists, err := tableExists(conn, table)
		if err != nil {
			return err
		}
		if !exists {
			continue
		}

		hasIsLocked, err := columnExists(conn, table, "is_locked")
		if err != nil {
			return err
		}
		if !hasIsLocked {
			if _, err := conn.Exec(fmt.Sprintf("ALTER TABLE %s ADD COLUMN is_locked INTEGER NOT NULL DEFAULT 0", table)); err != nil {
				return fmt.Errorf("add is_locked to %s: %w", table, err)
			}
			if _, err := conn.Exec(fmt.Sprintf("UPDATE %s SET is_locked = 1", table)); err != nil {
				return fmt.Errorf("default migrated is_locked for %s: %w", table, err)
			}
		}

		for _, column := range []string{"is_staged", "staged_order"} {
			hasColumn, err := columnExists(conn, table, column)
			if err != nil {
				return err
			}
			if hasColumn {
				continue
			}
			if _, err := conn.Exec(fmt.Sprintf("ALTER TABLE %s ADD COLUMN %s INTEGER NOT NULL DEFAULT 0", table, column)); err != nil {
				return fmt.Errorf("add %s to %s: %w", column, table, err)
			}
		}
	}
	return nil
}

func migrateSchedulePlanStorage(conn queryExecutor) error {
	for _, table := range []string{"scheduled_classes", "scheduled_classes_temp"} {
		exists, err := tableExists(conn, table)
		if err != nil {
			return err
		}
		if !exists {
			continue
		}

		hasColumn, err := columnExists(conn, table, "schedule_plan_id")
		if err != nil {
			return err
		}
		if !hasColumn {
			if _, err := conn.Exec(fmt.Sprintf("ALTER TABLE %s ADD COLUMN schedule_plan_id TEXT", table)); err != nil {
				return fmt.Errorf("add schedule_plan_id to %s: %w", table, err)
			}
		}
	}

	hasTempData, err := hasUnsavedChanges(conn)
	if err != nil {
		return err
	}
	if err := ensureDefaultSchedulePlan(conn, false); err != nil {
		return err
	}
	if hasTempData {
		if err := ensureDefaultSchedulePlan(conn, true); err != nil {
			return err
		}
	}
	return nil
}

func ensureSchedulePlanIndexes(conn queryExecutor) error {
	for _, index := range []struct {
		name  string
		table string
	}{
		{name: "idx_scheduled_classes_schedule_plan_id", table: "scheduled_classes"},
		{name: "idx_scheduled_classes_temp_schedule_plan_id", table: "scheduled_classes_temp"},
	} {
		if _, err := conn.Exec(fmt.Sprintf(
			"CREATE INDEX IF NOT EXISTS %s ON %s(schedule_plan_id)",
			index.name,
			index.table,
		)); err != nil {
			return fmt.Errorf("create %s: %w", index.name, err)
		}
	}
	return nil
}

func ensureDefaultSchedulePlan(conn queryExecutor, useTemp bool) error {
	planTable := tableName("schedule_plans", useTemp)
	stateTable := tableName("app_metadata", useTemp)
	scheduleTable := tableName("scheduled_classes", useTemp)

	var planCount int
	if err := conn.QueryRow(fmt.Sprintf("SELECT COUNT(*) FROM %s", planTable)).Scan(&planCount); err != nil {
		return fmt.Errorf("count schedule plans: %w", err)
	}
	if planCount == 0 {
		if _, err := conn.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name, sort_order) VALUES (?, ?, 0)", planTable),
			defaultSchedulePlanID,
			defaultSchedulePlanName,
		); err != nil {
			return fmt.Errorf("ensure default schedule plan: %w", err)
		}
	}

	plans, err := loadSchedulePlans(conn, useTemp)
	if err != nil {
		return err
	}
	activeID, err := loadActiveSchedulePlanID(conn, useTemp, plans)
	if err != nil {
		return err
	}

	if _, err := conn.Exec(
		fmt.Sprintf("UPDATE %s SET schedule_plan_id = ? WHERE schedule_plan_id IS NULL OR schedule_plan_id = ''", scheduleTable),
		activeID,
	); err != nil {
		return fmt.Errorf("migrate scheduled class plan ids: %w", err)
	}

	if _, err := conn.Exec(
		fmt.Sprintf("INSERT OR REPLACE INTO %s (key, value) VALUES (?, ?)", stateTable),
		activeSchedulePlanStateKey,
		activeID,
	); err != nil {
		return fmt.Errorf("ensure active schedule plan: %w", err)
	}
	return nil
}

func loadTeacherCampusesFromLegacy(conn queryExecutor, useTemp bool) ([]TeacherCampus, error) {
	teacherTable := tableName("teachers", useTemp)
	campusTable := tableName("campuses", useTemp)

	hasColumn, err := columnExists(conn, teacherTable, "is_only_shahe")
	if err != nil {
		return nil, err
	}
	if !hasColumn {
		return []TeacherCampus{}, nil
	}

	campusRows, err := conn.Query(fmt.Sprintf("SELECT id, name FROM %s", campusTable))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", campusTable, err)
	}
	defer func() { _ = campusRows.Close() }()

	var campuses []Campus
	for campusRows.Next() {
		var campus Campus
		if err := campusRows.Scan(&campus.ID, &campus.Name); err != nil {
			return nil, fmt.Errorf("scan legacy campuses: %w", err)
		}
		campuses = append(campuses, campus)
	}
	if err := campusRows.Err(); err != nil {
		return nil, err
	}

	allCampusIDs := make([]string, 0, len(campuses))
	shaheCampusIDs := []string{}
	for _, campus := range campuses {
		allCampusIDs = append(allCampusIDs, campus.ID)
		if campus.ID == "138697dc-1591-4c16-b60e-d0057964be56" || strings.Contains(campus.Name, "沙河") {
			shaheCampusIDs = append(shaheCampusIDs, campus.ID)
		}
	}
	if len(shaheCampusIDs) == 0 {
		shaheCampusIDs = append(shaheCampusIDs, allCampusIDs...)
	}

	rows, err := conn.Query(fmt.Sprintf("SELECT id, is_only_shahe FROM %s", teacherTable))
	if err != nil {
		return nil, fmt.Errorf("query legacy teacher campuses: %w", err)
	}
	defer func() { _ = rows.Close() }()

	relations := []TeacherCampus{}
	for rows.Next() {
		var teacherID string
		var isOnlyShahe int
		if err := rows.Scan(&teacherID, &isOnlyShahe); err != nil {
			return nil, fmt.Errorf("scan legacy teacher campus: %w", err)
		}
		campusIDs := allCampusIDs
		if isOnlyShahe != 0 {
			campusIDs = shaheCampusIDs
		}
		for _, campusID := range campusIDs {
			relations = append(relations, TeacherCampus{TeacherID: teacherID, CampusID: campusID})
		}
	}
	return relations, rows.Err()
}

func migrateTeacherCampuses(conn queryExecutor) error {
	for _, config := range []struct {
		teacherTable  string
		relationTable string
		useTemp       bool
	}{
		{teacherTable: "teachers", relationTable: "teacher_campuses", useTemp: false},
		{teacherTable: "teachers_temp", relationTable: "teacher_campuses_temp", useTemp: true},
	} {
		hasRelationTable, err := tableExists(conn, config.relationTable)
		if err != nil {
			return err
		}
		hasLegacyColumn, err := columnExists(conn, config.teacherTable, "is_only_shahe")
		if err != nil {
			return err
		}
		if !hasRelationTable || !hasLegacyColumn {
			continue
		}

		var relationCount int
		if err := conn.QueryRow(fmt.Sprintf("SELECT COUNT(*) FROM %s", config.relationTable)).Scan(&relationCount); err != nil {
			return fmt.Errorf("count %s: %w", config.relationTable, err)
		}
		if relationCount > 0 {
			continue
		}

		legacyRelations, err := loadTeacherCampusesFromLegacy(conn, config.useTemp)
		if err != nil {
			return err
		}
		for _, relation := range legacyRelations {
			if _, err := conn.Exec(
				fmt.Sprintf("INSERT OR IGNORE INTO %s (teacher_id, campus_id) VALUES (?, ?)", config.relationTable),
				relation.TeacherID,
				relation.CampusID,
			); err != nil {
				return fmt.Errorf("migrate %s: %w", config.relationTable, err)
			}
		}
	}
	return nil
}

func loadTeacherCampuses(conn queryExecutor, useTemp bool) ([]TeacherCampus, error) {
	table := tableName("teacher_campuses", useTemp)
	exists, err := tableExists(conn, table)
	if err != nil {
		return nil, err
	}
	if exists {
		rows, err := conn.Query(fmt.Sprintf("SELECT teacher_id, campus_id FROM %s", table))
		if err != nil {
			return nil, fmt.Errorf("query %s: %w", table, err)
		}
		defer func() { _ = rows.Close() }()

		relations := []TeacherCampus{}
		for rows.Next() {
			var relation TeacherCampus
			if err := rows.Scan(&relation.TeacherID, &relation.CampusID); err != nil {
				return nil, fmt.Errorf("scan %s: %w", table, err)
			}
			relations = append(relations, relation)
		}
		if err := rows.Err(); err != nil {
			return nil, err
		}
		if len(relations) > 0 {
			return relations, nil
		}
	}

	return loadTeacherCampusesFromLegacy(conn, useTemp)
}

func loadTeacherCourses(conn queryExecutor, useTemp bool) ([]TeacherCourse, error) {
	table := tableName("teacher_courses", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT teacher_id, course_id FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	relations := []TeacherCourse{}
	for rows.Next() {
		var relation TeacherCourse
		if err := rows.Scan(&relation.TeacherID, &relation.CourseID); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		relations = append(relations, relation)
	}
	return relations, rows.Err()
}

func loadSchedulePlans(conn queryExecutor, useTemp bool) ([]SchedulePlan, error) {
	table := tableName("schedule_plans", useTemp)
	exists, err := tableExists(conn, table)
	if err != nil {
		return nil, err
	}
	if !exists {
		return []SchedulePlan{{ID: defaultSchedulePlanID, Name: defaultSchedulePlanName, SortOrder: 0}}, nil
	}

	rows, err := conn.Query(fmt.Sprintf("SELECT id, name, sort_order FROM %s ORDER BY sort_order, name, id", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	plans := []SchedulePlan{}
	for rows.Next() {
		var plan SchedulePlan
		if err := rows.Scan(&plan.ID, &plan.Name, &plan.SortOrder); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		plans = append(plans, plan)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	if len(plans) == 0 {
		return []SchedulePlan{{ID: defaultSchedulePlanID, Name: defaultSchedulePlanName, SortOrder: 0}}, nil
	}
	return plans, nil
}

func loadActiveSchedulePlanID(conn queryExecutor, useTemp bool, plans []SchedulePlan) (string, error) {
	if len(plans) == 0 {
		return defaultSchedulePlanID, nil
	}

	validPlanIDs := make(map[string]struct{}, len(plans))
	for _, plan := range plans {
		validPlanIDs[plan.ID] = struct{}{}
	}

	table := tableName("app_metadata", useTemp)
	exists, err := tableExists(conn, table)
	if err != nil {
		return "", err
	}
	if exists {
		var activeID string
		err := conn.QueryRow(fmt.Sprintf("SELECT value FROM %s WHERE key = ?", table), activeSchedulePlanStateKey).Scan(&activeID)
		if err != nil && err != sql.ErrNoRows {
			return "", fmt.Errorf("load active schedule plan: %w", err)
		}
		if _, ok := validPlanIDs[activeID]; ok {
			return activeID, nil
		}
	}

	return plans[0].ID, nil
}

func loadScheduledClasses(conn queryExecutor, useTemp bool, activeSchedulePlanID string, scope scheduledClassLoadScope) ([]ScheduledClass, error) {
	table := tableName("scheduled_classes", useTemp)
	schedulePlanSelect := fmt.Sprintf("'%s'", defaultSchedulePlanID)
	hasSchedulePlanID, err := columnExists(conn, table, "schedule_plan_id")
	if err != nil {
		return nil, err
	}
	if hasSchedulePlanID {
		schedulePlanSelect = "schedule_plan_id"
	}

	isLockedSelect := "1"
	hasIsLocked, err := columnExists(conn, table, "is_locked")
	if err != nil {
		return nil, err
	}
	if hasIsLocked {
		isLockedSelect = "is_locked"
	}

	isStagedSelect := "0"
	hasIsStaged, err := columnExists(conn, table, "is_staged")
	if err != nil {
		return nil, err
	}
	if hasIsStaged {
		isStagedSelect = "is_staged"
	}

	stagedOrderSelect := "0"
	hasStagedOrder, err := columnExists(conn, table, "staged_order")
	if err != nil {
		return nil, err
	}
	if hasStagedOrder {
		stagedOrderSelect = "staged_order"
	}

	whereClause := ""
	args := []any{}
	if scope == scheduledClassLoadActive && hasSchedulePlanID {
		whereClause = " WHERE schedule_plan_id = ?"
		args = append(args, activeSchedulePlanID)
	}

	rows, err := conn.Query(fmt.Sprintf(
		"SELECT id, %s, teacher_id, course_id, day_id, time_id, campus_id, venue_id, %s, %s, %s FROM %s%s",
		schedulePlanSelect,
		isLockedSelect,
		isStagedSelect,
		stagedOrderSelect,
		table,
		whereClause,
	), args...)
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	classes := []ScheduledClass{}
	for rows.Next() {
		var class ScheduledClass
		var isLocked int
		var isStaged int
		if err := rows.Scan(&class.ID, &class.SchedulePlanID, &class.TeacherID, &class.CourseID, &class.DayID, &class.TimeID, &class.CampusID, &class.VenueID, &isLocked, &isStaged, &class.StagedOrder); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		if class.SchedulePlanID == "" {
			class.SchedulePlanID = activeSchedulePlanID
		}
		class.IsLocked = isLocked != 0
		class.IsStaged = isStaged != 0
		classes = append(classes, class)
	}
	return classes, rows.Err()
}

func loadTeacherUnavailability(conn queryExecutor, useTemp bool) ([]TeacherUnavailability, error) {
	table := tableName("teacher_unavailability", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT teacher_id, day_id, time_id FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	unavailability := []TeacherUnavailability{}
	for rows.Next() {
		var item TeacherUnavailability
		if err := rows.Scan(&item.TeacherID, &item.DayID, &item.TimeID); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		unavailability = append(unavailability, item)
	}
	return unavailability, rows.Err()
}

func loadScheduleDensity(conn queryExecutor, useTemp bool) ([]ScheduleDensity, error) {
	table := tableName("schedule_density", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT campus_id, day_id, time_id, count FROM %s", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	density := []ScheduleDensity{}
	for rows.Next() {
		var item ScheduleDensity
		if err := rows.Scan(&item.CampusID, &item.DayID, &item.TimeID, &item.Count); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		density = append(density, item)
	}
	return density, rows.Err()
}

func loadCampusFilterViews(conn queryExecutor, useTemp bool) ([]CampusFilterView, error) {
	table := tableName("campus_filter_views", useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT id, name, campus_id, sort_order FROM %s ORDER BY sort_order, name, id", table))
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	views := []CampusFilterView{}
	for rows.Next() {
		var view CampusFilterView
		if err := rows.Scan(&view.ID, &view.Name, &view.CampusID, &view.SortOrder); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		views = append(views, view)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	for index := range views {
		views[index].VenueIDs, err = loadCampusFilterViewRelationIDs(conn, useTemp, "campus_filter_view_venues", "venue_id", views[index].ID)
		if err != nil {
			return nil, err
		}
		views[index].TeacherIDs, err = loadCampusFilterViewRelationIDs(conn, useTemp, "campus_filter_view_teachers", "teacher_id", views[index].ID)
		if err != nil {
			return nil, err
		}
		views[index].CourseIDs, err = loadCampusFilterViewRelationIDs(conn, useTemp, "campus_filter_view_courses", "course_id", views[index].ID)
		if err != nil {
			return nil, err
		}
	}

	return views, nil
}

func loadCampusFilterViewRelationIDs(conn queryExecutor, useTemp bool, baseTable string, column string, viewID string) ([]string, error) {
	table := tableName(baseTable, useTemp)
	rows, err := conn.Query(fmt.Sprintf("SELECT %s FROM %s WHERE view_id = ? ORDER BY %s", column, table, column), viewID)
	if err != nil {
		return nil, fmt.Errorf("query %s: %w", table, err)
	}
	defer func() { _ = rows.Close() }()

	ids := []string{}
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, fmt.Errorf("scan %s: %w", table, err)
		}
		ids = append(ids, id)
	}
	return ids, rows.Err()
}

func normalizeSchedulePlanData(data *AllData) {
	plans := make([]SchedulePlan, 0, len(data.SchedulePlans))
	seenPlanIDs := map[string]struct{}{}
	for index, plan := range data.SchedulePlans {
		plan.ID = strings.TrimSpace(plan.ID)
		plan.Name = strings.TrimSpace(plan.Name)
		if plan.ID == "" {
			continue
		}
		if _, exists := seenPlanIDs[plan.ID]; exists {
			continue
		}
		if plan.Name == "" {
			plan.Name = defaultSchedulePlanName
		}
		if plan.SortOrder == 0 && index > 0 {
			plan.SortOrder = index
		}
		seenPlanIDs[plan.ID] = struct{}{}
		plans = append(plans, plan)
	}
	if len(plans) == 0 {
		plans = append(plans, SchedulePlan{ID: defaultSchedulePlanID, Name: defaultSchedulePlanName, SortOrder: 0})
		seenPlanIDs[defaultSchedulePlanID] = struct{}{}
	}

	activeID := strings.TrimSpace(data.ActiveSchedulePlanID)
	if _, ok := seenPlanIDs[activeID]; !ok {
		activeID = plans[0].ID
	}

	for index := range data.ScheduledClasses {
		planID := strings.TrimSpace(data.ScheduledClasses[index].SchedulePlanID)
		if _, ok := seenPlanIDs[planID]; !ok {
			planID = activeID
		}
		data.ScheduledClasses[index].SchedulePlanID = planID
	}

	data.SchedulePlans = plans
	data.ActiveSchedulePlanID = activeID
}

type scheduleReferenceSet struct {
	teacherIDs map[string]struct{}
	courseIDs  map[string]struct{}
	dayIDs     map[string]struct{}
	timeIDs    map[string]struct{}
	campusIDs  map[string]struct{}
	venueIDs   map[string]struct{}
}

func buildScheduleReferenceSet(data AllData) scheduleReferenceSet {
	refs := scheduleReferenceSet{
		teacherIDs: map[string]struct{}{},
		courseIDs:  map[string]struct{}{},
		dayIDs:     map[string]struct{}{},
		timeIDs:    map[string]struct{}{},
		campusIDs:  map[string]struct{}{},
		venueIDs:   map[string]struct{}{},
	}
	for _, teacher := range data.Teachers {
		refs.teacherIDs[teacher.ID] = struct{}{}
	}
	for _, course := range data.Courses {
		refs.courseIDs[course.ID] = struct{}{}
	}
	for _, day := range data.Day {
		refs.dayIDs[day.ID] = struct{}{}
	}
	for _, slot := range data.Time {
		refs.timeIDs[slot.ID] = struct{}{}
	}
	for _, campus := range data.Campuses {
		refs.campusIDs[campus.ID] = struct{}{}
	}
	for _, venue := range data.Venues {
		refs.venueIDs[venue.ID] = struct{}{}
	}
	return refs
}

func hasReference(references map[string]struct{}, id string) bool {
	_, ok := references[id]
	return ok
}

func scheduleClassReferencesExist(class ScheduledClass, refs scheduleReferenceSet) bool {
	return hasReference(refs.teacherIDs, class.TeacherID) &&
		hasReference(refs.courseIDs, class.CourseID) &&
		hasReference(refs.dayIDs, class.DayID) &&
		hasReference(refs.timeIDs, class.TimeID) &&
		hasReference(refs.campusIDs, class.CampusID) &&
		hasReference(refs.venueIDs, class.VenueID)
}

func schedulePlanIDSet(plans []SchedulePlan) map[string]struct{} {
	planIDs := make(map[string]struct{}, len(plans))
	for _, plan := range plans {
		planIDs[plan.ID] = struct{}{}
	}
	return planIDs
}

func writeActiveAllDataToTemp(tx *sql.Tx, content AllData) error {
	data := content
	normalizeSchedulePlanData(&data)

	sourceUseTemp, err := hasUnsavedChanges(tx)
	if err != nil {
		return err
	}

	existingSchedules, err := loadScheduledClasses(tx, sourceUseTemp, data.ActiveSchedulePlanID, scheduledClassLoadAll)
	if err != nil {
		return err
	}

	validPlanIDs := schedulePlanIDSet(data.SchedulePlans)
	refs := buildScheduleReferenceSet(data)
	nextSchedules := make([]ScheduledClass, 0, len(data.ScheduledClasses)+len(existingSchedules))
	for _, class := range data.ScheduledClasses {
		class.SchedulePlanID = data.ActiveSchedulePlanID
		nextSchedules = append(nextSchedules, class)
	}
	for _, class := range existingSchedules {
		if class.SchedulePlanID == data.ActiveSchedulePlanID {
			continue
		}
		if _, ok := validPlanIDs[class.SchedulePlanID]; !ok {
			continue
		}
		if !scheduleClassReferencesExist(class, refs) {
			continue
		}
		nextSchedules = append(nextSchedules, class)
	}

	data.ScheduledClasses = nextSchedules
	return writeAllDataToTables(tx, data, true)
}

func writeAllDataToTables(tx *sql.Tx, data AllData, useTemp bool) error {
	normalizeSchedulePlanData(&data)

	if err := clearTables(tx, useTemp); err != nil {
		return err
	}

	for _, slot := range data.Time {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, value, corresponding_hours) VALUES (?, ?, ?)", tableName("time_slots", useTemp)),
			slot.ID,
			slot.Value,
			slot.CorrespondingHours,
		); err != nil {
			return fmt.Errorf("insert time slot: %w", err)
		}
	}

	for _, day := range data.Day {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, value) VALUES (?, ?)", tableName("days", useTemp)),
			day.ID,
			day.Value,
		); err != nil {
			return fmt.Errorf("insert day: %w", err)
		}
	}

	for _, campus := range data.Campuses {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name) VALUES (?, ?)", tableName("campuses", useTemp)),
			campus.ID,
			campus.Name,
		); err != nil {
			return fmt.Errorf("insert campus: %w", err)
		}
	}

	for _, venue := range data.Venues {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, campus_id, name, capacity) VALUES (?, ?, ?, ?)", tableName("venues", useTemp)),
			venue.ID,
			venue.CampusID,
			venue.Name,
			venue.Capacity,
		); err != nil {
			return fmt.Errorf("insert venue: %w", err)
		}
	}

	for _, course := range data.Courses {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name) VALUES (?, ?)", tableName("courses", useTemp)),
			course.ID,
			course.Name,
		); err != nil {
			return fmt.Errorf("insert course: %w", err)
		}
	}

	for _, relation := range data.CourseVenues {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (course_id, venue_id) VALUES (?, ?)", tableName("course_venues", useTemp)),
			relation.CourseID,
			relation.VenueID,
		); err != nil {
			return fmt.Errorf("insert course venue: %w", err)
		}
	}

	for _, teacher := range data.Teachers {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name, max_teaching_hours) VALUES (?, ?, ?)", tableName("teachers", useTemp)),
			teacher.ID,
			teacher.Name,
			teacher.MaxTeachingHours,
		); err != nil {
			return fmt.Errorf("insert teacher: %w", err)
		}
	}

	for _, relation := range data.TeacherCourses {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (teacher_id, course_id) VALUES (?, ?)", tableName("teacher_courses", useTemp)),
			relation.TeacherID,
			relation.CourseID,
		); err != nil {
			return fmt.Errorf("insert teacher course: %w", err)
		}
	}

	for _, relation := range data.TeacherCampuses {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (teacher_id, campus_id) VALUES (?, ?)", tableName("teacher_campuses", useTemp)),
			relation.TeacherID,
			relation.CampusID,
		); err != nil {
			return fmt.Errorf("insert teacher campus: %w", err)
		}
	}

	for _, plan := range data.SchedulePlans {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name, sort_order) VALUES (?, ?, ?)", tableName("schedule_plans", useTemp)),
			plan.ID,
			plan.Name,
			plan.SortOrder,
		); err != nil {
			return fmt.Errorf("insert schedule plan: %w", err)
		}
	}

	if _, err := tx.Exec(
		fmt.Sprintf("INSERT INTO %s (key, value) VALUES (?, ?)", tableName("app_metadata", useTemp)),
		activeSchedulePlanStateKey,
		data.ActiveSchedulePlanID,
	); err != nil {
		return fmt.Errorf("insert active schedule plan state: %w", err)
	}

	for _, view := range data.CampusFilterViews {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, name, campus_id, sort_order) VALUES (?, ?, ?, ?)", tableName("campus_filter_views", useTemp)),
			view.ID,
			view.Name,
			view.CampusID,
			view.SortOrder,
		); err != nil {
			return fmt.Errorf("insert campus filter view: %w", err)
		}

		if err := insertCampusFilterViewRelationIDs(tx, useTemp, "campus_filter_view_venues", "venue_id", view.ID, view.VenueIDs); err != nil {
			return err
		}
		if err := insertCampusFilterViewRelationIDs(tx, useTemp, "campus_filter_view_teachers", "teacher_id", view.ID, view.TeacherIDs); err != nil {
			return err
		}
		if err := insertCampusFilterViewRelationIDs(tx, useTemp, "campus_filter_view_courses", "course_id", view.ID, view.CourseIDs); err != nil {
			return err
		}
	}

	for _, item := range data.TeacherUnavailability {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (teacher_id, day_id, time_id) VALUES (?, ?, ?)", tableName("teacher_unavailability", useTemp)),
			item.TeacherID,
			item.DayID,
			item.TimeID,
		); err != nil {
			return fmt.Errorf("insert teacher unavailability: %w", err)
		}
	}

	for _, class := range data.ScheduledClasses {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (id, schedule_plan_id, teacher_id, course_id, day_id, time_id, campus_id, venue_id, is_locked, is_staged, staged_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", tableName("scheduled_classes", useTemp)),
			class.ID,
			class.SchedulePlanID,
			class.TeacherID,
			class.CourseID,
			class.DayID,
			class.TimeID,
			class.CampusID,
			class.VenueID,
			boolToInt(class.IsLocked),
			boolToInt(class.IsStaged),
			class.StagedOrder,
		); err != nil {
			return fmt.Errorf("insert scheduled class: %w", err)
		}
	}

	for _, item := range data.ScheduleDensity {
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (campus_id, day_id, time_id, count) VALUES (?, ?, ?, ?)", tableName("schedule_density", useTemp)),
			item.CampusID,
			item.DayID,
			item.TimeID,
			item.Count,
		); err != nil {
			return fmt.Errorf("insert schedule density: %w", err)
		}
	}

	return nil
}

func insertCampusFilterViewRelationIDs(tx *sql.Tx, useTemp bool, baseTable string, column string, viewID string, ids []string) error {
	seen := make(map[string]struct{}, len(ids))
	for _, id := range ids {
		if id == "" {
			continue
		}
		if _, exists := seen[id]; exists {
			continue
		}
		seen[id] = struct{}{}
		if _, err := tx.Exec(
			fmt.Sprintf("INSERT INTO %s (view_id, %s) VALUES (?, ?)", tableName(baseTable, useTemp), column),
			viewID,
			id,
		); err != nil {
			return fmt.Errorf("insert campus filter view relation: %w", err)
		}
	}
	return nil
}

func clearTables(tx *sql.Tx, useTemp bool) error {
	tables := []string{
		tableName("app_metadata", useTemp),
		tableName("scheduled_classes", useTemp),
		tableName("schedule_plans", useTemp),
		tableName("teacher_unavailability", useTemp),
		tableName("campus_filter_view_courses", useTemp),
		tableName("campus_filter_view_teachers", useTemp),
		tableName("campus_filter_view_venues", useTemp),
		tableName("campus_filter_views", useTemp),
		tableName("teacher_campuses", useTemp),
		tableName("teacher_courses", useTemp),
		tableName("teachers", useTemp),
		tableName("course_venues", useTemp),
		tableName("courses", useTemp),
		tableName("schedule_density", useTemp),
		tableName("venues", useTemp),
		tableName("campuses", useTemp),
		tableName("days", useTemp),
		tableName("time_slots", useTemp),
	}

	for _, table := range tables {
		if _, err := tx.Exec(fmt.Sprintf("DELETE FROM %s", table)); err != nil {
			return fmt.Errorf("clear table %s: %w", table, err)
		}
	}
	return nil
}

func truncateAllTempTables(tx *sql.Tx) error {
	tables := []string{
		"app_metadata_temp",
		"scheduled_classes_temp",
		"schedule_plans_temp",
		"teacher_unavailability_temp",
		"campus_filter_view_courses_temp",
		"campus_filter_view_teachers_temp",
		"campus_filter_view_venues_temp",
		"campus_filter_views_temp",
		"teacher_campuses_temp",
		"teacher_courses_temp",
		"teachers_temp",
		"course_venues_temp",
		"courses_temp",
		"schedule_density_temp",
		"venues_temp",
		"campuses_temp",
		"days_temp",
		"time_slots_temp",
	}

	for _, table := range tables {
		if _, err := tx.Exec(fmt.Sprintf("DELETE FROM %s", table)); err != nil {
			return fmt.Errorf("truncate temp table %s: %w", table, err)
		}
	}
	return nil
}

func clearAllTempTables(conn queryExecutor) error {
	tables := []string{
		"app_metadata_temp",
		"scheduled_classes_temp",
		"schedule_plans_temp",
		"teacher_unavailability_temp",
		"campus_filter_view_courses_temp",
		"campus_filter_view_teachers_temp",
		"campus_filter_view_venues_temp",
		"campus_filter_views_temp",
		"teacher_campuses_temp",
		"teacher_courses_temp",
		"teachers_temp",
		"course_venues_temp",
		"courses_temp",
		"schedule_density_temp",
		"venues_temp",
		"campuses_temp",
		"days_temp",
		"time_slots_temp",
	}

	for _, table := range tables {
		if _, err := conn.Exec(fmt.Sprintf("DELETE FROM %s", table)); err != nil {
			return fmt.Errorf("clear temp table %s: %w", table, err)
		}
	}
	return nil
}

func tableName(base string, useTemp bool) string {
	if useTemp {
		return base + "_temp"
	}
	return base
}

func runtimeEventsEmit(app *App, eventName string) {
	if app != nil && app.ctx != nil {
		runtime.EventsEmit(app.ctx, eventName)
	}
}

const schemaSQL = `PRAGMA foreign_keys = ON;

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
    max_teaching_hours INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS teacher_courses (
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, course_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teacher_campuses (
    teacher_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, campus_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE
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

CREATE TABLE IF NOT EXISTS schedule_plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scheduled_classes (
    id TEXT PRIMARY KEY,
    schedule_plan_id TEXT NOT NULL DEFAULT 'default-schedule-plan',
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    is_locked INTEGER NOT NULL DEFAULT 0,
    is_staged INTEGER NOT NULL DEFAULT 0,
    staged_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (schedule_plan_id) REFERENCES schedule_plans(id) ON DELETE CASCADE,
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

CREATE TABLE IF NOT EXISTS campus_filter_views (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (campus_id) REFERENCES campuses(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_venues (
    view_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (view_id, venue_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_teachers (
    view_id TEXT NOT NULL,
    teacher_id TEXT NOT NULL,
    PRIMARY KEY (view_id, teacher_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views(id) ON DELETE CASCADE,
    FOREIGN KEY (teacher_id) REFERENCES teachers(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_courses (
    view_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (view_id, course_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
);

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
    max_teaching_hours INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS teacher_courses_temp (
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, course_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS teacher_campuses_temp (
    teacher_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    PRIMARY KEY (teacher_id, campus_id),
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE
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

CREATE TABLE IF NOT EXISTS schedule_plans_temp (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_metadata_temp (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scheduled_classes_temp (
    id TEXT PRIMARY KEY,
    schedule_plan_id TEXT NOT NULL DEFAULT 'default-schedule-plan',
    teacher_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    day_id TEXT NOT NULL,
    time_id TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    is_locked INTEGER NOT NULL DEFAULT 0,
    is_staged INTEGER NOT NULL DEFAULT 0,
    staged_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (schedule_plan_id) REFERENCES schedule_plans_temp(id) ON DELETE CASCADE,
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

CREATE TABLE IF NOT EXISTS campus_filter_views_temp (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    campus_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (campus_id) REFERENCES campuses_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_venues_temp (
    view_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    PRIMARY KEY (view_id, venue_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (venue_id) REFERENCES venues_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_teachers_temp (
    view_id TEXT NOT NULL,
    teacher_id TEXT NOT NULL,
    PRIMARY KEY (view_id, teacher_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (teacher_id) REFERENCES teachers_temp(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS campus_filter_view_courses_temp (
    view_id TEXT NOT NULL,
    course_id TEXT NOT NULL,
    PRIMARY KEY (view_id, course_id),
    FOREIGN KEY (view_id) REFERENCES campus_filter_views_temp(id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES courses_temp(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_venues_campus_id ON venues(campus_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_course_id ON course_venues(course_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_venue_id ON course_venues(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_teacher_id ON teacher_courses(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_course_id ON teacher_courses(course_id);
CREATE INDEX IF NOT EXISTS idx_teacher_campuses_teacher_id ON teacher_campuses(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_campuses_campus_id ON teacher_campuses(campus_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_teacher_id ON scheduled_classes(teacher_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_course_id ON scheduled_classes(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_venue_id ON scheduled_classes(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_unavailability_teacher_id ON teacher_unavailability(teacher_id);
CREATE INDEX IF NOT EXISTS idx_schedule_density_campus_id ON schedule_density(campus_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_views_campus_id ON campus_filter_views(campus_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_venues_venue_id ON campus_filter_view_venues(venue_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_teachers_teacher_id ON campus_filter_view_teachers(teacher_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_courses_course_id ON campus_filter_view_courses(course_id);

CREATE INDEX IF NOT EXISTS idx_venues_temp_campus_id ON venues_temp(campus_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_temp_course_id ON course_venues_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_course_venues_temp_venue_id ON course_venues_temp(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_temp_teacher_id ON teacher_courses_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_courses_temp_course_id ON teacher_courses_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_teacher_campuses_temp_teacher_id ON teacher_campuses_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_teacher_campuses_temp_campus_id ON teacher_campuses_temp(campus_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_teacher_id ON scheduled_classes_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_course_id ON scheduled_classes_temp(course_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_classes_temp_venue_id ON scheduled_classes_temp(venue_id);
CREATE INDEX IF NOT EXISTS idx_teacher_unavailability_temp_teacher_id ON teacher_unavailability_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_schedule_density_temp_campus_id ON schedule_density_temp(campus_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_views_temp_campus_id ON campus_filter_views_temp(campus_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_venues_temp_venue_id ON campus_filter_view_venues_temp(venue_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_teachers_temp_teacher_id ON campus_filter_view_teachers_temp(teacher_id);
CREATE INDEX IF NOT EXISTS idx_campus_filter_view_courses_temp_course_id ON campus_filter_view_courses_temp(course_id);`
