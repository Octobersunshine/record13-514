use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::Db;
use crate::error::AppError;
use crate::models::{Indicator, Report, ReportWithIndicators};
use crate::pdf_parser;

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub category: Option<String>,
}

pub async fn upload_pdf(
    State(db): State<Arc<Db>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ReportWithIndicators>), AppError> {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut filename = String::from("unknown.pdf");

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::PdfParse(format!("读取上传字段失败: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            filename = field.file_name().unwrap_or("unknown.pdf").to_string();
            pdf_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::PdfParse(format!("读取文件数据失败: {}", e)))?
                    .to_vec(),
            );
        }
    }

    let data = pdf_data.ok_or_else(|| AppError::PdfParse("未找到上传的PDF文件".to_string()))?;

    let raw_text = pdf_parser::extract_text_from_pdf(&data)?;

    let parsed_indicators = pdf_parser::parse_indicators(&raw_text);

    let report_id = uuid::Uuid::new_v4().to_string();
    let uploaded_at = chrono::Utc::now().to_rfc3339();

    let report = Report {
        id: report_id.clone(),
        filename,
        uploaded_at,
        raw_text,
    };
    db.insert_report(&report)?;

    let indicators = db.insert_indicators(&report_id, &parsed_indicators)?;

    Ok((StatusCode::CREATED, Json(ReportWithIndicators { report, indicators })))
}

pub async fn list_reports(
    State(db): State<Arc<Db>>,
) -> Result<Json<Vec<Report>>, AppError> {
    let reports = db.list_reports()?;
    Ok(Json(reports))
}

pub async fn get_report(
    State(db): State<Arc<Db>>,
    Path(id): Path<String>,
) -> Result<Json<ReportWithIndicators>, AppError> {
    let result = db.get_report_with_indicators(&id)?;
    Ok(Json(result))
}

pub async fn get_indicators(
    State(db): State<Arc<Db>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Indicator>>, AppError> {
    let indicators = db.get_indicators_by_report(&id)?;
    Ok(Json(indicators))
}

pub async fn get_abnormal_indicators(
    State(db): State<Arc<Db>>,
) -> Result<Json<Vec<Indicator>>, AppError> {
    let indicators = db.get_abnormal_indicators()?;
    Ok(Json(indicators))
}

pub async fn get_indicators_by_category(
    State(db): State<Arc<Db>>,
    Query(query): Query<CategoryQuery>,
) -> Result<Json<Vec<Indicator>>, AppError> {
    match query.category {
        Some(cat) => {
            let indicators = db.get_indicators_by_category(&cat)?;
            Ok(Json(indicators))
        }
        None => {
            let indicators = db.get_abnormal_indicators()?;
            Ok(Json(indicators))
        }
    }
}

pub async fn delete_report(
    State(db): State<Arc<Db>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    db.delete_report(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn health_check() -> &'static str {
    "OK"
}
