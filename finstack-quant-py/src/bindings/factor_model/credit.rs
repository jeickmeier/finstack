//! Python bindings for the credit factor hierarchy.
//!
//! Exposes [`PyCreditFactorModel`], [`PyCreditCalibrator`], the free functions
//! [`decompose_levels`] and [`decompose_period`], and
//! [`PyFactorCovarianceForecast`] which wraps the vol-forecast engine from
//! `finstack-quant-portfolio`.

use crate::bindings::pandas_utils::labeled_values_to_series;
use crate::bindings::pandas_utils::ColumnSchema;
use crate::bindings::pandas_utils::{serde_rows_to_dataframe_with_schema, serde_to_py};
use crate::errors::{core_to_py, display_to_py};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Column schema of `PyLevelsAtDate::to_dataframe`, kept so a level-free
/// snapshot still exports the documented columns.
const LEVEL_VALUE_COLUMNS: &[ColumnSchema<'static>] = &[
    ("date", "str"),
    ("level_index", "int64"),
    ("dimension", "str"),
    ("bucket", "str"),
    ("value", "float64"),
];

/// Column schema of `PyPeriodDecomposition::to_level_dataframe`, kept so a
/// level-free decomposition still exports the documented columns.
const LEVEL_DELTA_COLUMNS: &[ColumnSchema<'static>] = &[
    ("from_date", "str"),
    ("to_date", "str"),
    ("level_index", "int64"),
    ("dimension", "str"),
    ("bucket", "str"),
    ("delta", "float64"),
];

/// Column schema of `PyPeriodDecomposition::to_adder_dataframe`, kept so a
/// decomposition with no shared issuers still exports the documented columns.
const ADDER_DELTA_COLUMNS: &[ColumnSchema<'static>] = &[
    ("from_date", "str"),
    ("to_date", "str"),
    ("issuer_id", "str"),
    ("d_adder", "float64"),
];

/// Display label for a hierarchy dimension, matching
/// `PyCreditFactorModel::level_names` so the two line up on a join.
fn dimension_label(
    dim: &finstack_quant_factor_model::credit::hierarchy::HierarchyDimension,
) -> String {
    use finstack_quant_factor_model::credit::hierarchy::HierarchyDimension;
    match dim {
        HierarchyDimension::Rating => "Rating".to_owned(),
        HierarchyDimension::Region => "Region".to_owned(),
        HierarchyDimension::Sector => "Sector".to_owned(),
        HierarchyDimension::Custom(name) => name.clone(),
        _ => "Unknown".to_owned(),
    }
}

/// Calibrated credit factor hierarchy artifact.
///
/// Produced by :class:`CreditCalibrator` or loaded from JSON via
/// :meth:`from_json`.  All fields are read-only; mutations require
/// re-calibrating.
///
/// The canonical round-trip is::
///
///     model = CreditFactorModel.from_json(json_str)
///     json_out = model.to_json()
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, CreditFactorModel
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> calibrated = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> CreditFactorModel.from_json(calibrated.to_json()).schema
///     'finstack_quant.credit_factor_model/1'
#[pyclass(
    name = "CreditFactorModel",
    module = "finstack_quant.factor_model.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyCreditFactorModel {
    pub(crate) inner: finstack_quant_factor_model::credit::hierarchy::CreditFactorModel,
}

