use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use redb::{Database, ReadableTable, TableDefinition};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::AppError;
use crate::models::{Indicator, IndicatorTrend, ParsedIndicator, Report, ReportWithIndicators, TrendComparison};

const REPORTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("reports");
const INDICATORS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("indicators");
const META_TABLE: TableDefinition<&str, u64> = TableDefinition::new("meta");

static INDICATOR_ID: AtomicU64 = AtomicU64::new(1);

pub struct Db {
    db: Arc<Database>,
}

impl Db {
    pub fn new(path: &str) -> Result<Self, AppError> {
        let db = Database::create(path).map_err(|e| AppError::Database(e.to_string()))?;
        let db_arc = Arc::new(db);

        let instance = Self { db: db_arc };
        instance.init_tables()?;

        if let Ok(next_id) = instance.get_next_indicator_id() {
            INDICATOR_ID.store(next_id, Ordering::SeqCst);
        }

        Ok(instance)
    }

    fn init_tables(&self) -> Result<(), AppError> {
        let write_tx = self.db.begin_write().map_err(|e| AppError::Database(e.to_string()))?;
        {
            let _ = write_tx.open_table(REPORTS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            let _ = write_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            let mut meta = write_tx.open_table(META_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            if meta.get("next_indicator_id").unwrap_or(None).is_none() {
                meta.insert("next_indicator_id", 1u64).map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        write_tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_next_indicator_id(&self) -> Result<u64, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let meta = read_tx.open_table(META_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let val = meta.get("next_indicator_id").map_err(|e| AppError::Database(e.to_string()))?;
        Ok(val.map(|g| g.value()).unwrap_or(1u64))
    }

    fn save_next_indicator_id(&self, id: u64) -> Result<(), AppError> {
        let write_tx = self.db.begin_write().map_err(|e| AppError::Database(e.to_string()))?;
        {
            let mut meta = write_tx.open_table(META_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            meta.insert("next_indicator_id", id).map_err(|e| AppError::Database(e.to_string()))?;
        }
        write_tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn serialize<T: Serialize>(val: &T) -> Result<Vec<u8>, AppError> {
        serde_json::to_vec(val).map_err(|e| AppError::Database(e.to_string()))
    }

    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, AppError> {
        serde_json::from_slice(bytes).map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn insert_report(&self, report: &Report) -> Result<(), AppError> {
        let write_tx = self.db.begin_write().map_err(|e| AppError::Database(e.to_string()))?;
        {
            let mut table = write_tx.open_table(REPORTS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            let bytes = Self::serialize(report)?;
            table.insert(report.id.as_str(), bytes.as_slice()).map_err(|e| AppError::Database(e.to_string()))?;
        }
        write_tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn insert_indicators(&self, report_id: &str, indicators: &[ParsedIndicator]) -> Result<Vec<Indicator>, AppError> {
        let mut results = Vec::new();
        let write_tx = self.db.begin_write().map_err(|e| AppError::Database(e.to_string()))?;
        {
            let mut table = write_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            for parsed in indicators {
                let id = INDICATOR_ID.fetch_add(1, Ordering::SeqCst);
                let indicator = Indicator {
                    id,
                    report_id: report_id.to_string(),
                    category: parsed.category.clone(),
                    name: parsed.name.clone(),
                    value: parsed.value.clone(),
                    unit: parsed.unit.clone(),
                    reference_range: parsed.reference_range.clone(),
                    is_abnormal: parsed.is_abnormal,
                };
                let bytes = Self::serialize(&indicator)?;
                table.insert(id, bytes.as_slice()).map_err(|e| AppError::Database(e.to_string()))?;
                results.push(indicator);
            }
        }
        write_tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        self.save_next_indicator_id(INDICATOR_ID.load(Ordering::SeqCst))?;
        Ok(results)
    }

    pub fn list_reports(&self) -> Result<Vec<Report>, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let table = read_tx.open_table(REPORTS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let mut reports: Vec<Report> = Vec::new();
        for entry in table.iter().map_err(|e| AppError::Database(e.to_string()))? {
            let (_, val) = entry.map_err(|e| AppError::Database(e.to_string()))?;
            let report: Report = Self::deserialize(val.value())?;
            reports.push(report);
        }
        reports.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
        Ok(reports)
    }

    pub fn get_report(&self, id: &str) -> Result<Report, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let table = read_tx.open_table(REPORTS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let val = table.get(id).map_err(|e| AppError::Database(e.to_string()))?;
        match val {
            Some(guard) => Self::deserialize(guard.value()),
            None => Err(AppError::NotFound(format!("报告 {} 不存在", id))),
        }
    }

    pub fn get_report_with_indicators(&self, id: &str) -> Result<ReportWithIndicators, AppError> {
        let report = self.get_report(id)?;
        let indicators = self.get_indicators_by_report(id)?;
        Ok(ReportWithIndicators { report, indicators })
    }

    pub fn get_indicators_by_report(&self, report_id: &str) -> Result<Vec<Indicator>, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let table = read_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let mut indicators: Vec<Indicator> = Vec::new();
        for entry in table.iter().map_err(|e| AppError::Database(e.to_string()))? {
            let (_, val) = entry.map_err(|e| AppError::Database(e.to_string()))?;
            let indicator: Indicator = Self::deserialize(val.value())?;
            if indicator.report_id == report_id {
                indicators.push(indicator);
            }
        }
        indicators.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
        Ok(indicators)
    }

    pub fn get_abnormal_indicators(&self) -> Result<Vec<Indicator>, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let table = read_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let mut indicators: Vec<Indicator> = Vec::new();
        for entry in table.iter().map_err(|e| AppError::Database(e.to_string()))? {
            let (_, val) = entry.map_err(|e| AppError::Database(e.to_string()))?;
            let indicator: Indicator = Self::deserialize(val.value())?;
            if indicator.is_abnormal {
                indicators.push(indicator);
            }
        }
        indicators.sort_by(|a, b| a.report_id.cmp(&b.report_id).then(a.category.cmp(&b.category)));
        Ok(indicators)
    }

    pub fn get_indicators_by_category(&self, category: &str) -> Result<Vec<Indicator>, AppError> {
        let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
        let table = read_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
        let mut indicators: Vec<Indicator> = Vec::new();
        for entry in table.iter().map_err(|e| AppError::Database(e.to_string()))? {
            let (_, val) = entry.map_err(|e| AppError::Database(e.to_string()))?;
            let indicator: Indicator = Self::deserialize(val.value())?;
            if indicator.category == category {
                indicators.push(indicator);
            }
        }
        indicators.sort_by(|a, b| a.report_id.cmp(&b.report_id).then(a.id.cmp(&b.id)));
        Ok(indicators)
    }

    pub fn delete_report(&self, id: &str) -> Result<(), AppError> {
        self.get_report(id)?;

        let write_tx = self.db.begin_write().map_err(|e| AppError::Database(e.to_string()))?;
        {
            let mut report_table = write_tx.open_table(REPORTS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            report_table.remove(id).map_err(|e| AppError::Database(e.to_string()))?;

            let mut indicator_table = write_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            let read_tx = self.db.begin_read().map_err(|e| AppError::Database(e.to_string()))?;
            let read_indicators = read_tx.open_table(INDICATORS_TABLE).map_err(|e| AppError::Database(e.to_string()))?;
            let mut ids_to_remove = Vec::new();
            for entry in read_indicators.iter().map_err(|e| AppError::Database(e.to_string()))? {
                let (key, val) = entry.map_err(|e| AppError::Database(e.to_string()))?;
                let indicator: Indicator = Self::deserialize(val.value())?;
                if indicator.report_id == id {
                    ids_to_remove.push(key.value());
                }
            }
            for key_id in ids_to_remove {
                indicator_table.remove(key_id).map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        write_tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_trend_comparison(&self) -> Result<TrendComparison, AppError> {
        let reports = self.list_reports()?;
        if reports.len() < 2 {
            return Err(AppError::NotFound("至少需要两份体检报告才能进行趋势对比".to_string()));
        }

        let latest_report = &reports[0];
        let previous_report = &reports[1];

        let latest_indicators = self.get_indicators_by_report(&latest_report.id)?;
        let previous_indicators = self.get_indicators_by_report(&previous_report.id)?;

        use std::collections::HashMap;
        let mut prev_map: HashMap<String, &Indicator> = HashMap::new();
        for ind in &previous_indicators {
            prev_map.insert(ind.name.clone(), ind);
        }

        let mut trends = Vec::new();
        let mut new_indicators = Vec::new();

        for latest in &latest_indicators {
            if let Some(prev) = prev_map.get(&latest.name) {
                if let (Ok(latest_num), Ok(prev_num)) = (
                    latest.value.parse::<f64>(),
                    prev.value.parse::<f64>(),
                ) {
                    let diff = latest_num - prev_num;
                    let diff_percent = if prev_num != 0.0 {
                        (diff / prev_num.abs()) * 100.0
                    } else {
                        0.0
                    };

                    let direction = if diff > 0.0 {
                        "上升".to_string()
                    } else if diff < 0.0 {
                        "下降".to_string()
                    } else {
                        "不变".to_string()
                    };

                    trends.push(IndicatorTrend {
                        name: latest.name.clone(),
                        category: latest.category.clone(),
                        unit: latest.unit.clone(),
                        latest_value: latest.value.clone(),
                        previous_value: prev.value.clone(),
                        diff_value: diff,
                        diff_percent: (diff_percent * 100.0).round() / 100.0,
                        direction,
                        latest_report_id: latest_report.id.clone(),
                        previous_report_id: previous_report.id.clone(),
                        latest_uploaded_at: latest_report.uploaded_at.clone(),
                        previous_uploaded_at: previous_report.uploaded_at.clone(),
                    });
                }
                prev_map.remove(&latest.name);
            } else {
                new_indicators.push(latest.clone());
            }
        }

        let mut removed_indicators: Vec<Indicator> = Vec::new();
        for (_, ind) in prev_map {
            removed_indicators.push(ind.clone());
        }

        trends.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        new_indicators.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        removed_indicators.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));

        Ok(TrendComparison {
            latest_report: latest_report.clone(),
            previous_report: previous_report.clone(),
            trends,
            new_indicators,
            removed_indicators,
        })
    }
}
