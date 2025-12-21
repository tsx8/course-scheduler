use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AllData {
    #[serde(default)]
    pub time: Vec<TimeSlot>,
    #[serde(default)]
    pub day: Vec<Day>,
    #[serde(default)]
    pub campuses: Vec<Campus>,
    #[serde(default)]
    pub venues: Vec<Venue>,
    #[serde(default)]
    pub courses: Vec<Course>,
    #[serde(default)]
    pub teachers: Vec<Teacher>,
    #[serde(default)]
    pub course_venues: Vec<CourseVenue>,
    #[serde(default)]
    pub teacher_courses: Vec<TeacherCourse>,
    #[serde(default)]
    pub scheduled_classes: Vec<ScheduledClass>,
    #[serde(default)]
    pub teacher_unavailability: Vec<TeacherUnavailability>,
    #[serde(default)]
    pub schedule_density: Vec<ScheduleDensity>,
}

/// Migration statistics returned after JSON-to-SQLite migration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationStats {
    pub teachers: usize,
    pub courses: usize,
    pub schedules: usize,
    pub source_path: String,
    pub source_location: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeSlot {
    pub id: String,
    pub value: String,
    #[serde(rename = "corresponding_hours")]
    pub corresponding_hours: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Day {
    pub id: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Campus {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleDensity {
    pub campus_id: String,
    pub day_id: String,
    pub time_id: String,
    pub count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Venue {
    pub id: String,
    pub campus_id: String,
    pub name: String,
    pub capacity: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Course {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CourseVenue {
    pub course_id: String,
    pub venue_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Teacher {
    pub id: String,
    pub name: String,
    pub max_teaching_hours: u8,
    pub is_only_shahe: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeacherCourse {
    pub teacher_id: String,
    pub course_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduledClass {
    pub id: String,
    pub teacher_id: String,
    pub course_id: String,
    pub day_id: String,
    pub time_id: String,
    pub campus_id: String,
    pub venue_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeacherUnavailability {
    pub teacher_id: String,
    pub day_id: String,
    pub time_id: String,
}
