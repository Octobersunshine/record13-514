use lopdf::Document;
use regex::Regex;

use crate::error::AppError;
use crate::models::ParsedIndicator;

struct IndicatorDef {
    category: &'static str,
    names: &'static [&'static str],
    unit: &'static str,
    ref_low: f64,
    ref_high: f64,
}

fn get_indicator_definitions() -> Vec<IndicatorDef> {
    vec![
        IndicatorDef { category: "血常规", names: &["白细胞", "WBC", "白细胞计数"], unit: "10^9/L", ref_low: 3.5, ref_high: 9.5 },
        IndicatorDef { category: "血常规", names: &["红细胞", "RBC", "红细胞计数"], unit: "10^12/L", ref_low: 4.3, ref_high: 5.8 },
        IndicatorDef { category: "血常规", names: &["血红蛋白", "HGB", "Hb"], unit: "g/L", ref_low: 130.0, ref_high: 175.0 },
        IndicatorDef { category: "血常规", names: &["血小板", "PLT", "血小板计数"], unit: "10^9/L", ref_low: 125.0, ref_high: 350.0 },
        IndicatorDef { category: "血常规", names: &["中性粒细胞", "NEUT", "中性粒细胞比率"], unit: "%", ref_low: 40.0, ref_high: 75.0 },
        IndicatorDef { category: "血常规", names: &["淋巴细胞", "LYMPH", "淋巴细胞比率"], unit: "%", ref_low: 20.0, ref_high: 50.0 },
        IndicatorDef { category: "血常规", names: &["单核细胞", "MONO", "单核细胞比率"], unit: "%", ref_low: 3.0, ref_high: 10.0 },
        IndicatorDef { category: "肝功能", names: &["谷丙转氨酶", "ALT", "丙氨酸氨基转移酶"], unit: "U/L", ref_low: 0.0, ref_high: 40.0 },
        IndicatorDef { category: "肝功能", names: &["谷草转氨酶", "AST", "天门冬氨酸氨基转移酶"], unit: "U/L", ref_low: 0.0, ref_high: 40.0 },
        IndicatorDef { category: "肝功能", names: &["总胆红素", "TBIL"], unit: "μmol/L", ref_low: 5.1, ref_high: 22.2 },
        IndicatorDef { category: "肝功能", names: &["直接胆红素", "DBIL"], unit: "μmol/L", ref_low: 0.0, ref_high: 6.8 },
        IndicatorDef { category: "肝功能", names: &["白蛋白", "ALB"], unit: "g/L", ref_low: 40.0, ref_high: 55.0 },
        IndicatorDef { category: "肝功能", names: &["球蛋白", "GLB"], unit: "g/L", ref_low: 20.0, ref_high: 35.0 },
        IndicatorDef { category: "肾功能", names: &["肌酐", "Cr", "CREA", "血肌酐"], unit: "μmol/L", ref_low: 57.0, ref_high: 111.0 },
        IndicatorDef { category: "肾功能", names: &["尿素氮", "BUN"], unit: "mmol/L", ref_low: 3.1, ref_high: 8.0 },
        IndicatorDef { category: "肾功能", names: &["尿酸", "UA", "血尿酸"], unit: "μmol/L", ref_low: 208.0, ref_high: 428.0 },
        IndicatorDef { category: "血糖血脂", names: &["空腹血糖", "FBG", "GLU", "血糖"], unit: "mmol/L", ref_low: 3.9, ref_high: 6.1 },
        IndicatorDef { category: "血糖血脂", names: &["总胆固醇", "TC", "CHOL"], unit: "mmol/L", ref_low: 2.8, ref_high: 5.7 },
        IndicatorDef { category: "血糖血脂", names: &["甘油三酯", "TG"], unit: "mmol/L", ref_low: 0.56, ref_high: 1.7 },
        IndicatorDef { category: "血糖血脂", names: &["高密度脂蛋白", "HDL-C", "HDL"], unit: "mmol/L", ref_low: 1.0, ref_high: 1.9 },
        IndicatorDef { category: "血糖血脂", names: &["低密度脂蛋白", "LDL-C", "LDL"], unit: "mmol/L", ref_low: 0.0, ref_high: 3.4 },
    ]
}

