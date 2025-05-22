//! Database module
//! 
//! This module is used for connecting to the database. For the most part, these comments are just 
//! notes to myself for when I forgot how to use them. The documentation is plenty so an idiot like
//! me knows how to use them. 
//! 
//! To initialise, use ```Database::init_db().await?```
//! 
//! Transactions: 
//! ```rust
//! let tx = Database::start_transaction(&conn).await?;
//! {
//!     // Put what you need here
//! }
//! Database::commit_transaction(tx).await?;
//! ```
//! 

use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use chrono::{DateTime, Utc};
use libsql::{params, Builder, Connection, Transaction};

pub struct Database;

impl Database {
    pub async fn init_db() -> anyhow::Result<Connection> {
        // TODO: make this more secure like come on man what the shit?!?
        let db = Builder::new_local("winklink.db").build().await?;
        let conn = db.connect()?;

        conn.execute("CREATE TABLE IF NOT EXISTS users (
                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                            uuid TEXT UNIQUE NOT NULL,
                            serial_number TEXT UNIQUE NOT NULL,
                            device_name TEXT,
                            device_owner TEXT,  -- This is your username field
                            email TEXT UNIQUE NOT NULL,
                            password_hash TEXT,  -- Renamed from password to password_hash for clarity
                            created_at TEXT NOT NULL
                        )", ()).await?;
        
        log::debug!("Initialised sqlite3 database");
        Ok(conn)
    }

    pub async fn start_transaction(conn: &Connection) -> anyhow::Result<Transaction> {
        let tx = conn.transaction().await?;

        Ok(tx)
    }

    pub async fn commit_transaction(tx: Transaction) -> anyhow::Result<()> {
        log::debug!("Finished commiting transaction");
        tx.commit().await?;
        
        Ok(())
        // tx finna get dropped here, it aint gunna be here no more
    }

    pub async fn keyword_exists(
        conn: &Connection,
        keyword: WLdbKeyword,
    ) -> anyhow::Result<bool> {
        let query = match keyword {
            WLdbKeyword::SerialNumber(value) => {
                ("SELECT COUNT(*) FROM users WHERE serial_number = ?", value)
            }
            WLdbKeyword::Email(value) => {
                ("SELECT COUNT(*) FROM users WHERE email = ?", value)
            }
            WLdbKeyword::DeviceName(value) => {
                ("SELECT COUNT(*) FROM users WHERE device_name = ?", value)
            }
            WLdbKeyword::DeviceOwner(value) => {
                ("SELECT COUNT(*) FROM users WHERE device_owner = ?", value)
            }
            WLdbKeyword::UUID(value) => {
                ("SELECT COUNT(*) FROM users WHERE uuid = ?", value)
            }
        };

        let mut stmt = conn.prepare(query.0).await?;
        let mut rows = stmt.query(params![query.1]).await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            return Ok(count > 0);
        }

        Ok(false)
    }
}

pub struct Register;

impl Register {
    /// step 1, inserts the serial number and email
    /// 
    /// Sample usage:
    /// ```
    /// if let Err(e) = Register::insert_serial_and_email(&tx, serial_number, email).await {
    ///     tx.rollback().await?;
    ///     return Err(e);
    /// }
    /// ```
    /// Return value: ```anyhow::Result<String>```
    pub async fn insert_serial_and_email(
        tx: &libsql::Transaction,
        serial_number: &str,
        email: &str,
    ) -> anyhow::Result<String> {
        if serial_number.len() > 12 {
            return Err(anyhow::anyhow!("Serial number must be at most 12 characters long"));
        }

        let uuid = uuid::Uuid::new_v4();
        let uuid_string = uuid.clone().to_string();

        let created_at: DateTime<Utc> = Utc::now();
        let created_at_str = created_at.to_rfc3339();

        tx.execute("INSERT INTO users (uuid, serial_number, email, created_at) VALUES (?, ?, ?, ?)",
            params![uuid_string, serial_number, email, created_at_str]).await?;

        Ok(uuid.to_string())
    }

    /// step 2, inserts username and password
    /// 
    /// Sample usage: 
    /// ```rust
    /// if let Err(e) = Register::update_username_and_password(&tx, &uuid, username, password).await {
    ///     tx.rollback().await?;
    ///     return Err(e);
    /// }
    /// ```
    pub async fn update_username_and_password(
        tx: &libsql::Transaction,
        uuid: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<()> {
        // Hash the password before storing it
        let password_hash = Self::hash_password(password)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
            
        // Update the database with the hashed password
        tx.execute("UPDATE users SET device_owner = ?, password_hash = ? WHERE uuid = ?",
            params![username, password_hash, uuid]).await?;

        Ok(())
    }
    

    /// step 3, inserts device name
    ///
    /// Sample usage: 
    /// ```rust
    /// if let Err(e) = Register::update_device_name(&tx, &uuid, device_name).await {
    ///     tx.rollback().await?;
    ///     return Err(e);
    /// }
    /// ```
    pub async fn update_device_name(
        tx: &libsql::Transaction,
        uuid: &str,
        device_name: &str,
    ) -> anyhow::Result<()> {
        tx.execute("UPDATE users SET device_name = ? WHERE uuid = ?",
            params![device_name, uuid]).await?;

        Ok(())
    }

    fn hash_password(password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?
            .to_string();
        
        Ok(password_hash)
    }
}

pub enum WLdbKeyword {
    SerialNumber(String),
    Email(String),
    DeviceName(String),
    DeviceOwner(String),
    UUID(String),
}