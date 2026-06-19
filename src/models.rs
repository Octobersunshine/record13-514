use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub filename: String,
    pub uploaded_at: String,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indicator {
    pub id: u64,
    pub report_id: String,
    pub category: String,
    pub name: String,
    pub value: String,
    pub unit: String,
    pub reference_range: String,
    pub is_abnormal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportWithIndicators {
    pub report: Report,
    pub indicators: Vec<Indicator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIndicator {
    pub category: String,
    pub name: String,
    pub value: String,
    pub unit: String,
    pub reference_range: String,
    pub is_abnormal: bool,
}