impl PyCreditFactorModel {
    pub(crate) fn from_inner(
        inner: finstack_quant_factor_model::credit::hierarchy::CreditFactorModel,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditFactorModel {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a :class:`CreditFactorModel` from JSON.
    ///
    /// Validates the required ``schema`` marker and all structural constraints.
    ///
    /// Args:
    ///     json: JSON string produced by :meth:`to_json` or the offline calibrator.
    ///
    /// Returns:
    ///     Parsed :class:`CreditFactorModel` instance.
    ///
    /// Raises:
    ///     ValueError: If the JSON is malformed or fails validation.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_factor_model::credit::hierarchy::CreditFactorModel =
            serde_json::from_str(json).map_err(display_to_py)?;
        inner.validate().map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this model to compact JSON.
    ///
    /// Returns:
    ///     JSON string suitable for storage or transmission.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Namespaced schema marker (``"finstack_quant.credit_factor_model/1"``).
    #[getter]
    fn schema(&self) -> &'static str {
        self.inner.schema.as_str()
    }

    /// Calibration anchor date (ISO 8601 string).
    #[getter]
    fn as_of(&self) -> String {
        self.inner.as_of.to_string()
    }

    /// Number of hierarchy levels (broadest → narrowest).
    #[getter]
    fn n_levels(&self) -> usize {
        self.inner.hierarchy.levels.len()
    }

    /// Number of issuer beta rows in the artifact.
    #[getter]
    fn n_issuers(&self) -> usize {
        self.inner.issuer_betas.len()
    }

    /// Number of factors in the model configuration.
    #[getter]
    fn n_factors(&self) -> usize {
        self.inner.config.factors.len()
    }

    /// Hierarchy level names as a list of strings.
    ///
    /// Returns:
    ///     List of dimension names (e.g. ``["Rating", "Region", "Sector"]``).
    fn level_names(&self) -> Vec<String> {
        self.inner
            .hierarchy
            .levels
            .iter()
            .map(dimension_label)
            .collect()
    }

    /// Issuer IDs present in the artifact.
    ///
    /// Returns:
    ///     List of issuer ID strings.
    fn issuer_ids(&self) -> Vec<String> {
        self.inner
            .issuer_betas
            .iter()
            .map(|row| row.issuer_id.as_str().to_owned())
            .collect()
    }

    /// Factor IDs in the model configuration.
    ///
    /// Returns:
    ///     List of factor ID strings.
    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .config
            .factors
            .iter()
            .map(|f| f.id.to_string())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditFactorModel(as_of={:?}, n_levels={}, n_issuers={}, n_factors={})",
            self.inner.as_of,
            self.inner.hierarchy.levels.len(),
            self.inner.issuer_betas.len(),
            self.inner.config.factors.len(),
        )
    }
}

/// Deterministic calibrator that produces a :class:`CreditFactorModel`.
///
/// Configuration and inputs are passed as JSON strings so that Python callers
/// can work with plain dicts (serialized via ``json.dumps``) without needing
/// typed wrappers for every sub-field.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> CreditCalibrator(config_json).calibrate(inputs_json).n_issuers
///     1
#[pyclass(
    name = "CreditCalibrator",
    module = "finstack_quant.factor_model.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyCreditCalibrator {
    inner: finstack_quant_factor_model::credit::calibration::CreditCalibrator,
}

#[pymethods]
impl PyCreditCalibrator {
    /// Construct a calibrator from a JSON-serialized ``CreditCalibrationConfig``.
    ///
    /// Args:
    ///     config_json: JSON string of a ``CreditCalibrationConfig``.
    ///
    /// Raises:
    ///     ValueError: If ``config_json`` is not a valid ``CreditCalibrationConfig``.
    #[new]
    fn new(config_json: &str) -> PyResult<Self> {
        let config: finstack_quant_factor_model::credit::calibration::CreditCalibrationConfig =
            serde_json::from_str(config_json).map_err(display_to_py)?;
        Ok(Self {
            inner: finstack_quant_factor_model::credit::calibration::CreditCalibrator::new(config),
        })
    }

    /// Run the full calibration pipeline and return a :class:`CreditFactorModel`.
    ///
    /// Args:
    ///     inputs_json: JSON string of a ``CreditCalibrationInputs`` object
    ///         containing the history panel, issuer tags, generic factor
    ///         series, and anchor date.
    ///
    /// Returns:
    ///     Calibrated :class:`CreditFactorModel` artifact.
    ///
    /// Raises:
    ///     ValueError: If inputs are structurally invalid or calibration fails.
    fn calibrate(&self, py: Python<'_>, inputs_json: &str) -> PyResult<PyCreditFactorModel> {
        let inputs: finstack_quant_factor_model::credit::calibration::CreditCalibrationInputs =
            serde_json::from_str(inputs_json).map_err(display_to_py)?;
        let model = py
            .detach(|| self.inner.calibrate(inputs))
            .map_err(core_to_py)?;
        Ok(PyCreditFactorModel::from_inner(model))
    }

    fn __repr__(&self) -> String {
        "CreditCalibrator(...)".to_owned()
    }
}

/// Snapshot of all hierarchy-level factor values at a single date.
///
/// Produced by :func:`decompose_levels`.  Carry this into
/// :func:`decompose_period` to compute period-over-period changes.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, decompose_levels
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> levels = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
///     >>> (levels.date, levels.generic, levels.adder())
///     ('2024-03-01', 100.0, {'A': 5.0})
#[pyclass(
    name = "LevelsAtDate",
    module = "finstack_quant.factor_model.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyLevelsAtDate {
    inner: finstack_quant_factor_model::credit::decomposition::LevelsAtDate,
}

