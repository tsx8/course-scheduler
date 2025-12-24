use bcrypt::{hash, verify};
use rusqlite::{params, Connection};

const DEFAULT_COST: u32 = 4;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn generate_unique_username(base_username: &str, conn: &Connection) -> Result<String, String> {
    let mut username = base_username.to_string();
    let mut counter = 2;

    loop {
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

