//! Python wrappers for resolved composite instruments and dated history.

use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::money::PyMoney;
use crate::bindings::extract::{extract_instrument_json, extract_market};
use crate::errors::core_to_py;
use finstack_quant_core::types::InstrumentId;
use finstack_quant_valuations::instruments::composite::{
    CompositeExposureReport, CompositeHistoryEngine, CompositeHistoryRow, CompositeInstrument,
    CompositeLegSpec, CompositeMarketObservation, CompositeRebalanceResult, CompositeSpec,
    CompositeState, RebalanceFrequency, RebalanceRule, WeightingMethod,
};
use finstack_quant_valuations::instruments::{Instrument, InstrumentEnvelope, InstrumentJson};
use finstack_quant_valuations::metrics::MetricId;
use pyo3::prelude::*;
use pyo3::types::PyList;

fn parse_json<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> PyResult<T> {
    serde_json::from_str(json)
        .map_err(|err| crate::errors::serde_json_to_py(err, &format!("invalid {what} JSON")))
}

fn to_json<T: serde::Serialize>(value: &T, what: &str) -> PyResult<String> {
    serde_json::to_string(value)
        .map_err(|err| crate::errors::serde_json_to_py(err, &format!("failed to serialize {what}")))
}

fn parse_composite_envelope(json: &str) -> PyResult<CompositeInstrument> {
    match finstack_quant_valuations::pricer::json::parse_instrument_json(json)
        .map_err(core_to_py)?
    {
        InstrumentJson::Composite(instrument) => Ok(*instrument),
        other => Err(crate::errors::value_error(format!(
            "expected composite instrument envelope, found '{}'",
            other.type_tag()
        ))),
    }
}

fn composite_envelope_json(instrument: &CompositeInstrument) -> PyResult<String> {
    to_json(
        &InstrumentEnvelope::new(InstrumentJson::Composite(Box::new(instrument.clone()))),
        "composite instrument envelope",
    )
}

fn parse_observations(json: &str, what: &str) -> PyResult<Vec<CompositeMarketObservation>> {
    parse_json(json, what)
}

/// One self-contained composite leg definition.
#[pyclass(
    name = "CompositeLegSpec",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeLegSpec {
    pub(crate) inner: CompositeLegSpec,
}

#[pymethods]
impl PyCompositeLegSpec {
    /// Build a signed leg from an existing typed instrument or canonical envelope.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Identifier that must equal the embedded instrument's identifier.
    /// instrument : object | str
    ///     Typed Python instrument or canonical ``finstack_quant.instrument/1`` JSON.
    /// weight : float
    ///     Non-zero signed quantity or relative weighting score.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the instrument payload is malformed.
    #[new]
    #[pyo3(text_signature = "(instrument_id, instrument, weight)")]
    fn new(instrument_id: &str, instrument: &Bound<'_, PyAny>, weight: f64) -> PyResult<Self> {
        let envelope = extract_instrument_json(instrument)?;
        let instrument = finstack_quant_valuations::pricer::json::parse_instrument_json(&envelope)
            .map_err(core_to_py)?;
        Ok(Self {
            inner: CompositeLegSpec::new(instrument_id, instrument, weight),
        })
    }

    /// Deserialize a bare ``CompositeLegSpec`` JSON object.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_json(json, "CompositeLegSpec").map(|inner| Self { inner })
    }

    /// Serialize this leg as a bare ``CompositeLegSpec`` JSON object.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "CompositeLegSpec")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Declared and embedded instrument identifier.
    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.to_string()
    }

    /// Signed fixed quantity or dynamic weighting score.
    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    /// Canonical envelope for the embedded instrument.
    #[getter]
    fn instrument_json(&self) -> PyResult<String> {
        to_json(
            &InstrumentEnvelope::new(self.inner.instrument.as_ref().clone()),
            "embedded instrument envelope",
        )
    }
}

/// Serializable policy for resolving signed composite quantities.
#[pyclass(
    name = "WeightingMethod",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWeightingMethod {
    pub(crate) inner: WeightingMethod,
}

