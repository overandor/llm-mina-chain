//! Export functionality for query results to Parquet/JSON/CSV format
//! Enables downstream analytics with Python (Pandas, Polars), DuckDB, Spark, etc.

use serde_json::Value;
use std::path::Path;
use thiserror::Error;

#[cfg(feature = "polars")]
use polars::prelude::*;

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

/// Convert QueryResult to Polars DataFrame
#[cfg(feature = "polars")]
fn query_result_to_dataframe(result: &QueryResult) -> Result<DataFrame, ExportError> {
    let mut series_map = Vec::new();

    for (col_idx, col_name) in result.columns.iter().enumerate() {
        let mut string_values = Vec::new();
        let mut uint64_values = Vec::new();
        let mut float64_values = Vec::new();

        for row in &result.rows {
            if col_idx < row.len() {
                match &row[col_idx] {
                    Value::String(s) => {
                        string_values.push(s.as_str());
                        uint64_values.push(0);
                        float64_values.push(0.0);
                    }
                    Value::Number(n) => {
                        if n.is_i64() || n.is_u64() {
                            string_values.push("");
                            uint64_values.push(n.as_u64().unwrap_or(0));
                            float64_values.push(0.0);
                        } else {
                            string_values.push("");
                            uint64_values.push(0);
                            float64_values.push(n.as_f64().unwrap_or(0.0));
                        }
                    }
                    Value::Bool(b) => {
                        string_values.push(if *b { "true" } else { "false" });
                        uint64_values.push(if *b { 1 } else { 0 });
                        float64_values.push(if *b { 1.0 } else { 0.0 });
                    }
                    Value::Null => {
                        string_values.push("");
                        uint64_values.push(0);
                        float64_values.push(0.0);
                    }
                    _ => {
                        string_values.push(&row[col_idx].to_string());
                        uint64_values.push(0);
                        float64_values.push(0.0);
                    }
                }
            }
        }

        // Determine type based on data
        let has_strings = string_values.iter().any(|s| !s.is_empty());
        let has_uints = uint64_values.iter().any(|&n| n > 0);
        let has_floats = float64_values.iter().any(|&f| f > 0.0);

        let series: Series = if has_strings {
            StringChunked::new(col_name, &string_values).into_series()
        } else if has_floats {
            Float64Chunked::new(col_name, &float64_values).into_series()
        } else if has_uints {
            UInt64Chunked::new(col_name, &uint64_values).into_series()
        } else {
            StringChunked::new(col_name, &string_values).into_series()
        };

        series_map.push(series);
    }

    DataFrame::new(series_map)
        .map_err(|e| ExportError::Polars(e.to_string()))
}

/// Export to Parquet format
fn export_to_parquet(result: &QueryResult, path: &Path) -> Result<(), ExportError> {
    #[cfg(feature = "polars")]
    {
        let df = query_result_to_dataframe(result)?;

        let file = std::fs::File::create(path)
            .map_err(|e| ExportError::Io(e.to_string()))?;
        
        ParquetWriter::new(file)
            .finish(&mut df.clone())
            .map_err(|e| ExportError::Polars(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "polars"))]
    {
        let _ = path; // Suppress unused warning when feature is disabled
        Err(ExportError::Polars("Polars feature not enabled".to_string()))
    }
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
#[cfg(feature = "polars")]
pub fn export_multiple_to_parquet(
    results: Vec<QueryResult>,
    path: &Path,
) -> Result<(), ExportError> {
    if results.is_empty() {
        return Err(ExportError::NoData);
    }

    let mut dfs = Vec::new();
    for result in &results {
        let df = query_result_to_dataframe(result)?;
        dfs.push(df);
    }

    // Concatenate all dataframes
    let combined_df = polars::functions::concat(&dfs, false)
        .map_err(|e| ExportError::Polars(e.to_string()))?;

    let file = std::fs::File::create(path)
        .map_err(|e| ExportError::Io(e.to_string()))?;
    
    ParquetWriter::new(file)
        .finish(&mut combined_df.clone())
        .map_err(|e| ExportError::Polars(e.to_string()))?;

    Ok(())
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
