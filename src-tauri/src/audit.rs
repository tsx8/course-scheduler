// ============================================================================
// Audit Logging Module
// Feature: 001-rbac-audit-system
// ============================================================================

use chrono::Local;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tracing::error;
use uuid::Uuid;

pub fn compute_diff(old_val: &Value, new_val: &Value) -> Value {
    let mut diffs = Vec::new();

    if let (Value::Object(old_obj), Value::Object(new_obj)) = (old_val, new_val) {
        for (key, new_v) in new_obj {
            let old_v = old_obj.get(key).unwrap_or(&Value::Null);
            if old_v != new_v && key != "password_hash" {
                diffs.push(json!({
                    "field": key,
                    "old": old_v,
                    "new": new_v
                }));
            }
        }
    }

    Value::Array(diffs)
}

/// Create an audit log entry in the main audit_logs table
/// Note: Audit write failures are logged but do NOT block primary operations
pub fn create_audit_log_entry(
    conn: &Connection,
    user_id: &str,
    action_type: &str,
    target_table: Option<&str>,
    target_id: Option<&str>,
    change_details: Option<Value>,
    ip_address: Option<&str>,
    is_temp: bool,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let timestamp = Local::now().to_rfc3339();
    let details_json = change_details.map(|v| v.to_string());

    let table_name = if is_temp {
        "audit_logs_temp"
    } else {
        "audit_logs"
    };

    let sql = format!(
        "INSERT INTO {} (id, user_id, action_type, target_table, target_id, timestamp, change_details, ip_address)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        table_name
    );

    let result = conn.execute(
        &sql,
        params![
            id,
            user_id,
            action_type,
            target_table,
            target_id,
            timestamp,
            details_json,
            ip_address
        ],
    );

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // Log error but don't propagate - audit failures should not block business logic
            error!(
                "Failed to create audit log entry: {} - action: {}, user: {}",
                e, action_type, user_id
            );
            Err(format!("Audit logging failed: {}", e))
        }
    }
}
