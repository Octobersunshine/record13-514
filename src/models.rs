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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorTrend {
    pub name: String,
    pub category: String,
    pub unit: String,
    pub latest_value: String,
    pub previous_value: String,
    pub diff_value: f64,
    pub diff_percent: f64,
    pub direction: String,
    pub latest_report_id: String,
    pub previous_report_id: String,
    pub latest_uploaded_at: String,
    pub previous_uploaded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendComparison {
    pub latest_report: Report,
    pub previous_report: Report,
    pub trends: Vec<IndicatorTrend>,
    pub new_indicators: Vec<Indicator>,
    pub removed_indicators: Vec<Indicator>,
}
