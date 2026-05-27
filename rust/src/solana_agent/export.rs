//! Export functionality for query results to Parquet/JSON/CSV format
//! Enables downstream analytics with Python (Pandas, Polars), DuckDB, Spark, etc.

use serde_json::Value;
use std::path::Path;
use thiserror::Error;

use super::query_engine::QueryResult;

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Polars error: {0}")]
    Polars(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("No data to export")]
    NoData,
}

/// Export format options
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Parquet,
    Json,
    Csv,
}

/// Export query result to file
pub fn export_query_result(
    result: &QueryResult,
    path: &Path,
    format: ExportFormat,
) -> Result<(), ExportError> {
    if result.rows.is_empty() {
        return Err(ExportError::NoData);
    }

    match format {
        ExportFormat::Parquet => export_to_parquet(result, path),
        ExportFormat::Json => export_to_json(result, path),
        ExportFormat::Csv => export_to_csv(result, path),
    }
}

/// Export to Parquet format
fn export_to_parquet(_result: &QueryResult, _path: &Path) -> Result<(), ExportError> {
    Err(ExportError::Polars("Polars export not implemented".to_string()))
}

/// Export to JSON format
fn export_to_json(result: &QueryResult, path: &Path) -> Result<(), ExportError> {
    let json_output = serde_json::to_string_pretty(result)
        .map_err(|e| ExportError::Serialization(e.to_string()))?;

    std::fs::write(path, json_output)
        .map_err(|e| ExportError::Io(e.to_string()))?;

    Ok(())
}

/// Export to CSV format
fn export_to_csv(result: &QueryResult, path: &Path) -> Result<(), ExportError> {
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| ExportError::Io(e.to_string()))?;

    // Write header
    wtr.write_record(&result.columns)
        .map_err(|e| ExportError::Io(e.to_string()))?;

    // Write rows
    for row in &result.rows {
        let csv_row: Vec<String> = row.iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => String::new(),
                _ => v.to_string(),
            })
            .collect();
        
        wtr.write_record(&csv_row)
            .map_err(|e| ExportError::Io(e.to_string()))?;
    }

    wtr.flush()
        .map_err(|e| ExportError::Io(e.to_string()))?;

    Ok(())
}

/// Export multiple query results to a single Parquet file
pub fn export_multiple_to_parquet(
    _results: Vec<QueryResult>,
    _path: &Path,
) -> Result<(), ExportError> {
    Err(ExportError::Polars("Polars export not implemented".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_export_to_json() {
        let result = QueryResult {
            columns: vec!["name".to_string()],
            rows: vec![vec![json!("test")]],
            row_count: 1,
            execution_time_ms: 10.0,
            query_type: "test".to_string(),
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.json");
        
        export_to_json(&result, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_export_no_data() {
        let result = QueryResult {
            columns: vec!["name".to_string()],
            rows: vec![],
            row_count: 0,
            execution_time_ms: 10.0,
            query_type: "test".to_string(),
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.json");
        
        let result = export_to_json(&result, &path);
        assert!(matches!(result, Err(ExportError::NoData)));
    }
}