#[pymethods]
impl PyWeightingMethod {
    /// Use signed leg weights directly as quantities.
    #[staticmethod]
    fn fixed_quantity() -> Self {
        Self {
            inner: WeightingMethod::FixedQuantity,
        }
    }

    /// Normalize scores to a requested gross reporting-currency notional.
    #[staticmethod]
    fn notional_weighted(gross_notional: PyRef<'_, PyMoney>) -> Self {
        Self {
            inner: WeightingMethod::NotionalWeighted {
                gross_notional: gross_notional.inner,
            },
        }
    }

    /// Build general anchored metric weighting.
    #[staticmethod]
    #[pyo3(signature = (metric, anchor_leg_id, anchor_quantity, neutralize=false))]
    fn metric_weighted(
        metric: &str,
        anchor_leg_id: &str,
        anchor_quantity: f64,
        neutralize: bool,
    ) -> Self {
        Self {
            inner: WeightingMethod::MetricWeighted {
                metric: MetricId::custom(metric),
                anchor_leg_id: InstrumentId::new(anchor_leg_id),
                anchor_quantity,
                neutralize,
            },
        }
    }

    /// Build parallel-DV01-neutral weighting.
    #[staticmethod]
    fn dv01_neutral(anchor_leg_id: &str, anchor_quantity: f64) -> Self {
        Self {
            inner: WeightingMethod::dv01_neutral(anchor_leg_id, anchor_quantity),
        }
    }

    /// Build parallel-DV01-neutral curve weighting.
    #[staticmethod]
    fn curve_neutral(anchor_leg_id: &str, anchor_quantity: f64) -> Self {
        Self {
            inner: WeightingMethod::curve_neutral(anchor_leg_id, anchor_quantity),
        }
    }

    /// Build delta-neutral weighting.
    #[staticmethod]
    fn delta_neutral(anchor_leg_id: &str, anchor_quantity: f64) -> Self {
        Self {
            inner: WeightingMethod::delta_neutral(anchor_leg_id, anchor_quantity),
        }
    }

    /// Build modified-duration weighting without sign-group normalization.
    #[staticmethod]
    fn duration_weighted(anchor_leg_id: &str, anchor_quantity: f64) -> Self {
        Self {
            inner: WeightingMethod::duration_weighted(anchor_leg_id, anchor_quantity),
        }
    }

    /// Build inverse unit-P&L volatility weighting.
    #[staticmethod]
    fn volatility_weighted(
        anchor_leg_id: &str,
        anchor_quantity: f64,
        lookback: usize,
        min_observations: usize,
        annualization_factor: f64,
    ) -> Self {
        Self {
            inner: WeightingMethod::volatility_weighted(
                anchor_leg_id,
                anchor_quantity,
                lookback,
                min_observations,
                annualization_factor,
            ),
        }
    }

    /// Deserialize any weighting policy, including user-defined expressions.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_json(json, "WeightingMethod").map(|inner| Self { inner })
    }

    /// Serialize the canonical weighting policy.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "WeightingMethod")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Explicit or calendar-based composite rebalance schedule.
#[pyclass(
    name = "RebalanceRule",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyRebalanceRule {
    pub(crate) inner: RebalanceRule,
}

#[pymethods]
impl PyRebalanceRule {
    /// Rebalance only when explicitly requested.
    #[staticmethod]
    fn manual() -> Self {
        Self {
            inner: RebalanceRule::Manual,
        }
    }

