use std::sync::Arc;

use chrono::{DateTime, Utc};
use libsql::{Connection, Transaction};
use warp::{http::StatusCode, reply::{json, with_status, Reply}};

use crate::{database::{Database, Register, WLdbKeyword}, models::WLRegister, response::GenericResponse, WebResult};

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