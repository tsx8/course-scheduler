use bcrypt::{hash, verify};
use rusqlite::{params, Connection};

const DEFAULT_COST: u32 = 4;

/// Hash a plain text password using bcrypt with cost factor 12
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify a plain text password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

/// Generate a unique username by appending numeric suffix if collision detected
/// Example: "张老师" → "张老师2" → "张老师3"
pub fn generate_unique_username(base_username: &str, conn: &Connection) -> Result<String, String> {
    let mut username = base_username.to_string();
    let mut counter = 2;

    loop {
        // Check if username exists in main users table
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = ?1",
                params![&username],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if exists == 0 {
            return Ok(username);
        }
        username = format!("{}{}", base_username, counter);
        counter += 1;
    }
}