impl PyLevelsAtDate {
    fn from_inner(inner: finstack_quant_factor_model::credit::decomposition::LevelsAtDate) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLevelsAtDate {
    /// Observation date (ISO 8601 string).
    #[getter]
    fn date(&self) -> String {
        self.inner.date.to_string()
    }

    /// Generic (PC) factor value at this date.
    #[getter]
    fn generic(&self) -> f64 {
        self.inner.generic
    }

    /// Number of hierarchy levels.
    #[getter]
    fn n_levels(&self) -> usize {
        self.inner.by_level.len()
    }

    /// Bucket values for a given level index as a dict ``{bucket_path: value}``.
    ///
    /// Args:
    ///     level_index: Zero-based hierarchy level index.
    ///
    /// Returns:
    ///     Dict mapping bucket path string to factor value.
    ///
    /// Raises:
    ///     ValueError: If ``level_index`` is out of range.
    fn level_values<'py>(
        &self,
        py: Python<'py>,
        level_index: usize,
    ) -> PyResult<Bound<'py, PyDict>> {
        let lev = self.inner.by_level.get(level_index).ok_or_else(|| {
            crate::errors::value_error(format!(
                "level_index {} out of range (n_levels={})",
                level_index,
                self.inner.by_level.len()
            ))
        })?;
        let d = PyDict::new(py);
        for (k, v) in &lev.values {
            d.set_item(k, v)?;
        }
        Ok(d)
    }

    /// Per-issuer residual adder after peeling all levels, as a dict.
    ///
    /// Returns:
    ///     Dict mapping issuer ID to adder value.
    fn adder<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (issuer, val) in &self.inner.adder {
            d.set_item(issuer.as_str(), val)?;
        }
        Ok(d)
    }

    /// Export the per-level bucket values as a pandas ``DataFrame``.
    ///
    /// Columns: ``date``, ``level_index``, ``dimension``, ``bucket``,
    /// ``value``.
    ///
    /// One row per (level, bucket) pair — the long format the level values
    /// naturally take, since each level has its own bucket set. ``date``
    /// repeats on every row as an ISO string so a row survives ``pd.concat``
    /// across dates. Rows are ordered by ``level_index``, then by ``bucket``
    /// — the values are a ``BTreeMap``, so bucket order is the sorted key
    /// order and repeated exports are identical.
    ///
    /// A snapshot from a hierarchy with no levels yields a zero-row frame that
    /// still carries the columns above. The scalar ``generic`` factor and the
    /// per-issuer residuals are not levels; read them from the ``generic``
    /// getter and ``to_series`` respectively.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let date = self.inner.date.to_string();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for level in &self.inner.by_level {
            let dimension = dimension_label(&level.dimension);
            for (bucket, value) in &level.values {
                rows.push(serde_json::json!({
                    "date": date,
                    "level_index": level.level_index,
                    "dimension": dimension,
                    "bucket": bucket,
                    "value": value,
                }));
            }
        }
        serde_rows_to_dataframe_with_schema(py, &rows, LEVEL_VALUE_COLUMNS)
    }

    /// Export the per-issuer residual adders as a pandas ``Series``.
    ///
    /// Index is the issuer ID, values are the residual after peeling the
    /// generic factor and every hierarchy level; the Series is named
    /// ``adder``. Issuers are in sorted order — the adders are a ``BTreeMap``
    /// — so repeated exports and ``pd.concat([...], axis=1)`` across dates
    /// align on the index.
    #[pyo3(text_signature = "($self)")]
    fn to_series<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let labels: Vec<String> = self
            .inner
            .adder
            .keys()
            .map(|issuer| issuer.as_str().to_owned())
            .collect();
        let values: Vec<f64> = self.inner.adder.values().copied().collect();
        labeled_values_to_series(py, &labels, values, "adder")
    }

    fn __repr__(&self) -> String {
        format!(
            "LevelsAtDate(date={:?}, generic={:.4}, n_levels={}, n_issuers={})",
            self.inner.date,
            self.inner.generic,
            self.inner.by_level.len(),
            self.inner.adder.len(),
        )
    }
}