    /// Rebalance on strictly increasing ISO-8601 dates.
    #[staticmethod]
    fn dates(dates: Vec<String>) -> PyResult<Self> {
        let dates = dates
            .iter()
            .map(|date| crate::bindings::date_utils::parse_iso_date_py(date))
            .collect::<PyResult<Vec<_>>>()?;
        let inner = RebalanceRule::Dates { dates };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Build a calendar-aware cadence from canonical snake-case enum values.
    #[staticmethod]
    #[pyo3(signature = (start, frequency, calendar_id, business_day_convention, end=None))]
    fn calendar(
        start: &str,
        frequency: &str,
        calendar_id: &str,
        business_day_convention: &str,
        end: Option<&str>,
    ) -> PyResult<Self> {
        let inner = RebalanceRule::Calendar {
            start: crate::bindings::date_utils::parse_iso_date_py(start)?,
            end: end
                .map(crate::bindings::date_utils::parse_iso_date_py)
                .transpose()?,
            frequency: super::instruments::enum_from_str::<RebalanceFrequency>(
                frequency,
                "rebalance frequency",
            )?,
            calendar_id: calendar_id.to_string(),
            business_day_convention: super::instruments::enum_from_str(
                business_day_convention,
                "business-day convention",
            )?,
        };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Deserialize a canonical rebalance rule.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: RebalanceRule = parse_json(json, "RebalanceRule")?;
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize the canonical rebalance rule.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "RebalanceRule")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Unresolved composite economic definition and future policy.
#[pyclass(
    name = "CompositeSpec",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeSpec {
    pub(crate) inner: CompositeSpec,
}

#[pymethods]
impl PyCompositeSpec {
    /// Build a composite definition from typed legs and policies.
    #[new]
    fn new(
        id: &str,
        reporting_currency: PyRef<'_, PyCurrency>,
        capital: PyRef<'_, PyMoney>,
        legs: Vec<PyRef<'_, PyCompositeLegSpec>>,
        weighting_method: PyRef<'_, PyWeightingMethod>,
        rebalance_rule: PyRef<'_, PyRebalanceRule>,
    ) -> PyResult<Self> {
        let inner = CompositeSpec::new(
            id,
            reporting_currency.inner,
            capital.inner,
            legs.into_iter().map(|leg| leg.inner.clone()).collect(),
            weighting_method.inner.clone(),
            rebalance_rule.inner.clone(),
        );
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Deserialize and validate a bare ``CompositeSpec`` JSON object.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: CompositeSpec = parse_json(json, "CompositeSpec")?;
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this unresolved specification.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "CompositeSpec")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Stable composite identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// ISO reporting-currency code.
    #[getter]
    fn reporting_currency(&self) -> String {
        self.inner.reporting_currency.to_string()
    }

    /// Resolve a new immutable state using market data available through ``as_of``.
    #[pyo3(signature = (market, as_of, history_json="[]"))]
    fn initialize(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        history_json: &str,
    ) -> PyResult<PyCompositeRebalanceResult> {
        let market = extract_market(py, market)?;
        let as_of = crate::bindings::date_utils::extract_date(as_of)?;
        let history = parse_observations(history_json, "composite market history")?;
        self.inner
            .initialize(&market, as_of, &history)
            .map(PyCompositeRebalanceResult::from_inner)
            .map_err(core_to_py)
    }
}

/// Frozen resolved quantities and their effective date.
#[pyclass(
    name = "CompositeState",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeState {
    pub(crate) inner: CompositeState,
}

#[pymethods]
impl PyCompositeState {
    /// Deserialize a bare ``CompositeState`` JSON object.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_json(json, "CompositeState").map(|inner| Self { inner })
    }

    /// Serialize this frozen state.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "CompositeState")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// ISO-8601 state effective date.
    #[getter]
    fn effective_date(&self) -> String {
        self.inner.effective_date.to_string()
    }

    /// Resolved signed top-level quantities keyed by leg identifier.
    #[getter]
    fn resolved_quantities(&self) -> std::collections::BTreeMap<String, f64> {
        self.inner
            .resolved_legs
            .iter()
            .map(|leg| (leg.instrument_id.to_string(), leg.quantity))
            .collect()
    }
}

/// Priceable composite with an immutable resolved state.
#[pyclass(
    name = "CompositeInstrument",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeInstrument {
    pub(crate) inner: CompositeInstrument,
}

impl PyCompositeInstrument {
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        composite_envelope_json(&self.inner)
    }
}

