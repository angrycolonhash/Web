use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use argon2::{password_hash::Salt, Argon2, Params, PasswordHash, PasswordVerifier};
use chrono::{DateTime, NaiveDateTime, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use libsql::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
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

pub async fn login_handler(
    login_req: LoginRequest,
    db: Arc<Connection>
) -> WebResult<impl Reply> {
    // Query the database for the user with the provided email
    let mut rows = match db.query(
        "SELECT id, password FROM users WHERE email = ?",
        libsql::params![login_req.email]
    ).await {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Database query error: {}", e);
            return Ok(with_status(
                json(&GenericResponse {
                    status: "error".to_string(),
                    message: "Error querying the database".to_string(),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };
    
    let row = match rows.next().await {
        Ok(Some(row)) => row,
        Ok(None) => {
            // User does not exist
            return Ok(with_status(
                json(&GenericResponse {
                    status: "fail".to_string(),
                    message: "Invalid email or password".to_string(),
                }),
                StatusCode::UNAUTHORIZED,
            ));
        }
        Err(e) => {
            eprintln!("Failed to retrieve row: {}", e);
            return Ok(with_status(
                json(&GenericResponse {
                    status: "error".to_string(),
                    message: "Error retrieving user data".to_string(),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };
    
    let user_id: String = match row.get(0) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to get user_id: {}", e);
            return Ok(with_status(
                json(&GenericResponse {
                    status: "error".to_string(),
                    message: "Error processing user data".to_string(),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };
    let password_hash: String = match row.get(1) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Failed to get password_hash: {}", e);
            return Ok(with_status(
                json(&GenericResponse {
                    status: "error".to_string(),
                    message: "Error processing user data".to_string(),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let password_matches = match PasswordHash::new(&password_hash) {
        Ok(parsed_hash) => {
            Argon2::default()
                .verify_password(login_req.password.as_bytes(), &parsed_hash)
                .is_ok()
        }
        Err(e) => {
            eprintln!("Failed to parse password hash: {}", e);
            // Treat hash parsing errors as a security precaution, similar to a password mismatch
            false 
        }
    };

    if !password_matches {
        return Ok(with_status(
            json(&GenericResponse {
                status: "fail".to_string(),
                message: "Invalid email or password".to_string(),
            }),
            StatusCode::UNAUTHORIZED,
        ));
    }
    
    // Generate JWT token
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize;
    
    let claims = Claims {
        sub: user_id.clone(),
        exp: now + 60 * 60 * 24, // 24 hours expiration
        iat: now,
    };

    let secret = "my_super_secret_key";
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| warp::reject::custom(ServerError))?;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&LoginResponse {
            status: "success".to_string(),
            message: "Login successful".to_string(),
            token,
            user_id,
        }),
        StatusCode::OK,
    ))
}