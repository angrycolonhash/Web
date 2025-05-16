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