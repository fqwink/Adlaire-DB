use crate::{db::sqld_adapter::{SqlResult, SqlValue}, error::AppError};
use super::types::{Col, StmtResult, Value};

/// hrana `Value` → SQL 実行用 `SqlValue`
pub fn hrana_to_sql(v: &Value) -> Result<SqlValue, AppError> {
    Ok(match v {
        Value::Null             => SqlValue::Null,
        Value::Integer { value } => {
            let n = value.parse::<i64>().map_err(|_| {
                AppError::InvalidRequest
            })?;
            SqlValue::Integer(n)
        }
        Value::Real    { value } => SqlValue::Real(*value),
        Value::Text    { value } => SqlValue::Text(value.clone()),
        Value::Blob    { value } => {
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(value)
                .map_err(|_| AppError::InvalidRequest)?;
            SqlValue::Blob(bytes)
        }
    })
}

/// `SqlResult` → hrana `StmtResult`
pub fn sql_to_stmt_result(r: SqlResult) -> StmtResult {
    let rows = r.rows.into_iter().map(|row| {
        row.into_iter().map(sql_val_to_hrana).collect()
    }).collect();
    StmtResult {
        cols: r.cols.into_iter().map(|(name, decltype)| Col { name, decltype }).collect(),
        rows,
        rows_affected:     r.rows_affected,
        last_insert_rowid: r.last_insert_rowid.map(|n| n.to_string()),
    }
}

fn sql_val_to_hrana(v: SqlValue) -> Value {
    match v {
        SqlValue::Null       => Value::Null,
        SqlValue::Integer(n) => Value::Integer { value: n.to_string() },
        SqlValue::Real(f)    => Value::Real    { value: f },
        SqlValue::Text(s)    => Value::Text    { value: s },
        SqlValue::Blob(b)    => {
            use base64::Engine as _;
            Value::Blob { value: base64::engine::general_purpose::STANDARD.encode(&b) }
        }
    }
}