/// Component-wise difference between two :class:`LevelsAtDate` snapshots.
///
/// Produced by :func:`decompose_period`.  Satisfies the linear
/// reconciliation invariant
///
/// .. code-block:: text
///
///     ΔS_i ≡ β_i^PC · Δgeneric
///            + Σ_k β_i^level_k · ΔL_level_k(g_i^k)
///            + Δadder_i
///
/// for every issuer present in both snapshots.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, decompose_levels, decompose_period
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> start = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
///     >>> end = decompose_levels(model, '{"A": 0.01065}', 0.01015, "2024-03-02")
///     >>> period = decompose_period(start, end)
///     >>> (period.from_date, period.to_date, period.d_generic)
///     ('2024-03-01', '2024-03-02', 1.5)
#[pyclass(
    name = "PeriodDecomposition",
    module = "finstack_quant.factor_model.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyPeriodDecomposition {
    inner: finstack_quant_factor_model::credit::decomposition::PeriodDecomposition,
}

impl PyPeriodDecomposition {
    fn from_inner(
        inner: finstack_quant_factor_model::credit::decomposition::PeriodDecomposition,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPeriodDecomposition {
    /// Earlier snapshot date (ISO 8601).
    #[getter(from_date)]
    fn get_from_date(&self) -> String {
        self.inner.from.to_string()
    }

    /// Later snapshot date (ISO 8601).
    #[getter]
    fn to_date(&self) -> String {
        self.inner.to.to_string()
    }

    /// Change in the generic (PC) factor value.
    #[getter]
    fn d_generic(&self) -> f64 {
        self.inner.d_generic
    }

    /// Number of hierarchy levels.
    #[getter]
    fn n_levels(&self) -> usize {
        self.inner.by_level.len()
    }

    /// Bucket value deltas for a given level index as a dict.
    ///
    /// Args:
    ///     level_index: Zero-based hierarchy level index.
    ///
    /// Returns:
    ///     Dict mapping bucket path string to delta value.
    ///
    /// Raises:
    ///     ValueError: If ``level_index`` is out of range.
    fn level_deltas<'py>(
        &self,
        py: Python<'py>,
        level_index: usize,
    ) -> PyResult<Bound<'py, PyDict>> {
        let lev = self.inner.by_level.get(level_index).ok_or_else(|| {
            crate::errors::value_error(format!(
                "level_index {} out of range (n_levels={})",
                level_index,
                self.inner.by_level.len()
            ))
        })?;
        let d = PyDict::new(py);
        for (k, v) in &lev.deltas {
            d.set_item(k, v)?;
        }
        Ok(d)
    }

