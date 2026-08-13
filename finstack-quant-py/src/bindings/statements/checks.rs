//! Python wrappers for the financial statement checks framework.

use crate::bindings::pandas_utils::serde_rows_to_dataframe_with_schema;
use crate::bindings::pandas_utils::ColumnSchema;
use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Columns emitted by [`PyCheckReport::to_dataframe`].
const CHECK_RESULT_COLUMNS: [ColumnSchema<'static>; 5] = [
    ("check_id", "str"),
    ("check_name", "str"),
    ("category", "str"),
    ("passed", "bool"),
    ("finding_count", "int64"),
];

/// Columns emitted by [`PyCheckReport::to_findings_dataframe`].
const CHECK_FINDING_COLUMNS: [ColumnSchema<'static>; 11] = [
    ("check_id", "str"),
    ("check_name", "str"),
    ("category", "str"),
    ("severity", "str"),
    ("message", "str"),
    ("period", "str"),
    ("nodes", "str"),
    ("materiality_absolute", "float64"),
    ("materiality_relative_pct", "float64"),
    ("materiality_reference_value", "float64"),
    ("materiality_reference_label", "str"),
];

// CheckSuiteSpec

/// A serializable suite specification describing which checks to run.
#[pyclass(
    name = "CheckSuiteSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCheckSuiteSpec {
    pub(super) inner: finstack_quant_statements::checks::CheckSuiteSpec,
}

#[pymethods]
impl PyCheckSuiteSpec {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a suite spec from a JSON string.
    ///
    /// Unknown fields are rejected, so a team-wide check policy that drifts
    /// from the schema fails at load rather than silently running fewer
    /// checks than intended. Formula syntax and node references are validated
    /// later, when the suite runs against a model.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_statements::checks::CheckSuiteSpec =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this suite spec to compact JSON.
    ///
    /// The output is the canonical policy format — check it into version
    /// control to pin a team's validation rules and tolerances.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Suite name, used for display and logging.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Number of built-in checks the spec will materialize.
    ///
    /// Built-ins are the crate-provided accounting-identity, reconciliation,
    /// and data-quality checks, selected by name in the spec.
    #[getter]
    fn builtin_check_count(&self) -> usize {
        self.inner.builtin_checks.len()
    }

    /// Number of user-defined formula checks the spec will materialize.
    ///
    /// Formula checks are DSL expressions evaluated per period; their syntax
    /// is validated when the suite runs, not when the spec is loaded.
    #[getter]
    fn formula_check_count(&self) -> usize {
        self.inner.formula_checks.len()
    }

    /// Return the debug representation with the name and check counts.
    fn __repr__(&self) -> String {
        format!(
            "CheckSuiteSpec(name={:?}, builtins={}, formulas={})",
            self.inner.name,
            self.inner.builtin_checks.len(),
            self.inner.formula_checks.len(),
        )
    }
}

// CheckReport

/// Validation check report aggregating results and summary statistics.
#[pyclass(
    name = "CheckReport",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCheckReport {
    pub(super) inner: finstack_quant_statements::checks::CheckReport,
}

#[pymethods]
impl PyCheckReport {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a check report from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_statements::checks::CheckReport =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this report to compact JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Whether the whole report passed: no error-severity finding was
    /// retained by any check.
    ///
    /// Warnings and info findings do not fail a report. A suite's
    /// materiality threshold never suppresses error findings, so it cannot
    /// flip this to ``True``. Delegates to the canonical Rust
    /// ``CheckReport::has_errors``.
    #[getter]
    fn passed(&self) -> bool {
        !self.inner.has_errors()
    }

    /// Number of checks that ran, one per row of :meth:`to_dataframe`.
    ///
    /// Reads the canonical ``CheckSummary.total_checks`` counter.
    #[getter]
    fn total_checks(&self) -> usize {
        self.inner.summary.total_checks
    }

    /// Whether the report contains at least one error-severity finding.
    ///
    /// Delegates to the canonical Rust ``CheckReport::has_errors``.
    #[pyo3(text_signature = "($self)")]
    fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    /// Whether the report contains at least one warning-severity finding.
    ///
    /// Delegates to the canonical Rust ``CheckReport::has_warnings``.
    #[pyo3(text_signature = "($self)")]
    fn has_warnings(&self) -> bool {
        self.inner.has_warnings()
    }