pub fn extract_text_from_pdf(data: &[u8]) -> Result<String, AppError> {
    let doc = Document::load_mem(data).map_err(|e| AppError::PdfParse(format!("无法加载PDF: {}", e)))?;
    let mut text = String::new();
    let pages = doc.get_pages();
    for (page_num, _obj_id) in &pages {
        if let Ok(page_text) = extract_page_text(&doc, *page_num) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }
    Ok(text)
}

fn extract_page_text(doc: &Document, _page_num: u32) -> Result<String, AppError> {
    let mut text_parts: Vec<String> = Vec::new();

    fn collect_text_from_object(doc: &Document, obj: &lopdf::Object, parts: &mut Vec<String>) {
        match obj {
            lopdf::Object::Stream(stream) => {
                if let Ok(decoded) = stream.decompressed_content() {
                    let content_str = String::from_utf8_lossy(&decoded);
                    extract_text_operators(&content_str, parts);
                }
            }
            lopdf::Object::Array(arr) => {
                for item in arr {
                    if let Ok(obj_ref) = item.as_reference() {
                        if let Ok(child) = doc.get_object(obj_ref) {
                            collect_text_from_object(doc, child, parts);
                        }
                    }
                }
            }
            lopdf::Object::Reference(id) => {
                if let Ok(child) = doc.get_object(*id) {
                    collect_text_from_object(doc, child, parts);
                }
            }
            _ => {}
        }
    }

    let pages = doc.get_pages();
    if let Some(&obj_id) = pages.get(&_page_num) {
        if let Ok(page_obj) = doc.get_object(obj_id) {
            if let Some(contents_key) = page_obj.as_dict().ok().and_then(|d| d.get(b"Contents").ok()) {
                match contents_key {
                    lopdf::Object::Reference(ref_id) => {
                        if let Ok(obj) = doc.get_object(*ref_id) {
                            collect_text_from_object(doc, obj, &mut text_parts);
                        }
                    }
                    lopdf::Object::Array(arr) => {
                        for item in arr {
                            if let Ok(ref_id) = item.as_reference() {
                                if let Ok(obj) = doc.get_object(ref_id) {
                                    collect_text_from_object(doc, obj, &mut text_parts);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(text_parts.join(" "))
}

fn extract_text_operators(content: &str, parts: &mut Vec<String>) {
    let paren_re = Regex::new(r"\(([^)]*)\)").unwrap();
    for cap in paren_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let s = m.as_str();
            if !s.is_empty() && s.chars().any(|c| !c.is_control()) {
                parts.push(s.to_string());
            }
        }
    }

    let hex_re = Regex::new(r"<([0-9A-Fa-f]+)>").unwrap();
    for cap in hex_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let hex = m.as_str();
            if hex.len() % 2 == 0 {
                let bytes: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                    .collect();
                let decoded = String::from_utf8_lossy(&bytes);
                if !decoded.is_empty() && decoded.chars().any(|c| !c.is_control()) {
                    parts.push(decoded.into_owned());
                }
            }
        }
    }
}

pub fn parse_indicators(text: &str) -> Vec<ParsedIndicator> {
    let definitions = get_indicator_definitions();
    let mut results = Vec::new();
    let mut found_names: Vec<String> = Vec::new();

    for def in &definitions {
        let primary_name = def.names[0];
        if found_names.iter().any(|n| n == primary_name) {
            continue;
        }

        if let Some(indicator) = try_parse_indicator(text, def) {
            found_names.push(primary_name.to_string());
            results.push(indicator);
        }
    }

    results
}

fn try_parse_indicator(text: &str, def: &IndicatorDef) -> Option<ParsedIndicator> {
    let range_pat = r"(\d+\.?\d*)\s*[-–—~～]\s*(\d+\.?\d*)";
    let marker = r"(?:[↑↑]|\bH\b|偏高|高)";
    let marker_low = r"(?:[↓↓]|\bL\b|偏低|低)";
    let unit_escaped = regex::escape(def.unit);

    for &name in def.names {
        let escaped = regex::escape(name);

        let patterns: Vec<String> = vec![
            format!(r"{}\s*[：:]*\s*(\d+\.?\d*)\s*(?:{marker})?\s*{unit_escaped}\s*{range_pat}", escaped),
            format!(r"{}\s+(\d+\.?\d*)\s*(?:{marker})?\s+{unit_escaped}\s+{range_pat}", escaped),
            format!(r"{}\s*[：:]*\s*(\d+\.?\d*)\s*(?:{marker})?\s*{range_pat}\s*{unit_escaped}", escaped),
            format!(r"{}\s+(\d+\.?\d*)\s*(?:{marker})?\s+{range_pat}\s+{unit_escaped}", escaped),
            format!(r"{}\s*[：:]*\s*(\d+\.?\d*)\s*(?:{marker})?\s*{unit_escaped}", escaped),
            format!(r"{}\s+(\d+\.?\d*)\s*(?:{marker})?\s+{unit_escaped}", escaped),
            format!(r"{}\s*[：:]+\s*(\d+\.?\d*)\s*(?:{marker})?", escaped),
            format!(r"{}\s+(\d+\.?\d*)\s*(?:{marker})?", escaped),
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(text) {
                    if let Some(value_match) = caps.get(1) {
                        let value_str = value_match.as_str();
                        if let Ok(num_value) = value_str.parse::<f64>() {
                            let ref_low = caps.get(2)
                                .and_then(|m| m.as_str().parse::<f64>().ok());
                            let ref_high = caps.get(3)
                                .and_then(|m| m.as_str().parse::<f64>().ok());

                            let ctx_start = value_match.start().saturating_sub(5);
                            let ctx_end = (value_match.end() + 30).min(text.len());
                            let context = &text[ctx_start..ctx_end];
                            let has_high_marker = Regex::new(marker).unwrap().is_match(context);
                            let has_low_marker = Regex::new(marker_low).unwrap().is_match(context);

                            let final_low = ref_low.unwrap_or(def.ref_low);
                            let final_high = ref_high.unwrap_or(def.ref_high);
                            let is_abnormal = has_high_marker
                                || has_low_marker
                                || num_value < final_low
                                || num_value > final_high;

                            let ref_range = format!("{}-{}", final_low, final_high);
                            return Some(ParsedIndicator {
                                category: def.category.to_string(),
                                name: def.names[0].to_string(),
                                value: value_str.to_string(),
                                unit: def.unit.to_string(),
                                reference_range: ref_range,
                                is_abnormal,
                            });
                        }
                    }
                }
            }
        }

        if let Some(parsed) = try_parse_indicator_ref_range_first(text, name, def, &unit_escaped, &range_pat) {
            return Some(parsed);
        }
    }

    None
}

fn try_parse_indicator_ref_range_first(
    text: &str,
    name: &str,
    def: &IndicatorDef,
    unit_escaped: &str,
    range_pat: &str,
) -> Option<ParsedIndicator> {
    let escaped = regex::escape(name);
    let marker = r"(?:[↑↑]|\bH\b|偏高|高)";
    let marker_low = r"(?:[↓↓]|\bL\b|偏低|低)";

    let patterns: Vec<String> = vec![
        // ref_range + value + marker + unit
        format!(r"{}\s*[：:]*\s*{range_pat}\s+(\d+\.?\d*)\s*(?:{marker})?\s*{unit_escaped}", escaped),
        format!(r"{}\s+{range_pat}\s+(\d+\.?\d*)\s*(?:{marker})?\s+{unit_escaped}", escaped),
        // ref_range + value + marker (no unit)
        format!(r"{}\s*[：:]*\s*{range_pat}\s+(\d+\.?\d*)\s*(?:{marker})?", escaped),
        format!(r"{}\s+{range_pat}\s+(\d+\.?\d*)\s*(?:{marker})?", escaped),
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(text) {
                let ref_low = caps.get(1)
                    .and_then(|m| m.as_str().parse::<f64>().ok());
                let ref_high = caps.get(2)
                    .and_then(|m| m.as_str().parse::<f64>().ok());
                if let Some(value_match) = caps.get(3) {
                    let value_str = value_match.as_str();
                    if let Ok(num_value) = value_str.parse::<f64>() {
                        let ctx_start = value_match.start().saturating_sub(5);
                        let ctx_end = (value_match.end() + 30).min(text.len());
                        let context = &text[ctx_start..ctx_end];
                        let has_high_marker = Regex::new(marker).unwrap().is_match(context);
                        let has_low_marker = Regex::new(marker_low).unwrap().is_match(context);

                        let final_low = ref_low.unwrap_or(def.ref_low);
                        let final_high = ref_high.unwrap_or(def.ref_high);
                        let is_abnormal = has_high_marker
                            || has_low_marker
                            || num_value < final_low
                            || num_value > final_high;

                        let ref_range = format!("{}-{}", final_low, final_high);
                        return Some(ParsedIndicator {
                            category: def.category.to_string(),
                            name: def.names[0].to_string(),
                            value: value_str.to_string(),
                            unit: def.unit.to_string(),
                            reference_range: ref_range,
                            is_abnormal,
                        });
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_blood_routine() {
        let text = "血常规检查\n白细胞 5.2 10^9/L\n红细胞 4.8 10^12/L\n血红蛋白 145 g/L\n血小板 220 10^9/L";
        let indicators = parse_indicators(text);
        assert!(indicators.len() >= 4);
        let wbc = indicators.iter().find(|i| i.name == "白细胞").unwrap();
        assert_eq!(wbc.value, "5.2");
        assert!(!wbc.is_abnormal);
    }

    #[test]
    fn test_parse_liver_function() {
        let text = "肝功能\n谷丙转氨酶 58 U/L\n谷草转氨酶 25 U/L\n总胆红素 15.3 μmol/L";
        let indicators = parse_indicators(text);
        let alt = indicators.iter().find(|i| i.name == "谷丙转氨酶").unwrap();
        assert_eq!(alt.value, "58");
        assert!(alt.is_abnormal);
    }

    #[test]
    fn test_parse_with_colon() {
        let text = "空腹血糖：5.6 mmol/L";
        let indicators = parse_indicators(text);
        let fbg = indicators.iter().find(|i| i.name == "空腹血糖").unwrap();
        assert_eq!(fbg.value, "5.6");
        assert!(!fbg.is_abnormal);
    }

    #[test]
    fn test_ref_range_before_value() {
        let text = "谷丙转氨酶 0-40 58 U/L";
        let indicators = parse_indicators(text);
        let alt = indicators.iter().find(|i| i.name == "谷丙转氨酶").unwrap();
        assert_eq!(alt.value, "58");
        assert!(alt.is_abnormal);
    }

    #[test]
    fn test_value_before_ref_range() {
        let text = "尿酸 450 208-428 μmol/L";
        let indicators = parse_indicators(text);
        let ua = indicators.iter().find(|i| i.name == "尿酸").unwrap();
        assert_eq!(ua.value, "450");
        assert!(ua.is_abnormal);
    }

    #[test]
    fn test_value_with_high_marker() {
        let text = "谷丙转氨酶 58 ↑ U/L";
        let indicators = parse_indicators(text);
        let alt = indicators.iter().find(|i| i.name == "谷丙转氨酶").unwrap();
        assert_eq!(alt.value, "58");
        assert!(alt.is_abnormal);
    }

    #[test]
    fn test_value_with_low_marker() {
        let text = "白蛋白 38 ↓ g/L";
        let indicators = parse_indicators(text);
        let alb = indicators.iter().find(|i| i.name == "白蛋白").unwrap();
        assert_eq!(alb.value, "38");
        assert!(alb.is_abnormal);
    }

    #[test]
    fn test_pdf_ref_range_overrides_hardcoded() {
        let text = "空腹血糖 7.8 3.9-6.1 mmol/L";
        let indicators = parse_indicators(text);
        let fbg = indicators.iter().find(|i| i.name == "空腹血糖").unwrap();
        assert_eq!(fbg.value, "7.8");
        assert_eq!(fbg.reference_range, "3.9-6.1");
        assert!(fbg.is_abnormal);
    }

    #[test]
    fn test_normal_value_with_ref_range() {
        let text = "白细胞 5.2 3.5-9.5 10^9/L";
        let indicators = parse_indicators(text);
        let wbc = indicators.iter().find(|i| i.name == "白细胞").unwrap();
        assert_eq!(wbc.value, "5.2");
        assert!(!wbc.is_abnormal);
    }

    #[test]
    fn test_ref_range_first_with_marker() {
        let text = "谷丙转氨酶 0-40 58 ↑ U/L";
        let indicators = parse_indicators(text);
        let alt = indicators.iter().find(|i| i.name == "谷丙转氨酶").unwrap();
        assert_eq!(alt.value, "58");
        assert!(alt.is_abnormal);
    }
}