    /// Per-issuer adder deltas as a dict.
    ///
    /// Returns:
    ///     Dict mapping issuer ID to adder change.
    fn d_adder<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (issuer, val) in &self.inner.d_adder {
            d.set_item(issuer.as_str(), val)?;
        }
        Ok(d)
    }

    /// Export the per-level bucket deltas as a pandas ``DataFrame``.
    ///
    /// Columns: ``from_date``, ``to_date``, ``level_index``, ``dimension``,
    /// ``bucket``, ``delta``.
    ///
    /// This is the default export and the same table as
    /// ``to_level_dataframe`` — both call one implementation, so the two
    /// cannot drift apart. The per-issuer adder deltas are a separate,
    /// differently-keyed table; see ``to_adder_dataframe``. The scalar
    /// ``d_generic`` is metadata and is not repeated per row.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.to_level_dataframe(py)
    }

    /// Export the per-level bucket deltas as a pandas ``DataFrame``.
    ///
    /// Columns: ``from_date``, ``to_date``, ``level_index``, ``dimension``,
    /// ``bucket``, ``delta``. Identical to ``to_dataframe``.
    ///
    /// One row per (level, bucket) pair — the long format the level deltas
    /// naturally take, since each level has its own bucket set. The two dates
    /// repeat on every row as ISO strings so a row survives ``pd.concat``
    /// across periods. Rows are ordered by ``level_index``, then by
    /// ``bucket`` — the deltas are a ``BTreeMap``, so bucket order is the
    /// sorted key order and repeated exports are identical.
    ///
    /// A decomposition with no hierarchy levels yields a zero-row frame that
    /// still carries the columns above.
    fn to_level_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // `PeriodDecomposition` derives Serialize, but its wire form nests
        // `by_level` as a list of maps, so the long rows are built by hand.
        let from_date = self.inner.from.to_string();
        let to_date = self.inner.to.to_string();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for level in &self.inner.by_level {
            let dimension = dimension_label(&level.dimension);
            for (bucket, delta) in &level.deltas {
                rows.push(serde_json::json!({
                    "from_date": from_date,
                    "to_date": to_date,
                    "level_index": level.level_index,
                    "dimension": dimension,
                    "bucket": bucket,
                    "delta": delta,
                }));
            }
        }
        serde_rows_to_dataframe_with_schema(py, &rows, LEVEL_DELTA_COLUMNS)
    }

    /// Export the per-issuer adder deltas as a pandas ``DataFrame``.
    ///
    /// Columns: ``from_date``, ``to_date``, ``issuer_id``, ``d_adder``.
    ///
    /// One row per issuer. The two dates repeat on every row as ISO strings so
    /// a row survives ``pd.concat`` across periods. Rows are ordered by
    /// ``issuer_id`` — the adders are a ``BTreeMap``, so this is the sorted
    /// key order and repeated exports are identical.
    ///
    /// A decomposition sharing no issuers between snapshots yields a zero-row
    /// frame that still carries the columns above.
    fn to_adder_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let from_date = self.inner.from.to_string();
        let to_date = self.inner.to.to_string();
        let rows: Vec<serde_json::Value> = self
            .inner
            .d_adder
            .iter()
            .map(|(issuer, delta)| {
                serde_json::json!({
                    "from_date": from_date,
                    "to_date": to_date,
                    "issuer_id": issuer.as_str(),
                    "d_adder": delta,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, ADDER_DELTA_COLUMNS)
    }

    fn __repr__(&self) -> String {
        format!(
            "PeriodDecomposition(from={:?}, to={:?}, d_generic={:.4}, n_levels={})",
            self.inner.from,
            self.inner.to,
            self.inner.d_generic,
            self.inner.by_level.len(),
        )
    }
}

// decompose_levels  (free function)

/// Decompose observed issuer spreads at a point in time into per-level factor
/// values and per-issuer residual adders.
///
/// Args:
///     model: Calibrated :class:`CreditFactorModel` artifact.
///     observed_spreads_json: JSON string (e.g. via ``json.dumps``) of a mapping
///         from issuer ID string to observed spread (float).
///     observed_generic: Generic (PC) factor value at ``as_of``.
///     as_of: Valuation date, either a date-like object (``datetime.date``,
///         ``pandas.Timestamp``) or an ISO 8601 string.
///     runtime_tags_json: Optional JSON string of
///         ``{issuer_id: {dim_key: tag_value}}`` for issuers not present in
///         the model.
///
/// Returns:
///     :class:`LevelsAtDate` snapshot with generic value, per-level bucket values,
///     and per-issuer residual adders.
///
/// Raises:
///     ValueError: If an issuer has no model row and no ``runtime_tags`` entry,
///         or if an issuer is missing a required hierarchy tag.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, decompose_levels
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> decompose_levels(model, '{"A": 0.0125}', 0.0120, "2025-06-30").generic
///     120.0
#[pyfunction]
#[pyo3(signature = (model, observed_spreads_json, observed_generic, as_of, runtime_tags_json=None))]
fn decompose_levels(
    model: &PyCreditFactorModel,
    observed_spreads_json: &str,
    observed_generic: f64,
    as_of: &Bound<'_, PyAny>,
    runtime_tags_json: Option<&str>,
) -> PyResult<PyLevelsAtDate> {
    let observed_spreads: std::collections::BTreeMap<finstack_quant_core::types::IssuerId, f64> =
        serde_json::from_str(observed_spreads_json).map_err(display_to_py)?;

    let date = crate::bindings::date_utils::extract_date(as_of)?;

    let runtime_tags: Option<
        std::collections::BTreeMap<
            finstack_quant_core::types::IssuerId,
            finstack_quant_factor_model::credit::hierarchy::IssuerTags,
        >,
    > = match runtime_tags_json {
        Some(json) => Some(serde_json::from_str(json).map_err(display_to_py)?),
        None => None,
    };

    let result = finstack_quant_factor_model::credit::decomposition::decompose_levels(
        &model.inner,
        &observed_spreads,
        observed_generic,
        date,
        runtime_tags.as_ref(),
    )
    .map_err(display_to_py)?;

    Ok(PyLevelsAtDate::from_inner(result))
}

// decompose_period  (free function)

/// Difference two :class:`LevelsAtDate` snapshots component-wise.
///
/// Output buckets and issuers are restricted to those present in **both**
/// snapshots so the linear reconciliation invariant on ``ΔS_i`` holds for
/// every entry.
///
/// Args:
///     from_levels: Earlier :class:`LevelsAtDate` snapshot.
///     to_levels: Later :class:`LevelsAtDate` snapshot.
///
/// Returns:
///     :class:`PeriodDecomposition` with ``d_generic``, per-level bucket deltas,
///     and per-issuer adder deltas.
///
/// Raises:
///     ValueError: If ``from_levels.date > to_levels.date`` or the two
///         snapshots disagree on hierarchy depth.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, decompose_levels, decompose_period
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> start = decompose_levels(model, '{"A": 0.0105}', 0.010, "2024-03-01")
///     >>> end = decompose_levels(model, '{"A": 0.01065}', 0.01015, "2024-03-02")
///     >>> decompose_period(start, end).d_generic
///     1.5
#[pyfunction]
fn decompose_period(
    from_levels: &PyLevelsAtDate,
    to_levels: &PyLevelsAtDate,
) -> PyResult<PyPeriodDecomposition> {
    let result = finstack_quant_factor_model::credit::decomposition::decompose_period(
        &from_levels.inner,
        &to_levels.inner,
    )
    .map_err(display_to_py)?;
    Ok(PyPeriodDecomposition::from_inner(result))
}

/// Validated factor covariance matrix with deterministic row-major storage.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import FactorCovarianceMatrix
///     >>> matrix = FactorCovarianceMatrix.from_json('{"factor_ids":["credit::generic"],"n":1,"data":[0.04]}')
///     >>> matrix.variance("credit::generic")
///     0.04
#[pyclass(
    name = "FactorCovarianceMatrix",
    module = "finstack_quant.factor_model.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorCovarianceMatrix {
    pub(crate) inner: finstack_quant_factor_model::FactorCovarianceMatrix,
}

