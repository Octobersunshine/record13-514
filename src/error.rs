use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("PDF解析失败: {0}")]
    PdfParse(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("未找到报告: {0}")]
    NotFound(String),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::PdfParse(_) => (axum::http::StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            AppError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::NotFound(_) => (axum::http::StatusCode::NOT_FOUND, self.to_string()),
            AppError::Io(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
    }
}
