use serde::Serialize;

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct WLDeviceResponse {
    pub device_owner: String,
    pub device_name: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub message: String,
    pub token: String,
    pub user_id: String,
}