impl PyFactorCovarianceMatrix {
    fn from_inner(inner: finstack_quant_factor_model::FactorCovarianceMatrix) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorCovarianceMatrix {
    /// Deserialize and validate a covariance matrix from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this covariance matrix to canonical JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Number of factors represented by the matrix.
    #[getter]
    fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    /// Ordered factor identifiers corresponding to rows and columns.
    #[getter]
    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Row-major covariance data with `n_factors * n_factors` entries.
    #[getter]
    fn data(&self) -> Vec<f64> {
        self.inner.as_slice().to_vec()
    }

    /// Variance for `factor_id`, or zero when the factor is unknown.
    fn variance(&self, factor_id: &str) -> f64 {
        self.inner
            .variance(&finstack_quant_factor_model::FactorId::new(factor_id))
    }

    /// Covariance between two factors, or zero when either factor is unknown.
    fn covariance(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner.covariance(
            &finstack_quant_factor_model::FactorId::new(lhs),
            &finstack_quant_factor_model::FactorId::new(rhs),
        )
    }

    /// Correlation between two factors, or zero for unknown/zero-variance factors.
    fn correlation(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner.correlation(
            &finstack_quant_factor_model::FactorId::new(lhs),
            &finstack_quant_factor_model::FactorId::new(rhs),
        )
    }

    /// Support pickle through the canonical JSON representation.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    fn __repr__(&self) -> String {
        format!(
            "FactorCovarianceMatrix(n_factors={})",
            self.inner.n_factors()
        )
    }
}

/// Portfolio factor-model configuration assembled at a forecast horizon.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import FactorModelConfig
///     >>> config = FactorModelConfig.from_json('{"factors":[],"covariance":{"factor_ids":[],"n":0,"data":[]},"matching":{"mapping_table":[]},"pricing_mode":"delta_based","risk_measure":"variance"}')
///     >>> config.n_factors
///     0
#[pyclass(
    name = "FactorModelConfig",
    module = "finstack_quant.factor_model.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorModelConfig {
    pub(crate) inner: finstack_quant_factor_model::FactorModelConfig,
}

