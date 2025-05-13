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

use libsql::{params, Builder, Connection, Transaction};

pub struct Database;

impl Database {
    pub async fn init_db() -> anyhow::Result<Connection> {
        // TODO: make this more secure like come on man what the shit?!?
        let db = Builder::new_local("winklink.db").build().await?;  // this is just for testing probably going to switch to some other connection type
        let conn = db.connect()?;

        conn.execute("CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE NOT NULL,
                serial_number TEXT UNIQUE NOT NULL,
                device_name TEXT,
                device_owner TEXT,
                email TEXT UNIQUE NOT NULL,
                password TEXT
            )", ()).await?;
        
        println!("Initialised sqlite3 database");
        Ok(conn)
    }

    pub async fn start_transaction(conn: &Connection) -> anyhow::Result<Transaction> {
        let tx = conn.transaction().await?;

        Ok(tx)
    }

    pub async fn commit_transaction(tx: Transaction) -> anyhow::Result<()> {
        println!("Finished commiting transaction");
        tx.commit().await?;
        
        Ok(())
        // tx finna get dropped here, it aint gunna be here no more
    }
}

pub struct Register;

impl Register {
    /// step 1, inserts the serial number and email
    /// 
    /// Sample usage:
    /// ```
    /// if let Err(e) = Register::insert_serial_and_email(&tx, &uuid, serial_number, email).await {
    ///     tx.rollback().await?;
    ///     return Err(e);
    /// }
    /// ```
    pub async fn insert_serial_and_email(
        tx: &libsql::Transaction,
        uuid: &str,
        serial_number: &str,
        email: &str,
    ) -> anyhow::Result<()> {
        if serial_number.len() > 12 {
            return Err(anyhow::anyhow!("Serial number must be at most 12 characters long"));
        }

        tx.execute("INSERT INTO users (uuid, serial_number, email) VALUES (?, ?, ?)",
            params![uuid, serial_number, email]).await?;

        Ok(())
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
        tx.execute("UPDATE users SET device_owner = ?, password = ? WHERE uuid = ?",
            params![username, password, uuid]).await?;

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
}