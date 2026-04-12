package backend

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

func (a *App) ImportJSON(filePath string) (ImportStats, error) {
	content, err := os.ReadFile(filePath)
	if err != nil {
		return ImportStats{}, fmt.Errorf("read json file: %w", err)
	}

	var allData AllData
	if err := json.Unmarshal(content, &allData); err != nil {
		return ImportStats{}, fmt.Errorf("parse json file: %w", err)
	}
	if err := fillLegacyTeacherCampuses(content, &allData); err != nil {
		return ImportStats{}, err
	}

	stats := ImportStats{
		Teachers:  len(allData.Teachers),
		Courses:   len(allData.Courses),
		Schedules: len(allData.ScheduledClasses),
	}
	if err := a.SaveTempData(allData); err != nil {
		return ImportStats{}, err
	}
	return stats, nil
}

func (a *App) ImportDatabase(filePath string) (ImportStats, error) {
	sourceDB, err := openDatabase(filePath)
	if err != nil {
		return ImportStats{}, fmt.Errorf("open source database: %w", err)
	}
	defer sourceDB.Close()

	requiredTables := []string{
		"time_slots",
		"days",
		"campuses",
		"venues",
		"courses",
		"course_venues",
		"teachers",
		"teacher_courses",
		"teacher_unavailability",
		"scheduled_classes",
		"schedule_density",
	}
	for _, table := range requiredTables {
		exists, err := tableExists(sourceDB, table)
		if err != nil {
			return ImportStats{}, err
		}
		if !exists {
			return ImportStats{}, fmt.Errorf("source database missing required table: %s", table)
		}
	}

	allData, err := loadAllDataFromConnection(sourceDB, false)
	if err != nil {
		return ImportStats{}, err
	}

	stats := ImportStats{
		Teachers:  len(allData.Teachers),
		Courses:   len(allData.Courses),
		Schedules: len(allData.ScheduledClasses),
	}
	if err := a.SaveTempData(allData); err != nil {
		return ImportStats{}, err
	}
	return stats, nil
}

func (a *App) ExportDatabase(filePath string) error {
	targetDB, err := openDatabase(filePath)
	if err != nil {
		return fmt.Errorf("open target database: %w", err)
	}
	defer targetDB.Close()

	if err := initDatabase(targetDB); err != nil {
		return fmt.Errorf("init target schema: %w", err)
	}

	allData, err := loadAllDataFromConnection(a.db, false)
	if err != nil {
		return err
	}

	tx, err := targetDB.Begin()
	if err != nil {
		return fmt.Errorf("begin target export transaction: %w", err)
	}
	defer tx.Rollback()

	if err := writeAllDataToTables(tx, allData, false); err != nil {
		return err
	}
	return tx.Commit()
}

func (a *App) ExportJSON(filePath string) error {
	allData, err := loadAllDataFromConnection(a.db, false)
	if err != nil {
		return err
	}

	content, err := json.MarshalIndent(allData, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal json: %w", err)
	}
	if err := os.WriteFile(filePath, content, 0o644); err != nil {
		return fmt.Errorf("write json file: %w", err)
	}
	return nil
}

func fillLegacyTeacherCampuses(content []byte, allData *AllData) error {
	if len(allData.TeacherCampuses) > 0 {
		return nil
	}

	allCampusIDs := make([]string, 0, len(allData.Campuses))
	shaheCampusIDs := []string{}
	for _, campus := range allData.Campuses {
		allCampusIDs = append(allCampusIDs, campus.ID)
		if campus.ID == "138697dc-1591-4c16-b60e-d0057964be56" || strings.Contains(campus.Name, "沙河") {
			shaheCampusIDs = append(shaheCampusIDs, campus.ID)
		}
	}
	if len(shaheCampusIDs) == 0 {
		shaheCampusIDs = append(shaheCampusIDs, allCampusIDs...)
	}

	var payload legacyTeacherPayload
	if err := json.Unmarshal(content, &payload); err != nil {
		return fmt.Errorf("parse legacy teacher campuses: %w", err)
	}

	for _, teacher := range payload.Teachers {
		campusIDs := allCampusIDs
		if teacher.IsOnlyShahe {
			campusIDs = shaheCampusIDs
		}
		for _, campusID := range campusIDs {
			allData.TeacherCampuses = append(allData.TeacherCampuses, TeacherCampus{
				TeacherID: teacher.ID,
				CampusID:  campusID,
			})
		}
	}
	return nil
}

var _ queryExecutor = (*sql.DB)(nil)