impl PyFactorModelConfig {
    fn from_inner(inner: finstack_quant_factor_model::FactorModelConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorModelConfig {
    /// Deserialize and validate a factor-model configuration from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_factor_model::FactorModelConfig =
            serde_json::from_str(json).map_err(display_to_py)?;
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this configuration to canonical JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Number of configured factors.
    #[getter]
    fn n_factors(&self) -> usize {
        self.inner.factors.len()
    }

    /// Ordered factor identifiers used by definitions and covariance axes.
    #[getter]
    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factors
            .iter()
            .map(|factor| factor.id.to_string())
            .collect()
    }

    /// Factor definitions as Python dictionaries following canonical serde fields.
    #[getter]
    fn factors<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.factors)
    }

    /// Covariance matrix aligned to `factor_ids`.
    #[getter]
    fn covariance(&self) -> PyFactorCovarianceMatrix {
        PyFactorCovarianceMatrix::from_inner(self.inner.covariance.clone())
    }

    /// Declarative dependency-to-factor matching configuration.
    #[getter]
    fn matching<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.matching)
    }

    /// Sensitivity extraction strategy (`delta_based` or `full_repricing`).
    #[getter]
    fn pricing_mode(&self) -> String {
        self.inner.pricing_mode.to_string()
    }

    /// Risk measure represented as its canonical Python scalar or dictionary.
    #[getter]
    fn risk_measure<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.risk_measure)
    }

    /// Optional finite-difference bump overrides as a Python dictionary.
    #[getter]
    fn bump_size<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .bump_size
            .as_ref()
            .map(|value| serde_to_py(py, value))
            .transpose()
    }

    /// Policy for unmatched dependencies, or `None` when the default applies.
    #[getter]
    fn unmatched_policy(&self) -> Option<String> {
        self.inner.unmatched_policy.map(|policy| policy.to_string())
    }

    /// Validate that matching rules emit only declared factor identifiers.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Support pickle through the canonical JSON representation.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    fn __repr__(&self) -> String {
        format!("FactorModelConfig(n_factors={})", self.inner.factors.len())
    }
}

/// Vol-forecast view over a calibrated :class:`CreditFactorModel`.
///
/// The forecaster is a thin wrapper; all business logic stays in Rust.
/// ``VolHorizon::Custom`` is intentionally **not** exposed (closures don't
/// cross the FFI boundary cleanly).
///
/// Horizon strings accepted by :meth:`covariance_at` and
/// :meth:`idiosyncratic_vol`:
///
/// - ``"one_step"`` — calibrated annualized variance unchanged.
/// - ``"unconditional"`` — long-run (identical to ``"one_step"`` for
///   ``Sample`` vol model).
/// - ``{"n_steps": N}`` (JSON string) — variance scaled by ``N``.
///
/// Example:
///     >>> from finstack_quant.factor_model.credit import CreditCalibrator, FactorCovarianceForecast
///     >>> config_json = (
///     ...     '{"policy":"globally_off","hierarchy":{"levels":[]},"min_bucket_size_per_level":{"per_level":[]},'
///     ...     '"vol_model":"sample","covariance_strategy":"diagonal","beta_shrinkage":"none",'
///     ...     '"use_returns_or_levels":"returns","panel_frequency":"monthly","bucket_weighting":"equal"}'
///     ... )
///     >>> inputs_json = (
///     ...     '{"history_panel":{"dates":["2024-01-01","2024-02-01"],"spreads":{"A":[0.010,0.0101]}},'
///     ...     '"issuer_tags":{"tags":{"A":{}}},"generic_factor":{"spec":{"name":"G","series_id":"G"},'
///     ...     '"values":[0.010,0.0101]},"as_of":"2024-02-01","as_of_spreads":{"A":0.0101},'
///     ...     '"idiosyncratic_overrides":{}}'
///     ... )
///     >>> model = CreditCalibrator(config_json).calibrate(inputs_json)
///     >>> forecast = FactorCovarianceForecast(model)
///     >>> forecast.covariance_at("one_step").factor_ids
///     ['credit::generic']
#[pyclass(
    name = "FactorCovarianceForecast",
    module = "finstack_quant.factor_model.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyFactorCovarianceForecast {
    /// We store the model by value (cloned from the Python wrapper) so that
    /// `FactorCovarianceForecast<'a>` lifetime requirements don't escape.
    model: finstack_quant_factor_model::credit::hierarchy::CreditFactorModel,
}

/// Parse a horizon descriptor from a Python string.
///
/// Accepted forms:
/// - `"one_step"` → `VolHorizon::OneStep`
/// - `"unconditional"` → `VolHorizon::Unconditional`
/// - JSON string `'{"n_steps": 5}'` → `VolHorizon::NSteps(5)`
///
/// Delegates to the canonical [`VolHorizon::parse`] implementation in
/// `finstack-quant-portfolio`; this wrapper only maps the error to `PyValueError`.
fn parse_vol_horizon(s: &str) -> PyResult<finstack_quant_portfolio::factor_model::VolHorizon> {
    finstack_quant_portfolio::factor_model::VolHorizon::parse(s).map_err(crate::errors::value_error)
}

