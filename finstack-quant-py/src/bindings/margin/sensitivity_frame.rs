//! Shared long-format DataFrame shape for regulatory sensitivity containers.
//!
//! `FrtbSensitivities` and `SimmSensitivities` both store their inputs as many
//! `HashMap<key tuple, f64>` buckets. Exporting one column per bucket would
//! give a different schema for every portfolio, so both export the same
//! six-column long format instead: `risk_class`, `bucket`, `tenor`, `issuer`,
//! `kind`, `amount`.
//!
//! Rows are sorted by their key columns before the frame is built, because the
//! underlying `HashMap` iteration order is not stable across runs.

use crate::bindings::pandas_utils::serde_rows_to_dataframe_with_schema;
use crate::bindings::pandas_utils::ColumnSchema;
use pyo3::prelude::*;

/// Column schema shared by every sensitivity long-format export.
pub(super) const SENSITIVITY_COLUMNS: [ColumnSchema<'static>; 6] = [
    ("risk_class", "str"),
    ("bucket", "str"),
    ("tenor", "str"),
    ("issuer", "str"),
    ("kind", "str"),
    ("amount", "float64"),
];

/// One long-format sensitivity row.
struct SensitivityRow {
    risk_class: String,
    bucket: Option<String>,
    tenor: Option<String>,
    issuer: Option<String>,
    kind: &'static str,
    amount: f64,
}

/// Accumulator that emits sensitivity rows in a deterministic order.
#[derive(Default)]
pub(super) struct SensitivityRows {
    rows: Vec<SensitivityRow>,
}

impl SensitivityRows {
    /// Record one sensitivity value.
    ///
    /// `bucket`, `tenor` and `issuer` are `None` for risk classes that do not
    /// carry that axis; they surface as missing values in pandas rather than
    /// as empty strings.
    pub(super) fn push(
        &mut self,
        risk_class: impl Into<String>,
        kind: &'static str,
        issuer: Option<String>,
        bucket: Option<String>,
        tenor: Option<String>,
        amount: f64,
    ) {
        self.rows.push(SensitivityRow {
            risk_class: risk_class.into(),
            bucket,
            tenor,
            issuer,
            kind,
            amount,
        });
    }

    /// Record both halves of a curvature `(cvr_up, cvr_down)` pair.
    pub(super) fn push_curvature(
        &mut self,
        risk_class: &str,
        issuer: Option<String>,
        bucket: Option<String>,
        (cvr_up, cvr_down): (f64, f64),
    ) {
        self.push(
            risk_class,
            "curvature_up",
            issuer.clone(),
            bucket.clone(),
            None,
            cvr_up,
        );
        self.push(risk_class, "curvature_down", issuer, bucket, None, cvr_down);
    }

    /// Build the pandas DataFrame.
    ///
    /// Rows are sorted by `(risk_class, kind, issuer, bucket, tenor)` so two
    /// runs over the same sensitivities produce identical frames. An empty
    /// container still yields all six columns.
    pub(super) fn into_dataframe<'py>(mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.rows.sort_by(|left, right| {
            (
                &left.risk_class,
                left.kind,
                &left.issuer,
                &left.bucket,
                &left.tenor,
            )
                .cmp(&(
                    &right.risk_class,
                    right.kind,
                    &right.issuer,
                    &right.bucket,
                    &right.tenor,
                ))
        });

        let rows: Vec<serde_json::Value> = self
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "risk_class": row.risk_class,
                    "bucket": row.bucket,
                    "tenor": row.tenor,
                    "issuer": row.issuer,
                    "kind": row.kind,
                    "amount": row.amount,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &SENSITIVITY_COLUMNS)
    }
}