#[pymethods]
impl PyCompositeInstrument {
    /// Deserialize and validate a canonical composite instrument envelope.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_composite_envelope(json).map(|inner| Self { inner })
    }

    /// Serialize as a canonical ``finstack_quant.instrument/1`` envelope.
    fn to_json(&self) -> PyResult<String> {
        self.envelope_json()
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Stable composite identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.spec.id.to_string()
    }

    /// Clone the unresolved economic specification.
    #[getter]
    fn spec(&self) -> PyCompositeSpec {
        PyCompositeSpec {
            inner: self.inner.spec.clone(),
        }
    }

    /// Clone the frozen resolved state.
    #[getter]
    fn state(&self) -> PyCompositeState {
        PyCompositeState {
            inner: self.inner.state.clone(),
        }
    }

    /// Explicitly resolve a new immutable state and primitive quantity deltas.
    #[pyo3(signature = (market, as_of, history_json="[]"))]
    fn rebalance(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        history_json: &str,
    ) -> PyResult<PyCompositeRebalanceResult> {
        let market = extract_market(py, market)?;
        let as_of = crate::bindings::date_utils::extract_date(as_of)?;
        let history = parse_observations(history_json, "composite market history")?;
        self.inner
            .rebalance(&market, as_of, &history)
            .map(PyCompositeRebalanceResult::from_inner)
            .map_err(core_to_py)
    }

    /// Price and aggregate primitive net/gross value and additive risk.
    #[pyo3(signature = (market, as_of, metrics=None))]
    fn primitive_exposures(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        metrics: Option<Vec<String>>,
    ) -> PyResult<PyCompositeExposureReport> {
        let market = extract_market(py, market)?;
        let as_of = crate::bindings::date_utils::extract_date(as_of)?;
        let metrics = metrics
            .unwrap_or_default()
            .into_iter()
            .map(MetricId::custom)
            .collect::<Vec<_>>();
        self.inner
            .primitive_exposure_report(&market, as_of, &metrics)
            .map(PyCompositeExposureReport::from_inner)
            .map_err(core_to_py)
    }

    /// Return primitive execution deltas from an optional prior resolved state.
    #[pyo3(signature = (previous=None))]
    fn execution_trades(&self, previous: Option<&PyCompositeInstrument>) -> PyResult<String> {
        let trades = self
            .inner
            .execution_trades(previous.map(|value| &value.inner))
            .map_err(core_to_py)?;
        to_json(&trades, "composite execution trades")
    }
}

/// New resolved instrument plus flattened primitive trades.
#[pyclass(
    name = "CompositeRebalanceResult",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeRebalanceResult {
    pub(crate) inner: CompositeRebalanceResult,
}

