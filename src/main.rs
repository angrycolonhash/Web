use std::sync::Arc;

use libsql::Connection;
use warp::{http::Method, Filter, Rejection};
use crate::database::Database;
use crate::models::DeviceRequest;

mod database;
mod handler;
mod models;
mod response;

type WebResult<T> = std::result::Result<T, Rejection>;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    // Initialize the database connection and wrap it in an Arc
    let conn = Arc::new(Database::init_db().await?);

    // Define the health checker route
    let health_checker = warp::path!("api" / "healthchecker")
        .and(warp::get())
        .and_then(handler::health_checker_handler);

    // Define the register route
    let register_routes = warp::path!("api" / "register")
        .and(warp::post()) // Handle POST requests
        .and(warp::body::json()) // Parse the request body as JSON
        .and(with_db(conn.clone())) // Pass the database connection as a reference
        .and_then(handler::register_handler);

    let device_lookup_routes = warp::path!("api" / "device")
        .and(warp::post()) // Handle POST requests
        .and(warp::body::json::<DeviceRequest>()) // Parse the request body as JSON
        .and(with_db(conn.clone())) // Pass the database connection as a reference
        .and_then(handler::device_lookup_handler);

    // Configure CORS
    let cors = warp::cors()
        .allow_methods(&[Method::GET, Method::POST])
        .allow_origins(vec!["http://localhost:3000", "http://localhost:8000"])
        .allow_headers(vec!["content-type"])
        .allow_credentials(true);

    // Combine all routes
    let routes = register_routes
        .or(health_checker)
        .or(device_lookup_routes)
        .with(cors)
        .with(warp::log("api"));

    // Start the Warp server
    println!("🚀 Server started successfully");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

fn with_db(
    conn: Arc<Connection>,
) -> impl Filter<Extract = (Arc<Connection>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || conn.clone()) // Pass a cloned Arc of the connection
}