#[pymethods]
impl PyFactorCovarianceForecast {
    /// Wrap a :class:`CreditFactorModel` for vol forecasting.
    ///
    /// Args:
    ///     model: Calibrated :class:`CreditFactorModel` artifact.
    #[new]
    fn new(model: &PyCreditFactorModel) -> Self {
        Self {
            model: model.inner.clone(),
        }
    }

    /// Build the factor covariance matrix ``Σ(t, h) = D · ρ_static · D``.
    ///
    /// Args:
    ///     horizon: Horizon descriptor string — ``"one_step"``,
    ///         ``"unconditional"``, or a JSON string ``'{"n_steps": N}'``.
    ///
    /// Returns:
    ///     Typed :class:`FactorCovarianceMatrix` with row-major data and factor accessors.
    ///
    /// Raises:
    ///     ValueError: If the horizon string is invalid or the model data is
    ///         inconsistent (mismatched axes, negative variance).
    fn covariance_at(&self, py: Python<'_>, horizon: &str) -> PyResult<PyFactorCovarianceMatrix> {
        let h = parse_vol_horizon(horizon)?;
        let cov = py
            .detach(|| {
                let forecast =
                    finstack_quant_portfolio::factor_model::FactorCovarianceForecast::new(
                        &self.model,
                    );
                forecast.covariance_at(h)
            })
            .map_err(display_to_py)?;
        Ok(PyFactorCovarianceMatrix::from_inner(cov))
    }

    /// Idiosyncratic vol (std dev) for a specific issuer at the requested horizon.
    ///
    /// Args:
    ///     issuer_id: Issuer identifier string.
    ///     horizon: Horizon descriptor (same vocabulary as :meth:`covariance_at`).
    ///
    /// Returns:
    ///     Idiosyncratic standard deviation (annualized).
    ///
    /// Raises:
    ///     ValueError: If the issuer is not present in the model's vol state or
    ///         the calibrated variance is negative.
    fn idiosyncratic_vol(&self, py: Python<'_>, issuer_id: &str, horizon: &str) -> PyResult<f64> {
        let h = parse_vol_horizon(horizon)?;
        let id = finstack_quant_core::types::IssuerId::new(issuer_id);
        py.detach(|| {
            let forecast =
                finstack_quant_portfolio::factor_model::FactorCovarianceForecast::new(&self.model);
            forecast.idiosyncratic_vol(&id, h)
        })
        .map_err(display_to_py)
    }

    /// Build a typed portfolio-level factor-model configuration using
    /// ``Σ(t, h)`` at the given horizon and risk measure.
    ///
    /// Args:
    ///     horizon: Horizon descriptor (same vocabulary as :meth:`covariance_at`).
    ///     risk_measure_json: Risk measure — ``"variance"``, ``"volatility"``,
    ///         or a JSON string (e.g. ``'{"var": {"confidence": 0.99}}'``).
    ///
    /// Returns:
    ///     Typed :class:`FactorModelConfig` ready for inspection or ``to_json()``.
    ///
    /// Raises:
    ///     ValueError: If the horizon or risk measure is invalid, or the model
    ///         builder rejects the assembled configuration.
    fn factor_model_at(
        &self,
        py: Python<'_>,
        horizon: &str,
        risk_measure_json: &str,
    ) -> PyResult<PyFactorModelConfig> {
        let h = parse_vol_horizon(horizon)?;
        let measure: finstack_quant_factor_model::RiskMeasure =
            serde_json::from_str(risk_measure_json).map_err(display_to_py)?;
        let config = py
            .detach(|| {
                let forecast =
                    finstack_quant_portfolio::factor_model::FactorCovarianceForecast::new(
                        &self.model,
                    );
                forecast.factor_model_config_at(h, measure)
            })
            .map_err(display_to_py)?;
        Ok(PyFactorModelConfig::from_inner(config))
    }

    fn __repr__(&self) -> String {
        format!(
            "FactorCovarianceForecast(as_of={:?}, n_factors={})",
            self.model.as_of,
            self.model.config.factors.len(),
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCreditFactorModel>()?;
    m.add_class::<PyCreditCalibrator>()?;
    m.add_class::<PyLevelsAtDate>()?;
    m.add_class::<PyPeriodDecomposition>()?;
    m.add_class::<PyFactorCovarianceMatrix>()?;
    m.add_class::<PyFactorModelConfig>()?;
    m.add_class::<PyFactorCovarianceForecast>()?;
    m.add_function(pyo3::wrap_pyfunction!(decompose_levels, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(decompose_period, m)?)?;
    Ok(())
}