impl PyCompositeRebalanceResult {
    fn from_inner(inner: CompositeRebalanceResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCompositeRebalanceResult {
    /// Deserialize a complete rebalance result.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: CompositeRebalanceResult = parse_json(json, "CompositeRebalanceResult")?;
        inner.instrument.validate_invariants().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Newly resolved immutable composite instrument.
    #[getter]
    fn instrument(&self) -> PyCompositeInstrument {
        PyCompositeInstrument {
            inner: self.inner.instrument.clone(),
        }
    }

    /// JSON array of net primitive quantity deltas.
    #[getter]
    fn trades_json(&self) -> PyResult<String> {
        to_json(&self.inner.trades, "composite rebalance trades")
    }

    /// Serialize the complete rebalance result.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "CompositeRebalanceResult")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Primitive path, net, and gross exposure report.
#[pyclass(
    name = "CompositeExposureReport",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeExposureReport {
    pub(crate) inner: CompositeExposureReport,
}

impl PyCompositeExposureReport {
    fn from_inner(inner: CompositeExposureReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCompositeExposureReport {
    /// Deserialize a primitive exposure report.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_json(json, "CompositeExposureReport").map(|inner| Self { inner })
    }

    /// Serialize path-level and aggregate exposures as JSON.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "CompositeExposureReport")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Chronological composite history rows.
#[pyclass(
    name = "CompositeHistoryResult",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompositeHistoryResult {
    pub(crate) inner: Vec<CompositeHistoryRow>,
}

#[pymethods]
impl PyCompositeHistoryResult {
    /// Deserialize a chronological array of composite history rows.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        parse_json(json, "composite history").map(|inner| Self { inner })
    }

    /// Number of dated output rows.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Serialize one zero-based history row.
    fn row_json(&self, index: usize) -> PyResult<String> {
        let row = self.inner.get(index).ok_or_else(|| {
            pyo3::exceptions::PyIndexError::new_err(format!(
                "history row index {index} is out of range"
            ))
        })?;
        to_json(row, "CompositeHistoryRow")
    }

    /// Serialize every dated history row as a JSON array.
    fn to_json(&self) -> PyResult<String> {
        to_json(&self.inner, "composite history")
    }

    /// Support pickle through the canonical JSON round-trip.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

/// Dated-market engine for composite P&L, returns, exposures, and rebalances.
#[pyclass(
    name = "CompositeHistoryEngine",
    module = "finstack_quant.valuations.composite",
    frozen,
    skip_from_py_object
)]
pub struct PyCompositeHistoryEngine;

#[pymethods]
impl PyCompositeHistoryEngine {
    /// Initialize a specification at the first observation and run history.
    #[staticmethod]
    #[pyo3(signature = (spec, observations_json, warmup_json="[]", metrics=None))]
    fn run_from_spec(
        spec: &PyCompositeSpec,
        observations_json: &str,
        warmup_json: &str,
        metrics: Option<Vec<String>>,
    ) -> PyResult<PyCompositeHistoryResult> {
        let observations = parse_observations(observations_json, "composite observations")?;
        let warmup = parse_observations(warmup_json, "composite warmup")?;
        let metrics = metrics
            .unwrap_or_default()
            .into_iter()
            .map(MetricId::custom)
            .collect::<Vec<_>>();
        CompositeHistoryEngine::run_from_spec(&spec.inner, &warmup, &observations, &metrics)
            .map(|inner| PyCompositeHistoryResult { inner })
            .map_err(core_to_py)
    }

    /// Run history from an already-resolved immutable composite.
    #[staticmethod]
    #[pyo3(signature = (instrument, observations_json, metrics=None))]
    fn run(
        instrument: &PyCompositeInstrument,
        observations_json: &str,
        metrics: Option<Vec<String>>,
    ) -> PyResult<PyCompositeHistoryResult> {
        let observations = parse_observations(observations_json, "composite observations")?;
        let metrics = metrics
            .unwrap_or_default()
            .into_iter()
            .map(MetricId::custom)
            .collect::<Vec<_>>();
        CompositeHistoryEngine::run(&instrument.inner, &observations, &metrics)
            .map(|inner| PyCompositeHistoryResult { inner })
            .map_err(core_to_py)
    }
}

pub(crate) const EXPORTS: &[&str] = &[
    "CompositeExposureReport",
    "CompositeHistoryEngine",
    "CompositeHistoryResult",
    "CompositeInstrument",
    "CompositeLegSpec",
    "CompositeRebalanceResult",
    "CompositeSpec",
    "CompositeState",
    "RebalanceRule",
    "WeightingMethod",
];

/// Register the ``finstack_quant.valuations.composite`` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "composite")?;
    let qual = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &module,
        "composite",
        "finstack_quant.valuations",
    )?;
    module.setattr(
        "__doc__",
        "Resolved cross-asset composite instruments, primitive exposures, and dated history.",
    )?;
    module.add_class::<PyCompositeLegSpec>()?;
    module.add_class::<PyWeightingMethod>()?;
    module.add_class::<PyRebalanceRule>()?;
    module.add_class::<PyCompositeSpec>()?;
    module.add_class::<PyCompositeState>()?;
    module.add_class::<PyCompositeInstrument>()?;
    module.add_class::<PyCompositeRebalanceResult>()?;
    module.add_class::<PyCompositeExposureReport>()?;
    module.add_class::<PyCompositeHistoryResult>()?;
    module.add_class::<PyCompositeHistoryEngine>()?;
    module.setattr("__all__", PyList::new(py, EXPORTS)?)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &module, &qual)
}
