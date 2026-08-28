//! Python wrappers for scenario engine application.

use crate::bindings::core::market_data::context::PyMarketContext;
use crate::bindings::extract::{extract_market, extract_model_ref};
use crate::bindings::pandas_utils::{serde_object_to_single_row_dataframe, serde_to_py};
use crate::bindings::statements::types::PyFinancialModelSpec;
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyAny;

/// Report describing what a scenario application changed.
///
/// Returned by :func:`apply_scenario` and :func:`apply_scenario_to_market` as
/// the ``report`` attribute of an :class:`ApplicationResult`.
#[pyclass(
    name = "ApplicationReport",
    module = "finstack_quant.scenarios",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyApplicationReport {
    pub(crate) inner: finstack_quant_scenarios::engine::ApplicationReport,
}

#[pymethods]
impl PyApplicationReport {
    /// Number of effects successfully applied to the execution context.
    ///
    /// One user-level operation can produce zero, one, or many effects. Inspect
    /// ``changes`` and ``warnings`` rather than treating this as coverage.
    #[getter]
    fn operations_applied(&self) -> usize {
        self.inner.operations_applied
    }

    /// Number of user-provided operations before hierarchy expansion.
    #[getter]
    fn user_operations(&self) -> usize {
        self.inner.user_operations
    }

    /// Number of operations the engine attempted after hierarchy expansion.
    #[getter]
    fn expanded_operations(&self) -> usize {
        self.inner.expanded_operations
    }

    /// Non-fatal warnings raised while applying the scenario.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner
            .warnings
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Audit stamp: numeric mode, rounding context, and FX policy in force.
    #[getter]
    fn meta<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .meta
            .as_ref()
            .map(|meta| serde_to_py(py, meta))
            .transpose()
    }

    /// Metadata describing exactly which market state the effects changed.
    #[getter]
    fn changes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.changes)
    }

    /// Roll-forward report, present only when the scenario contained a
    /// ``time_roll_forward`` operation.
    #[getter]
    fn time_roll<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .time_roll
            .as_ref()
            .map(|roll| serde_to_py(py, roll))
            .transpose()
    }

    /// Export the report counters as a single-row :class:`pandas.DataFrame`.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe(py, &self.inner)
    }

    /// Serialize to a compact JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_scenarios::engine::ApplicationReport =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("ApplicationReport", &self.inner)
    }
}

/// Result of applying a scenario: the mutated market, the mutated model (when
/// one was supplied), and the application report.
#[pyclass(
    name = "ApplicationResult",
    module = "finstack_quant.scenarios",
    frozen,
    skip_from_py_object
)]
pub struct PyApplicationResult {
    market: finstack_quant_core::market_data::context::MarketContext,
    model: Option<finstack_quant_statements::FinancialModelSpec>,
    report: finstack_quant_scenarios::engine::ApplicationReport,
}

#[pymethods]
impl PyApplicationResult {
    /// The mutated market context.
    #[getter]
    fn market(&self) -> PyMarketContext {
        PyMarketContext::from_inner(self.market.clone())
    }

    /// The mutated financial model, or ``None`` when no model was supplied.
    #[getter]
    fn model(&self) -> Option<PyFinancialModelSpec> {
        self.model.as_ref().map(|inner| PyFinancialModelSpec {
            inner: inner.clone(),
        })
    }

    /// What the scenario changed.
    #[getter]
    fn report(&self) -> PyApplicationReport {
        PyApplicationReport {
            inner: self.report.clone(),
        }
    }

    /// Serialize to a compact JSON string.
    ///
    /// Emits the canonical
    /// :rust:struct:`finstack_quant_scenarios::engine::ApplicationEnvelope`
    /// shape, with ``market`` and ``model`` as nested objects.
    fn to_json(&self) -> PyResult<String> {
        let envelope = finstack_quant_scenarios::engine::ApplicationEnvelope::from_contexts(
            self.report.clone(),
            &self.market,
            self.model.as_ref(),
        )
        .map_err(display_to_py)?;
        serde_json::to_string(&envelope).map_err(display_to_py)
    }

