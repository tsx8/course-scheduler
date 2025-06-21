use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AllData {
    pub time: Vec<TimeSlot>,
    pub day: Vec<Day>,
    pub campuses: Vec<Campus>,
    pub courses: Vec<Course>,
    pub teachers: Vec<Teacher>,
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
    pub venues: Vec<Venue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Venue {
    pub id: String,
    pub name: String,
    pub capacity: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Course {
    pub id: String,
    pub name: String,
    pub place: Vec<CoursePlace>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoursePlace {
    pub campus_id: String,
    pub venue_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Teacher {
    pub id: String,
    pub name: String,
    pub max_teaching_hours: u8,
    pub is_only_shahe: bool,
    #[serde(default)]
    pub unavailable: Vec<BlockedSlot>,
    pub teaches: Vec<String>,
    pub scheduled: Vec<ScheduledClass>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduledClass {
    pub id: String,
    pub day_id: String,
    pub time_id: String,
    pub course_id: String,
    pub campus_id: String,
    pub venue_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockedSlot {
    pub day_id: String,
    pub time_id: String,
}
