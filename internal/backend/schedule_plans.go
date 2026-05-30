package backend

import (
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
)

var errSchedulePlanBlockedByUnsavedChanges = errors.New("存在未保存更改，请先保存或撤销后再管理课表版本")

func (a *App) CreateSchedulePlan(name string) (AllData, error) {
	return a.createSchedulePlan(name, false)
}

func (a *App) CopySchedulePlan(name string) (AllData, error) {
	return a.createSchedulePlan(name, true)
}

func (a *App) SwitchSchedulePlan(planID string) (AllData, error) {
	trimmedPlanID := strings.TrimSpace(planID)
	if trimmedPlanID == "" {
		return AllData{}, errors.New("课表版本 ID 不能为空")
	}

	tx, err := a.db.Begin()
	if err != nil {
		return AllData{}, fmt.Errorf("begin switch schedule plan transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := ensureNoUnsavedSchedulePlanChanges(tx); err != nil {
		return AllData{}, err
	}
	if err := ensureSchedulePlanExists(tx, false, trimmedPlanID); err != nil {
		return AllData{}, err
	}
	if err := saveActiveSchedulePlanID(tx, false, trimmedPlanID); err != nil {
		return AllData{}, err
	}
	if err := tx.Commit(); err != nil {
		return AllData{}, fmt.Errorf("switch schedule plan: %w", err)
	}

	return a.LoadData()
}

func (a *App) RenameSchedulePlan(planID string, name string) (AllData, error) {
	trimmedPlanID := strings.TrimSpace(planID)
	if trimmedPlanID == "" {
		return AllData{}, errors.New("课表版本 ID 不能为空")
	}
	trimmedName, err := normalizeSchedulePlanName(name)
	if err != nil {
		return AllData{}, err
	}

	tx, err := a.db.Begin()
	if err != nil {
		return AllData{}, fmt.Errorf("begin rename schedule plan transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := ensureNoUnsavedSchedulePlanChanges(tx); err != nil {
		return AllData{}, err
	}
	if err := ensureSchedulePlanExists(tx, false, trimmedPlanID); err != nil {
		return AllData{}, err
	}
	if _, err := tx.Exec("UPDATE schedule_plans SET name = ? WHERE id = ?", trimmedName, trimmedPlanID); err != nil {
		return AllData{}, fmt.Errorf("rename schedule plan: %w", err)
	}
	if err := tx.Commit(); err != nil {
		return AllData{}, fmt.Errorf("commit schedule plan rename: %w", err)
	}

	return a.LoadData()
}

func (a *App) DeleteSchedulePlan(planID string) (AllData, error) {
	trimmedPlanID := strings.TrimSpace(planID)
	if trimmedPlanID == "" {
		return AllData{}, errors.New("课表版本 ID 不能为空")
	}

	tx, err := a.db.Begin()
	if err != nil {
		return AllData{}, fmt.Errorf("begin delete schedule plan transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := ensureNoUnsavedSchedulePlanChanges(tx); err != nil {
		return AllData{}, err
	}
	plans, err := loadSchedulePlans(tx, false)
	if err != nil {
		return AllData{}, err
	}
	if len(plans) <= 1 {
		return AllData{}, errors.New("不能删除最后一个课表版本")
	}

	remainingPlans := make([]SchedulePlan, 0, len(plans)-1)
	found := false
	for _, plan := range plans {
		if plan.ID == trimmedPlanID {
			found = true
			continue
		}
		remainingPlans = append(remainingPlans, plan)
	}
	if !found {
		return AllData{}, fmt.Errorf("课表版本不存在: %s", trimmedPlanID)
	}

	activePlanID, err := loadActiveSchedulePlanID(tx, false, plans)
	if err != nil {
		return AllData{}, err
	}

	if _, err := tx.Exec("DELETE FROM scheduled_classes WHERE schedule_plan_id = ?", trimmedPlanID); err != nil {
		return AllData{}, fmt.Errorf("delete schedule plan classes: %w", err)
	}
	if _, err := tx.Exec("DELETE FROM schedule_plans WHERE id = ?", trimmedPlanID); err != nil {
		return AllData{}, fmt.Errorf("delete schedule plan: %w", err)
	}
	nextActivePlanID := activePlanID
	if activePlanID == trimmedPlanID {
		nextActivePlanID = remainingPlans[0].ID
	}
	if err := saveActiveSchedulePlanID(tx, false, nextActivePlanID); err != nil {
		return AllData{}, err
	}
	if err := tx.Commit(); err != nil {
		return AllData{}, fmt.Errorf("commit schedule plan delete: %w", err)
	}

	return a.LoadData()
}

func (a *App) createSchedulePlan(name string, copyCurrent bool) (AllData, error) {
	trimmedName, err := normalizeSchedulePlanName(name)
	if err != nil {
		return AllData{}, err
	}

	tx, err := a.db.Begin()
	if err != nil {
		return AllData{}, fmt.Errorf("begin create schedule plan transaction: %w", err)
	}
	defer func() { _ = tx.Rollback() }()

	if err := ensureNoUnsavedSchedulePlanChanges(tx); err != nil {
		return AllData{}, err
	}

	newPlanID, err := newUUIDString()
	if err != nil {
		return AllData{}, err
	}
	sortOrder, err := nextSchedulePlanSortOrder(tx, false)
	if err != nil {
		return AllData{}, err
	}
	if _, err := tx.Exec(
		"INSERT INTO schedule_plans (id, name, sort_order) VALUES (?, ?, ?)",
		newPlanID,
		trimmedName,
		sortOrder,
	); err != nil {
		return AllData{}, fmt.Errorf("insert schedule plan: %w", err)
	}

	if copyCurrent {
		if err := copyActiveSchedulePlanClasses(tx, newPlanID); err != nil {
			return AllData{}, err
		}
	}
	if err := saveActiveSchedulePlanID(tx, false, newPlanID); err != nil {
		return AllData{}, err
	}
	if err := tx.Commit(); err != nil {
		return AllData{}, fmt.Errorf("commit schedule plan create: %w", err)
	}

	return a.LoadData()
}

func ensureNoUnsavedSchedulePlanChanges(conn queryExecutor) error {
	hasChanges, err := hasUnsavedChanges(conn)
	if err != nil {
		return err
	}
	if hasChanges {
		return errSchedulePlanBlockedByUnsavedChanges
	}
	return nil
}

func ensureSchedulePlanExists(conn queryExecutor, useTemp bool, planID string) error {
	table := tableName("schedule_plans", useTemp)
	var count int
	if err := conn.QueryRow(fmt.Sprintf("SELECT COUNT(*) FROM %s WHERE id = ?", table), planID).Scan(&count); err != nil {
		return fmt.Errorf("find schedule plan: %w", err)
	}
	if count == 0 {
		return fmt.Errorf("课表版本不存在: %s", planID)
	}
	return nil
}

func normalizeSchedulePlanName(name string) (string, error) {
	trimmedName := strings.TrimSpace(name)
	if trimmedName == "" {
		return "", errors.New("课表版本名称不能为空")
	}
	return trimmedName, nil
}

func nextSchedulePlanSortOrder(conn queryExecutor, useTemp bool) (int, error) {
	table := tableName("schedule_plans", useTemp)
	var maxSortOrder sql.NullInt64
	if err := conn.QueryRow(fmt.Sprintf("SELECT MAX(sort_order) FROM %s", table)).Scan(&maxSortOrder); err != nil {
		return 0, fmt.Errorf("load next schedule plan order: %w", err)
	}
	if !maxSortOrder.Valid {
		return 0, nil
	}
	return int(maxSortOrder.Int64) + 1, nil
}

func saveActiveSchedulePlanID(conn queryExecutor, useTemp bool, planID string) error {
	table := tableName("app_metadata", useTemp)
	if _, err := conn.Exec(
		fmt.Sprintf("INSERT OR REPLACE INTO %s (key, value) VALUES (?, ?)", table),
		activeSchedulePlanStateKey,
		planID,
	); err != nil {
		return fmt.Errorf("save active schedule plan: %w", err)
	}
	return nil
}

func copyActiveSchedulePlanClasses(tx *sql.Tx, targetPlanID string) error {
	plans, err := loadSchedulePlans(tx, false)
	if err != nil {
		return err
	}
	activePlanID, err := loadActiveSchedulePlanID(tx, false, plans)
	if err != nil {
		return err
	}
	classes, err := loadScheduledClasses(tx, false, activePlanID, scheduledClassLoadActive)
	if err != nil {
		return err
	}

	for _, class := range classes {
		newClassID, err := newUUIDString()
		if err != nil {
			return err
		}
		if _, err := tx.Exec(
			`INSERT INTO scheduled_classes
				(id, schedule_plan_id, teacher_id, course_id, day_id, time_id, campus_id, venue_id, is_locked, is_staged, staged_order)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			newClassID,
			targetPlanID,
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
			return fmt.Errorf("copy schedule plan class: %w", err)
		}
	}
	return nil
}

func newUUIDString() (string, error) {
	var bytes [16]byte
	if _, err := rand.Read(bytes[:]); err != nil {
		return "", fmt.Errorf("generate uuid: %w", err)
	}

	bytes[6] = (bytes[6] & 0x0f) | 0x40
	bytes[8] = (bytes[8] & 0x3f) | 0x80

	encoded := make([]byte, 36)
	hex.Encode(encoded[0:8], bytes[0:4])
	encoded[8] = '-'
	hex.Encode(encoded[9:13], bytes[4:6])
	encoded[13] = '-'
	hex.Encode(encoded[14:18], bytes[6:8])
	encoded[18] = '-'
	hex.Encode(encoded[19:23], bytes[8:10])
	encoded[23] = '-'
	hex.Encode(encoded[24:36], bytes[10:16])

	return string(encoded), nil
}