    /// Deserialize from a JSON string produced by :meth:`to_json`.
    ///
    /// Accepts the canonical
    /// :rust:struct:`finstack_quant_scenarios::engine::ApplicationEnvelope`
    /// shape and rebuilds the market, model, and report.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let envelope: finstack_quant_scenarios::engine::ApplicationEnvelope =
            serde_json::from_str(json).map_err(display_to_py)?;
        let market: finstack_quant_core::market_data::context::MarketContext =
            serde_json::from_value(envelope.market).map_err(display_to_py)?;
        let model: Option<finstack_quant_statements::FinancialModelSpec> = envelope
            .model
            .map(serde_json::from_value)
            .transpose()
            .map_err(display_to_py)?;
        Ok(Self {
            market,
            model,
            report: finstack_quant_scenarios::engine::ApplicationReport {
                operations_applied: envelope.operations_applied,
                user_operations: envelope.user_operations,
                expanded_operations: envelope.expanded_operations,
                changes: envelope.changes,
                warnings: envelope.warnings,
                meta: envelope.meta,
                time_roll: envelope.time_roll,
            },
        })
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("ApplicationResult", &self.market)
    }
}

fn apply_with_context(
    spec: &finstack_quant_scenarios::ScenarioSpec,
    market: &mut finstack_quant_core::market_data::context::MarketContext,
    model: Option<&mut finstack_quant_statements::FinancialModelSpec>,
    as_of: time::Date,
) -> finstack_quant_scenarios::Result<finstack_quant_scenarios::engine::ApplicationReport> {
    let engine = finstack_quant_scenarios::ScenarioEngine::new().with_recalibration_provider(
        std::sync::Arc::new(
            finstack_quant_calibration::recalibration::CachedRecalibrationProvider::new(),
        ),
    );
    let mut ctx = finstack_quant_scenarios::ExecutionContext {
        market,
        model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of,
    };
    engine.apply(spec, &mut ctx)
}

/// Apply a scenario to a market context and financial model.
///
/// Parameters
/// ----------
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// model : FinancialModelSpec | str
///     A ``FinancialModelSpec`` object or a JSON string.
/// as_of : datetime.date | str
///     Valuation date, either a date-like object or an ISO 8601 string.
///
/// Returns
/// -------
/// ApplicationResult
///     Typed result exposing ``market`` (:class:`MarketContext`), ``model``
///     (:class:`FinancialModelSpec`), and ``report``
///     (:class:`ApplicationReport`). Call ``.to_json()`` for the canonical
///     JSON envelope.
///
/// Notes
/// -----
/// This entry point supplies no instrument portfolio and no holiday calendar
/// to the engine, so instrument-scoped operations
/// (``instrument_price_pct_by_*``, ``instrument_spread_bp_by_*``,
/// ``asset_correlation_pts``, ``prepay_default_correlation_pts``) are inert
/// and produce a warning, and ``time_roll_forward`` in ``business_days`` mode
/// adjusts without holiday information.
#[pyfunction]
fn apply_scenario(
    py: Python<'_>,
    scenario_json: &str,
    market: &Bound<'_, PyAny>,
    model: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
) -> PyResult<PyApplicationResult> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(display_to_py)?;
    let mut market = extract_market(py, market)?;
    let mut model = extract_model_ref(model)?.into_owned();
    let date = crate::bindings::date_utils::extract_date(as_of)?;

    // Release the GIL for scenario application: shifts + re-pricing can run for seconds.
    let (report, market, model) = py.detach(|| {
        let report = apply_with_context(&spec, &mut market, Some(&mut model), date);
        (report, market, model)
    });
    let report = report.map_err(display_to_py)?;

    Ok(PyApplicationResult {
        market,
        model: Some(model),
        report,
    })
}

/// Apply a scenario to a market context only (no model).
///
/// Parameters
/// ----------
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : datetime.date | str
///     Valuation date, either a date-like object or an ISO 8601 string.
///
/// Returns
/// -------
/// ApplicationResult
///     Typed result whose ``model`` attribute is ``None``.
///
/// Notes
/// -----
/// As with ``apply_scenario``, no instrument portfolio or holiday calendar is
/// supplied: instrument-scoped operations are inert (with a warning) and
/// business-day time rolls adjust without holiday information.
#[pyfunction]
fn apply_scenario_to_market(
    py: Python<'_>,
    scenario_json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
) -> PyResult<PyApplicationResult> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(display_to_py)?;
    let mut market = extract_market(py, market)?;
    let date = crate::bindings::date_utils::extract_date(as_of)?;

    let (report, market) = py.detach(|| {
        let report = apply_with_context(&spec, &mut market, None, date);
        (report, market)
    });
    let report = report.map_err(display_to_py)?;

    Ok(PyApplicationResult {
        market,
        model: None,
        report,
    })
}

/// Register engine functions.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyApplicationReport>()?;
    m.add_class::<PyApplicationResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario_to_market, m)?)?;
    Ok(())
}
