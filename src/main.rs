mod db;
mod error;
mod handlers;
mod models;
mod pdf_parser;

use std::sync::Arc;

use axum::{routing::delete, routing::get, routing::post, Router};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db = Arc::new(db::Db::new("health_report.redb")?);
    tracing::info!("数据库初始化完成");

    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/reports/upload", post(handlers::upload_pdf))
        .route("/api/reports", get(handlers::list_reports))
        .route("/api/reports/{id}", get(handlers::get_report))
        .route("/api/reports/{id}", delete(handlers::delete_report))
        .route("/api/reports/{id}/indicators", get(handlers::get_indicators))
        .route("/api/indicators/abnormal", get(handlers::get_abnormal_indicators))
        .route("/api/indicators/search", get(handlers::get_indicators_by_category))
        .route("/api/trend", get(handlers::get_trend_comparison))
        .layer(CorsLayer::permissive())
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("服务器启动于 http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