    /// Return all retained findings of one severity as serde dicts.
    ///
    /// Delegates to the canonical Rust ``CheckReport::findings_by_severity``.
    ///
    /// Parameters
    /// ----------
    /// severity : str
    ///     One of ``"info"``, ``"warning"``, ``"error"`` (the canonical
    ///     snake_case severity discriminants).
    ///
    /// Returns
    /// -------
    /// list[dict]
    ///     One ``CheckFinding`` serde dict per matching retained finding.
    #[pyo3(text_signature = "($self, severity)")]
    fn findings_by_severity<'py>(
        &self,
        py: Python<'py>,
        severity: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let severity: finstack_quant_statements::checks::Severity = serde_json::from_value(
            serde_json::Value::String(severity.to_string()),
        )
        .map_err(|e| {
            crate::errors::value_error(format!(
                "invalid severity {severity:?}: {e}; expected info, warning, or error"
            ))
        })?;
        let findings = self.inner.findings_by_severity(severity);
        crate::bindings::pandas_utils::serde_to_py(py, &findings)
    }

    /// Number of retained findings across all checks.
    ///
    /// Counts what survived the suite's ``min_severity`` and
    /// ``materiality_threshold`` reporting filters, not every raw diagnostic
    /// the checks produced — it matches the row count of
    /// :meth:`to_findings_dataframe`.
    #[getter]
    fn total_findings(&self) -> usize {
        self.inner.summary.errors + self.inner.summary.warnings + self.inner.summary.infos
    }

    /// Number of retained error-severity findings across all checks.
    #[getter]
    fn total_errors(&self) -> usize {
        self.inner.summary.errors
    }

    /// Number of retained warning-severity findings across all checks.
    #[getter]
    fn total_warnings(&self) -> usize {
        self.inner.summary.warnings
    }

    /// Export one row per executed check as a pandas ``DataFrame``.
    ///
    /// Columns: ``check_id`` (stable identifier), ``check_name``
    /// (human-readable name), ``category`` (snake_case group, one of
    /// ``accounting_identity``, ``cross_statement_reconciliation``,
    /// ``internal_consistency``, ``credit_reasonableness``,
    /// ``data_quality``), ``passed`` (``True`` when the check retained no
    /// error-severity finding), ``finding_count`` (retained findings for this
    /// check, after the suite's reporting filters).
    ///
    /// One row per check result, so the row count equals
    /// :attr:`total_checks`. A report with no checks still yields the
    /// documented columns with zero rows. Per-finding detail — severity,
    /// message, and the observed-vs-reference materiality — lives in
    /// :meth:`to_findings_dataframe`.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows: Vec<serde_json::Value> = self
            .inner
            .results
            .iter()
            .map(|result| {
                serde_json::json!({
                    "check_id": result.check_id,
                    "check_name": result.check_name,
                    "category": result.category,
                    "passed": result.passed,
                    "finding_count": result.findings.len(),
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &CHECK_RESULT_COLUMNS)
    }

    /// Export one row per retained finding as a pandas ``DataFrame``.
    ///
    /// Columns: ``check_id``, ``check_name``, ``category`` (as in
    /// :meth:`to_dataframe`), ``severity`` (``info``, ``warning`` or
    /// ``error``), ``message`` (human-readable description), ``period``
    /// (period identifier string such as ``"2025Q1"``, ``None`` when the
    /// finding is not period-specific), ``nodes`` (comma-joined node
    /// identifiers involved, empty string when none),
    /// ``materiality_absolute`` (size of the discrepancy in the compared
    /// nodes' own units — currency amounts for monetary nodes),
    /// ``materiality_relative_pct`` (that discrepancy as a **percentage** of
    /// the reference value, i.e. already multiplied by 100, unlike the
    /// decimal-fraction rates used elsewhere in this module),
    /// ``materiality_reference_value`` (the denominator used) and
    /// ``materiality_reference_label`` (what that denominator is, e.g.
    /// ``total_assets``).
    ///
    /// The four materiality columns are ``None`` for findings that carry no
    /// materiality context. The row count equals :attr:`total_findings`; a
    /// report with no findings still yields the documented columns with zero
    /// rows.
    #[pyo3(text_signature = "($self)")]
    fn to_findings_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows: Vec<serde_json::Value> = self
            .inner
            .results
            .iter()
            .flat_map(|result| {
                result.findings.iter().map(move |finding| {
                    let materiality = finding.materiality.as_ref();
                    serde_json::json!({
                        "check_id": finding.check_id,
                        "check_name": result.check_name,
                        "category": result.category,
                        "severity": finding.severity,
                        "message": finding.message,
                        "period": finding.period.map(|p| p.to_string()),
                        "nodes": finding
                            .nodes
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                        "materiality_absolute": materiality.map(|m| m.absolute),
                        "materiality_relative_pct": materiality.map(|m| m.relative_pct),
                        "materiality_reference_value": materiality.map(|m| m.reference_value),
                        "materiality_reference_label": materiality
                            .map(|m| m.reference_label.clone()),
                    })
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &CHECK_FINDING_COLUMNS)
    }

    /// Return the debug representation with check, error and warning counts.
    fn __repr__(&self) -> String {
        format!(
            "CheckReport(checks={}, passed={}, errors={}, warnings={})",
            self.inner.summary.total_checks,
            !self.inner.has_errors(),
            self.inner.summary.errors,
            self.inner.summary.warnings,
        )
    }

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
    }
}

/// Register check types on the parent module.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCheckSuiteSpec>()?;
    m.add_class::<PyCheckReport>()?;
    Ok(())
}
