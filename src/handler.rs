use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use argon2::{password_hash::{rand_core::OsRng, Salt, SaltString}, Argon2, Params, PasswordHash, PasswordVerifier};
use chrono::{DateTime, NaiveDateTime, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use libsql::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use argon2::PasswordHasher;
use warp::{http::StatusCode, reply::{json, with_status, Reply}};

use crate::{database::{Database, Register, WLdbKeyword}, models::{DeviceRequest, LoginRequest, WLRegister}, response::{GenericResponse, LoginResponse, WLDeviceResponse}, WebResult};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,   // Subject (user ID)
    exp: usize,    // Expiration time
    iat: usize,    // Issued at
}
#[derive(Debug)]
struct ServerError;

impl warp::reject::Reject for ServerError {}

pub async fn health_checker_handler() -> WebResult<impl Reply> {
    const MESSAGE: &str = "WinkLink Simple API";

    let response_json = &GenericResponse {
        status: "success".to_string(),
        message: MESSAGE.to_string(),
    };
    Ok(json(response_json))
}

pub async fn register_handler(body: WLRegister, conn: Arc<libsql::Connection>) -> WebResult<impl Reply> {
    // Check if the serial number already exists
    if let Ok(val) = Database::keyword_exists(&conn, WLdbKeyword::SerialNumber(body.serial_number.clone())).await {
        if val {
            let error_response = GenericResponse {
                status: "fail".to_string(),
                message: "Serial number already exists".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::CONFLICT));
        }
    }

    // Check if the email already exists
    if let Ok(val) = Database::keyword_exists(&conn, WLdbKeyword::Email(body.email.clone())).await {
        if val {
            let error_response = GenericResponse {
                status: "fail".to_string(),
                message: "Email already exists".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::CONFLICT));
        }
    }

    // Check if the username already exists
    if let Ok(val) = Database::keyword_exists(&conn, WLdbKeyword::DeviceOwner(body.username.clone())).await {
        if val {
            let error_response = GenericResponse {
                status: "fail".to_string(),
                message: "Username already exists".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::CONFLICT));
        }
    }

    // Start a transaction
    let tx = match Database::start_transaction(&conn).await {
        Ok(tx) => tx,
        Err(e) => {
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: format!("Failed to start transaction: {}", e),
            };
            return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    // Insert user data into the database (Step 1)
    let uuid = match Register::insert_serial_and_email(&tx, &body.serial_number, &body.email).await {
        Ok(uuid) => uuid,
        Err(e) => {
            let _ = tx.rollback().await; // Rollback the transaction on failure
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: format!("Failed to insert user data: {}", e),
            };
            return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    // Update username and password (Step 2)
    if let Err(e) = Register::update_username_and_password(&tx, &uuid, &body.username, &body.password).await {
        let _ = tx.rollback().await; // Rollback the transaction on failure
        let error_response = GenericResponse {
            status: "error".to_string(),
            message: format!("Failed to update username and password: {}", e),
        };
        return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
    }

    // Update device name (Step 3)
    if let Err(e) = Register::update_device_name(&tx, &uuid, &body.device_name).await {
        let _ = tx.rollback().await; // Rollback the transaction on failure
        let error_response = GenericResponse {
            status: "error".to_string(),
            message: format!("Failed to update device name: {}", e),
        };
        return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
    }

    // Commit the transaction
    if let Err(e) = Database::commit_transaction(tx).await {
        let error_response = GenericResponse {
            status: "error".to_string(),
            message: format!("Failed to commit transaction: {}", e),
        };
        return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
    }

    // Success response
    let json_response = GenericResponse {
        status: "success".to_string(),
        message: format!("User {} has been created at {} [utc]", body.username, Utc::now()),
    };

    Ok(with_status(json(&json_response), StatusCode::CREATED))
}

pub async fn device_lookup_handler(body: DeviceRequest, conn: Arc<libsql::Connection>) -> WebResult<impl Reply> {
    if let Ok(val) = Database::keyword_exists(&conn, WLdbKeyword::SerialNumber(body.serial_number.clone())).await {
        if !val {
            let error_response = GenericResponse {
                status: "fail".to_string(),
                message: "Device with this serial number not found".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::NOT_FOUND));
        }
    } else {
        // Error checking if serial number exists
        let error_response = GenericResponse {
            status: "error".to_string(),
            message: "Failed to query database".to_string(),
        };
        return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
    }

    // Query for device details
    let mut stmt = match conn.prepare("SELECT device_owner, device_name FROM users WHERE serial_number = ?").await {
        Ok(stmt) => stmt,
        Err(_) => {
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: "Failed to prepare database query".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
        }
    };
    let mut rows = match stmt.query(params![body.serial_number]).await {
        Ok(rows) => rows,
        Err(_) => {
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: "Database query failed".to_string(),
            };
            return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    // Try to get the first row
    match rows.next().await {
        Ok(Some(row)) => {
            // Extract the values from the row
            let device_owner: String = match row.get(0) {
                Ok(value) => value,
                Err(_) => String::new(), // Default value if null
            };
            
            let device_name: String = match row.get(1) {
                Ok(value) => value,
                Err(_) => String::new(), // Default value if null
            };
            
            let response = WLDeviceResponse {
                device_owner,
                device_name,
            };
            
            Ok(with_status(json(&response), StatusCode::OK))
        },
        Ok(None) => {
            // This shouldn't happen since we checked existence above, but just in case
            let error_response = GenericResponse {
                status: "fail".to_string(),
                message: "Device not found".to_string(),
            };
            Ok(with_status(json(&error_response), StatusCode::NOT_FOUND))
        },
        Err(_) => {
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: "Failed to retrieve device data".to_string(),
            };
            Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

pub async fn login_handler(body: LoginRequest, conn: Arc<libsql::Connection>) -> WebResult<impl Reply> {
    // Add basic validation for request body
    if body.email.is_empty() || body.password.is_empty() {
        let error_response = GenericResponse {
            status: "fail".to_string(),
            message: "Email and password are required".to_string(),
        };
        return Ok(with_status(json(&error_response), StatusCode::BAD_REQUEST));
    }
    
    // Wrap the entire handler in a try-catch to prevent server crashes
    match login_user(&body, &conn).await {
        Ok(response) => {
            return Ok(with_status(json(&response), StatusCode::OK));
        },
        Err(e) => {
            let error_response = GenericResponse {
                status: "error".to_string(),
                message: format!("Login failed: {}", e),
            };
            return Ok(with_status(json(&error_response), StatusCode::INTERNAL_SERVER_ERROR));
        }
    }
}

async fn login_user(body: &LoginRequest, conn: &Arc<libsql::Connection>) -> Result<LoginResponse, Box<dyn std::error::Error>> {
    // Query user by email
    let query = "SELECT uuid, email, device_owner, password_hash FROM users WHERE email = ?";
    let mut stmt = conn.prepare(query).await?;
    let row = stmt.query_row([body.email.clone()]).await?;

    let user_id: String = row.get(0)?;
    let stored_hash: String = row.get(3)?;
    let username: String = row.get(2)?;

    // Verify password using Argon2
    let parsed_hash = PasswordHash::new(&stored_hash).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
    if Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).is_ok() {
        // Generate JWT token
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let claims = Claims {
            sub: user_id.clone(),
            exp: (now + 7 * 24 * 60 * 60) as usize,
            iat: now as usize,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("your_secret_key".as_ref())
        )?;

        Ok(LoginResponse {
            status: "success".to_string(),
            message: format!("User {} logged in successfully", username),
            token,
            user_id,
        })
    } else {
        Err("Invalid email or password".into())
    }
}