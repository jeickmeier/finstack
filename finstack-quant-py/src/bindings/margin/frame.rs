//! pandas ``DataFrame`` ingestion helpers shared by the margin bindings.
//!
//! The ``from_dataframe`` constructors accept the long-format frames the
//! matching ``to_dataframe`` exits emit. Rows are pulled through
//! ``DataFrame.to_dict("records")`` so the binding only touches plain Python
//! dicts; all interpretation of the rows happens in the Rust ``add_*`` adders.

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Materialise a ``DataFrame`` (or any object with ``to_dict``) as row dicts.
pub(super) fn records<'py>(frame: &Bound<'py, PyAny>) -> PyResult<Vec<Bound<'py, PyDict>>> {
    let rows = frame.call_method1(pyo3::intern!(frame.py(), "to_dict"), ("records",))?;
    rows.try_iter()?
        .map(|row| row?.cast_into::<PyDict>().map_err(PyErr::from))
        .collect()
}

/// Whether a cell should be treated as missing (``None`` or a float ``NaN``).
fn is_missing(value: &Bound<'_, PyAny>) -> bool {
    value.is_none() || value.extract::<f64>().is_ok_and(f64::is_nan)
}

/// Read an optional text cell; ``None``/``NaN`` become ``None``.
///
/// Numeric cells (a bucket index that pandas inferred as ``int64``) are
/// rendered with ``str()`` so a frame round-tripped through ``pd.to_numeric``
/// still ingests.
pub(super) fn opt_str(row: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<String>> {
    match row.get_item(key)? {
        Some(value) if !is_missing(&value) => {
            if let Ok(text) = value.extract::<String>() {
                return Ok(Some(text));
            }
            if let Ok(number) = value.extract::<f64>() {
                if number.fract() == 0.0 {
                    return Ok(Some(format!("{}", number as i64)));
                }
            }
            Ok(Some(value.str()?.to_string()))
        }
        _ => Ok(None),
    }
}

/// Read a required text cell.
pub(super) fn req_str(row: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    opt_str(row, key)?.ok_or_else(|| {
        crate::errors::value_error(format!("from_dataframe: column '{key}' is missing or null"))
    })
}

/// Read a required float cell.
pub(super) fn req_f64(row: &Bound<'_, PyDict>, key: &str) -> PyResult<f64> {
    match row.get_item(key)? {
        Some(value) if !value.is_none() => value.extract::<f64>().map_err(|_| {
            crate::errors::value_error(format!(
                "from_dataframe: column '{key}' must be numeric, got {}",
                value
                    .get_type()
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ))
        }),
        _ => Err(crate::errors::value_error(format!(
            "from_dataframe: column '{key}' is missing or null"
        ))),
    }
}

/// Read a required bucket cell as a 1-based ``u8`` index.
pub(super) fn req_bucket(row: &Bound<'_, PyDict>, key: &str) -> PyResult<u8> {
    let text = req_str(row, key)?;
    text.trim().parse::<u8>().map_err(|_| {
        crate::errors::value_error(format!(
            "from_dataframe: column '{key}' must be a bucket index, got {text:?}"
        ))
    })
}

/// Read a required boolean cell.
pub(super) fn req_bool(row: &Bound<'_, PyDict>, key: &str) -> PyResult<bool> {
    match row.get_item(key)? {
        Some(value) if !is_missing(&value) => value.extract::<bool>().map_err(|_| {
            crate::errors::value_error(format!("from_dataframe: column '{key}' must be boolean"))
        }),
        _ => Err(crate::errors::value_error(format!(
            "from_dataframe: column '{key}' is missing or null"
        ))),
    }
}

/// Read a required date-like cell.
pub(super) fn req_date(row: &Bound<'_, PyDict>, key: &str) -> PyResult<time::Date> {
    match row.get_item(key)? {
        Some(value) if !is_missing(&value) => crate::bindings::date_utils::extract_date(&value),
        _ => Err(crate::errors::value_error(format!(
            "from_dataframe: column '{key}' is missing or null"
        ))),
    }
}

/// Split an ``"A/B"`` pair label into its two halves.
pub(super) fn split_pair(label: &str, what: &str) -> PyResult<(String, String)> {
    match label.split_once('/') {
        Some((a, b)) if !a.is_empty() && !b.is_empty() => Ok((a.to_string(), b.to_string())),
        _ => Err(crate::errors::value_error(format!(
            "from_dataframe: {what} must be a 'CCY1/CCY2' pair, got {label:?}"
        ))),
    }
}

/// Convert ``list[tuple[float, float]] | pandas.Series`` into ``(x, y)`` pairs.
///
/// A ``Series`` contributes its index as ``x`` and its values as ``y``; any
/// other iterable must yield two-element tuples.
pub(super) fn pairs_from_series_or_list(obj: &Bound<'_, PyAny>) -> PyResult<Vec<(f64, f64)>> {
    if obj.hasattr("items")? && obj.hasattr("index")? {
        let mut out = Vec::new();
        for item in obj.call_method0("items")?.try_iter()? {
            let (x, y): (f64, f64) = item?.extract()?;
            out.push((x, y));
        }
        return Ok(out);
    }
    obj.extract::<Vec<(f64, f64)>>()
}
