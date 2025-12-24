use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

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
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub users: Vec<User>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    pub role_id: String,
    pub teacher_id: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditLogEntry {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub action_type: String,
    pub target_table: Option<String>,
    pub target_id: Option<String>,
    pub timestamp: String,
    pub change_details: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    pub current_page: usize,
    pub per_page: usize,
    pub total_entries: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogsResponse {
    pub entries: Vec<AuditLogEntry>,
    pub pagination: PaginationInfo,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogFilters {
    pub username: Option<String>,
    pub action_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub target_table: Option<String>,
}

pub type SessionStore = Mutex<HashMap<String, User>>;
