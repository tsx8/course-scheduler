package backend

import "encoding/json"

type AllData struct {
	Time                  []TimeSlot              `json:"time"`
	Day                   []Day                   `json:"day"`
	Campuses              []Campus                `json:"campuses"`
	Venues                []Venue                 `json:"venues"`
	Courses               []Course                `json:"courses"`
	Teachers              []Teacher               `json:"teachers"`
	CourseVenues          []CourseVenue           `json:"course_venues"`
	TeacherCourses        []TeacherCourse         `json:"teacher_courses"`
	TeacherCampuses       []TeacherCampus         `json:"teacher_campuses"`
	ScheduledClasses      []ScheduledClass        `json:"scheduled_classes"`
	TeacherUnavailability []TeacherUnavailability `json:"teacher_unavailability"`
	ScheduleDensity       []ScheduleDensity       `json:"schedule_density"`
}

type TimeSlot struct {
	ID                 string `json:"id"`
	Value              string `json:"value"`
	CorrespondingHours int    `json:"corresponding_hours"`
}

type Day struct {
	ID    string `json:"id"`
	Value string `json:"value"`
}

type Campus struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

type ScheduleDensity struct {
	CampusID string `json:"campus_id"`
	DayID    string `json:"day_id"`
	TimeID   string `json:"time_id"`
	Count    int    `json:"count"`
}

type Venue struct {
	ID       string `json:"id"`
	CampusID string `json:"campus_id"`
	Name     string `json:"name"`
	Capacity int    `json:"capacity"`
}

type Course struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

type CourseVenue struct {
	CourseID string `json:"course_id"`
	VenueID  string `json:"venue_id"`
}

type Teacher struct {
	ID               string `json:"id"`
	Name             string `json:"name"`
	MaxTeachingHours int    `json:"max_teaching_hours"`
}

type TeacherCourse struct {
	TeacherID string `json:"teacher_id"`
	CourseID  string `json:"course_id"`
}

type TeacherCampus struct {
	TeacherID string `json:"teacher_id"`
	CampusID  string `json:"campus_id"`
}

type ScheduledClass struct {
	ID          string `json:"id"`
	TeacherID   string `json:"teacher_id"`
	CourseID    string `json:"course_id"`
	DayID       string `json:"day_id"`
	TimeID      string `json:"time_id"`
	CampusID    string `json:"campus_id"`
	VenueID     string `json:"venue_id"`
	IsLocked    bool   `json:"is_locked"`
	IsStaged    bool   `json:"is_staged"`
	StagedOrder int    `json:"staged_order"`
}

func (s *ScheduledClass) UnmarshalJSON(data []byte) error {
	type scheduledClassAlias ScheduledClass
	class := scheduledClassAlias{IsLocked: true}
	if err := json.Unmarshal(data, &class); err != nil {
		return err
	}
	*s = ScheduledClass(class)
	return nil
}

type TeacherUnavailability struct {
	TeacherID string `json:"teacher_id"`
	DayID     string `json:"day_id"`
	TimeID    string `json:"time_id"`
}

type ImportStats struct {
	Teachers  int `json:"teachers"`
	Courses   int `json:"courses"`
	Schedules int `json:"schedules"`
}

type DialogFileFilter struct {
	DisplayName string `json:"displayName"`
	Pattern     string `json:"pattern"`
}

type OpenDialogOptions struct {
	Title       string             `json:"title"`
	DefaultPath string             `json:"defaultPath"`
	Filters     []DialogFileFilter `json:"filters"`
	Multiple    bool               `json:"multiple"`
}

type SaveDialogOptions struct {
	Title       string             `json:"title"`
	DefaultPath string             `json:"defaultPath"`
	Filters     []DialogFileFilter `json:"filters"`
}

type legacyTeacher struct {
	ID          string `json:"id"`
	IsOnlyShahe bool   `json:"is_only_shahe"`
}

type legacyTeacherPayload struct {
	Teachers []legacyTeacher `json:"teachers"`
}
