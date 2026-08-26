//! Python wrapper for the type-state ModelBuilder.
//!
//! Since Python cannot model Rust type-state at the type level, we collapse
//! the two states into a single class and track readiness at runtime.

use super::capital_structure::PyWaterfallSpec;
use super::types::{PyFinancialModelSpec, PyForecastSpec};
use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, serde_json_to_py, statements_to_py};
use finstack_quant_core::dates::PeriodId;
use finstack_quant_core::money::fx::FxConversionPolicy;
use finstack_quant_statements::builder::{MixedNodeBuilder, ModelBuilder};
use finstack_quant_statements::types::{AmountOrScalar, FinancialStatementInstrument};
use pyo3::prelude::*;

/// Validate a formula the same way `ModelBuilder::compute` / `formula` do,
/// without consuming the builder.
///
/// The Rust builder methods consume `self` and return `Err` on an invalid
/// formula, so calling them and mapping the error still leaves the Python
/// wrapper's `inner` as `None`, bricking every later call. Running the identical
/// checks first — reserved-prefix node id, non-empty formula, and
/// `parse_and_compile` — lets a typo fail without destroying accumulated state.
fn validate_compute_args(node_id: &str, formula: &str) -> PyResult<()> {
    finstack_quant_statements::builder::validate_node_id(node_id).map_err(statements_to_py)?;
    if formula.trim().is_empty() {
        return Err(crate::errors::value_error("Formula cannot be empty"));
    }
    finstack_quant_statements::dsl::parse_and_compile(formula).map_err(statements_to_py)?;
    Ok(())
}

/// Validate a period range the same way `ModelBuilder::periods` does, without
/// consuming the builder (see [`validate_compute_args`]).
fn validate_periods_args(range: &str, actuals_until: Option<&str>) -> PyResult<()> {
    finstack_quant_core::dates::build_periods(range, actuals_until).map_err(core_to_py)?;
    Ok(())
}

/// Builder for financial models (type-state collapsed for Python).
///
/// Every configuration method returns the builder, so calls chain::
///
///     model = (
///         ModelBuilder("Acme Corp")
///         .periods("2025Q1..Q4", "2025Q2")
///         .value("revenue", [("2025Q1", 10_000_000.0), ("2025Q2", 11_000_000.0)])
///         .compute("cogs", "revenue * 0.6")
///         .build()
///     )
///
/// Statement-per-line style works identically, since each call also mutates in
/// place::
///
///     builder = ModelBuilder("Acme Corp")
///     builder.periods("2025Q1..Q4", "2025Q2")
///     builder.compute("cogs", "revenue * 0.6")
///     model = builder.build()
///
/// ``build()`` and ``mixed()`` are terminal: they consume the builder, so no
/// further configuration call may follow on the same object.
#[pyclass(name = "ModelBuilder", module = "finstack_quant.statements")]
pub struct PyModelBuilder {
    inner: Option<BuilderState>,
}

impl PyModelBuilder {
    /// Shared constructor behind `ModelBuilder(id)` and
    /// `FinancialModelSpec.builder(id)`.
    pub(crate) fn start(id: &str) -> Self {
        Self {
            inner: Some(BuilderState::NeedPeriods(ModelBuilder::new(id))),
        }
    }
}

enum BuilderState {
    NeedPeriods(ModelBuilder<finstack_quant_statements::builder::NeedPeriods>),
    Ready(ModelBuilder<finstack_quant_statements::builder::Ready>),
}

/// Metric registry used to add reusable statement metrics to a model.
#[pyclass(
    name = "MetricRegistry",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMetricRegistry {
    inner: finstack_quant_statements::registry::Registry,
}

#[pymethods]
impl PyMetricRegistry {
    /// Create an empty metric registry.
    #[new]
    fn new() -> Self {
        Self {
            inner: finstack_quant_statements::registry::Registry::new(),
        }
    }

    /// Create a registry preloaded with built-in metrics.
    #[staticmethod]
    fn with_builtins() -> PyResult<Self> {
        let inner = finstack_quant_statements::registry::Registry::with_builtins()
            .map_err(statements_to_py)?;
        Ok(Self { inner })
    }

    /// Load built-in metrics into this registry.
    fn load_builtins(&mut self) -> PyResult<()> {
        self.inner.load_builtins().map_err(statements_to_py)
    }

    /// Load metrics from a JSON string.
    fn load_from_json_str(&mut self, json: &str) -> PyResult<()> {
        self.inner
            .load_from_json_str(json)
            .map(|_| ())
            .map_err(statements_to_py)
    }

    /// Load metrics from a JSON file path.
    fn load_from_json(&mut self, path: &str) -> PyResult<()> {
        self.inner.load_from_json(path).map_err(statements_to_py)
    }

    /// Return whether a fully qualified metric exists.
    fn has(&self, qualified_id: &str) -> bool {
        self.inner.has(qualified_id)
    }

    /// Number of metrics in the registry.
    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

/// Fluent builder for a mixed statement node.
#[pyclass(name = "MixedNodeBuilder", module = "finstack_quant.statements")]
pub struct PyMixedNodeBuilder {
    inner: Option<MixedNodeBuilder>,
}

#[pymethods]
impl PyMixedNodeBuilder {
    /// Set scalar explicit values for the mixed node.
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     This builder, for chaining.
    fn values<'py>(
        mut slf: PyRefMut<'py, Self>,
        values: Vec<(String, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let builder = slf.take()?;
        let parsed = parse_scalar_values(values)?;
        slf.inner = Some(builder.values(&parsed));
        Ok(slf)
    }

    /// Set monetary explicit values for the mixed node.
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     This builder, for chaining.
    fn values_money<'py>(
        mut slf: PyRefMut<'py, Self>,
        values: Vec<(String, PyMoney)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let builder = slf.take()?;
        let parsed: Vec<(PeriodId, AmountOrScalar)> = values
            .into_iter()
            .map(|(p, money)| {
                let pid: PeriodId = p.parse().map_err(core_to_py)?;
                Ok((pid, AmountOrScalar::Amount(money.inner)))
            })
            .collect::<PyResult<Vec<_>>>()?;
        slf.inner = Some(builder.values(&parsed));
        Ok(slf)
    }

    /// Set the forecast specification.
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     This builder, for chaining.
    fn forecast<'py>(
        mut slf: PyRefMut<'py, Self>,
        forecast_spec: PyRef<'_, PyForecastSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let builder = slf.take()?;
        slf.inner = Some(builder.forecast(forecast_spec.inner.clone()));
        Ok(slf)
    }

    /// Set the fallback formula.
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     This builder, for chaining.
    fn formula<'py>(mut slf: PyRefMut<'py, Self>, formula: &str) -> PyResult<PyRefMut<'py, Self>> {
        // Validate before `take()` so a bad formula does not consume the
        // mixed-node builder and brick the chain. `MixedNodeBuilder::formula`
        // validates via the same `parse_and_compile`.
        if formula.trim().is_empty() {
            return Err(crate::errors::value_error("Formula cannot be empty"));
        }
        finstack_quant_statements::dsl::parse_and_compile(formula).map_err(statements_to_py)?;
        let builder = slf.take()?;
        slf.inner = Some(builder.formula(formula).map_err(statements_to_py)?);
        Ok(slf)
    }

    /// Set the human-readable name.
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     This builder, for chaining.
    fn name<'py>(mut slf: PyRefMut<'py, Self>, name: &str) -> PyResult<PyRefMut<'py, Self>> {
        let builder = slf.take()?;
        slf.inner = Some(builder.name(name));
        Ok(slf)
    }

    /// Build the mixed node and return a ready model builder.
    fn build(&mut self) -> PyResult<PyModelBuilder> {
        let builder = self.take()?;
        let ready = builder.build().map_err(statements_to_py)?;
        Ok(PyModelBuilder {
            inner: Some(BuilderState::Ready(ready)),
        })
    }
}

impl PyMixedNodeBuilder {
    fn take(&mut self) -> PyResult<MixedNodeBuilder> {
        self.inner
            .take()
            .ok_or_else(|| crate::errors::value_error("MixedNodeBuilder has already been consumed"))
    }
}

#[pymethods]
impl PyModelBuilder {
    /// Create a new model builder.
    ///
    /// :meth:`FinancialModelSpec.builder` is the canonical entry point and
    /// returns exactly this value.
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn new(id: &str) -> Self {
        Self::start(id)
    }

    /// Reconstruct a ready builder from an existing model specification.
    ///
    /// Parameters
    /// ----------
    /// spec : FinancialModelSpec
    ///     Typed model whose periods, nodes, metadata, and capital structure
    ///     seed the builder.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Ready builder that can accept additional transformations.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the model has no periods.
    #[staticmethod]
    fn from_spec(spec: &PyFinancialModelSpec) -> PyResult<Self> {
        let builder = ModelBuilder::from_spec(spec.inner.clone()).map_err(statements_to_py)?;
        Ok(Self {
            inner: Some(BuilderState::Ready(builder)),
        })
    }

    /// Define periods using a range expression (e.g. ``"2025Q1..Q4"``).
    ///
    /// Parameters
    /// ----------
    /// range : str
    ///     Period range expression.
    /// actuals_until : str | None
    ///     Optional cutoff for actual values.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(signature = (range, actuals_until=None), text_signature = "($self, range, actuals_until=None)")]
    fn periods<'py>(
        mut slf: PyRefMut<'py, Self>,
        range: &str,
        actuals_until: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Validate before `take_any()` so a bad range does not brick the builder
        // (see `validate_periods_args`).
        validate_periods_args(range, actuals_until)?;
        let state = slf.take_any()?;
        match state {
            BuilderState::NeedPeriods(b) => {
                let ready = b.periods(range, actuals_until).map_err(statements_to_py)?;
                slf.inner = Some(BuilderState::Ready(ready));
                Ok(slf)
            }
            BuilderState::Ready(b) => {
                slf.inner = Some(BuilderState::Ready(b));
                Err(crate::errors::value_error("Periods already set"))
            }
        }
    }

    /// Add a value node with explicit period values.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// values : list[tuple[str, float]]
    ///     List of (period_string, value) tuples.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, node_id, values)")]
    fn value<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        values: Vec<(String, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Parse arguments BEFORE `take_ready()` so a bad period string does not
        // permanently consume the in-progress builder (leaving a misleading
        // "consumed by build()" error on the next call).
        let parsed = parse_scalar_values(values)?;
        let state = slf.take_ready()?;

        let ready = state.value(node_id, &parsed);
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add a scalar value node with explicit period values.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, node_id, values)")]
    fn value_scalar<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        values: Vec<(String, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Parse before `take_ready()` (see `value`).
        let parsed: Vec<(PeriodId, f64)> = values
            .into_iter()
            .map(|(p, v)| {
                let pid: PeriodId = p.parse().map_err(core_to_py)?;
                Ok((pid, v))
            })
            .collect::<PyResult<Vec<_>>>()?;
        let state = slf.take_ready()?;

        let ready = state.value_scalar(node_id, &parsed);
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add a monetary value node with explicit period values.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, node_id, values)")]
    fn value_money<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        values: Vec<(String, PyMoney)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Parse before `take_ready()` (see `value`).
        let parsed: Vec<(PeriodId, finstack_quant_core::money::Money)> = values
            .into_iter()
            .map(|(p, money)| {
                let pid: PeriodId = p.parse().map_err(core_to_py)?;
                Ok((pid, money.inner))
            })
            .collect::<PyResult<Vec<_>>>()?;
        let state = slf.take_ready()?;

        let ready = state.value_money(node_id, &parsed);
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Set point-in-time availability dates for explicit observations.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Existing value or mixed node.
    /// availability_dates : list[tuple[str, datetime.date | str]]
    ///     Period IDs paired with the date each observation became available.
    ///     Unspecified observations default to the period's exclusive end.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If a period/date is invalid or the node does not exist.
    #[pyo3(text_signature = "($self, node_id, availability_dates)")]
    fn availability_dates<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        availability_dates: Vec<(String, Bound<'py, PyAny>)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let parsed = availability_dates
            .into_iter()
            .map(|(period, date)| {
                Ok((
                    period.parse::<PeriodId>().map_err(core_to_py)?,
                    py_to_date(&date)?,
                ))
            })
            .collect::<PyResult<Vec<_>>>()?;
        let state = slf.take_ready()?;
        let ready = state
            .availability_dates(node_id, &parsed)
            .map_err(statements_to_py)?;
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add a computed node with a formula.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// formula : str
    ///     DSL formula expression (e.g. ``"revenue - cogs"``).
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, node_id, formula)")]
    fn compute<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        formula: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Validate BEFORE `take_ready()` so a bad formula does not permanently
        // consume the in-progress builder. `ModelBuilder::compute` consumes
        // `self` and returns `Err` on an invalid formula, which would otherwise
        // leave `inner = None` and brick every subsequent call with a
        // misleading "consumed by build()" error. These are the exact checks
        // `compute` runs internally, so behaviour is unchanged on success.
        validate_compute_args(node_id, formula)?;
        let state = slf.take_ready()?;
        let ready = state.compute(node_id, formula).map_err(statements_to_py)?;
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Start configuring a mixed node.
    #[pyo3(text_signature = "($self, node_id)")]
    fn mixed(&mut self, node_id: &str) -> PyResult<PyMixedNodeBuilder> {
        let state = self.take_ready()?;
        Ok(PyMixedNodeBuilder {
            inner: Some(state.mixed(node_id)),
        })
    }

    /// Attach a forecast specification to an existing or new node.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, node_id, forecast_spec)")]
    fn forecast<'py>(
        mut slf: PyRefMut<'py, Self>,
        node_id: &str,
        forecast_spec: PyRef<'_, PyForecastSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let state = slf.take_ready()?;
        let ready = state.forecast(node_id, forecast_spec.inner.clone());
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Attach a where clause to the last added node.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, where_clause)")]
    fn where_clause<'py>(
        mut slf: PyRefMut<'py, Self>,
        where_clause: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let state = slf.take_ready()?;
        let ready = state.where_clause(where_clause);
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add model-level metadata from a JSON payload.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, key, value_json)")]
    fn with_meta<'py>(
        mut slf: PyRefMut<'py, Self>,
        key: &str,
        value_json: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let value: serde_json::Value = serde_json::from_str(value_json)
            .map_err(|e| serde_json_to_py(e, "invalid meta value JSON"))?;
        let state = slf.take_ready()?;
        let ready = state.with_meta(key, value);
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add all built-in statement metrics.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self)")]
    fn with_builtin_metrics(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let state = slf.take_ready()?;
        let ready = state.with_builtin_metrics().map_err(statements_to_py)?;
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add one metric and its dependencies from a registry.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, qualified_id, registry)")]
    fn add_metric_from_registry<'py>(
        mut slf: PyRefMut<'py, Self>,
        qualified_id: &str,
        registry: PyRef<'_, PyMetricRegistry>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Check membership BEFORE `take_ready()`. An unknown metric id is a
        // routine typo, and the consuming Rust call would otherwise leave
        // `inner = None`, bricking the builder and forcing the caller to
        // rebuild the whole model (same rationale as `validate_compute_args`).
        if !registry.inner.has(qualified_id) {
            return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Metric not found: '{qualified_id}'"
            )));
        }
        let state = slf.take_ready()?;
        let ready = state
            .add_metric_from_registry(qualified_id, &registry.inner)
            .map_err(statements_to_py)?;
        slf.inner = Some(BuilderState::Ready(ready));
        Ok(slf)
    }

    /// Add a fixed-rate bond to the capital structure (US conventions: 30/360, semi-annual).
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Principal amount (must be in a valid Currency).
    /// coupon_rate : float
    ///     Annual coupon rate (e.g. 0.05 for 5%).
    /// issue_date, maturity_date : datetime.date
    ///     Bond issue and maturity dates.
    /// discount_curve_id : str
    ///     Discount curve identifier used for pricing.
    #[pyo3(
        text_signature = "($self, id, notional, coupon_rate, issue_date, maturity_date, discount_curve_id)"
    )]
    fn add_bond<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        coupon_rate: f64,
        issue_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let notional = notional.inner;
        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_bond(
                    id,
                    notional,
                    coupon_rate,
                    issue,
                    maturity,
                    discount_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_bond(
                    id,
                    notional,
                    coupon_rate,
                    issue,
                    maturity,
                    discount_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Add an interest rate swap to the capital structure (US conventions).
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Swap notional.
    /// fixed_rate : float
    ///     Fixed leg rate (e.g. 0.04 for 4%).
    /// start_date, maturity_date : datetime.date
    /// discount_curve_id, forward_curve_id : str
    ///     Discount curve and floating-leg forward curve identifiers.
    #[pyo3(
        text_signature = "($self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn add_swap<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        fixed_rate: f64,
        start_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: &str,
        forward_curve_id: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let notional = notional.inner;
        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_swap(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_swap(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Add a fixed-rate bond with a market convention preset.
    ///
    /// Applies regional day-count, coupon-frequency, and calendar conventions
    /// automatically; ``add_bond`` uses US corporate conventions instead.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Principal amount (must be in a valid Currency).
    /// coupon_rate : float
    ///     Annual coupon rate as a decimal fraction (e.g. ``0.03`` for 3%).
    /// issue_date, maturity_date : datetime.date
    ///     Bond issue and maturity dates.
    /// convention : str
    ///     Regional convention preset, as the canonical snake_case
    ///     identifier: ``"us_treasury"``, ``"us_agency"``, ``"german_bund"``,
    ///     ``"uk_gilt"``, ``"french_oat"``, ``"jgb"``, ``"us_corporate"``,
    ///     or ``"eur_corporate"``.
    /// discount_curve_id : str
    ///     Discount curve identifier used for pricing.
    #[pyo3(
        text_signature = "($self, id, notional, coupon_rate, issue_date, maturity_date, convention, discount_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn add_bond_with_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        coupon_rate: f64,
        issue_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        convention: &str,
        discount_curve_id: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let notional = notional.inner;
        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;
        let convention: finstack_quant_valuations::instruments::BondConvention =
            serde_json::from_value(serde_json::Value::String(convention.to_string())).map_err(
                |e| {
                    crate::errors::value_error(format!(
                        "invalid bond convention {convention:?}: {e}; expected one of \
                         us_treasury, us_agency, german_bund, uk_gilt, french_oat, jgb, \
                         us_corporate, eur_corporate"
                    ))
                },
            )?;
        let rate = finstack_quant_core::types::Rate::from_decimal(coupon_rate);
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_bond_with_convention(
                    id,
                    notional,
                    rate,
                    issue,
                    maturity,
                    convention,
                    discount_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_bond_with_convention(
                    id,
                    notional,
                    rate,
                    issue,
                    maturity,
                    convention,
                    discount_curve_id,
                )
                .map_err(statements_to_py)?,
            ),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Add an interest rate swap with custom leg conventions.
    ///
    /// Exposes day-count, frequency, and business-day-convention parameters
    /// for non-USD swaps (e.g. EUR annual ACT/360 fixed legs); ``add_swap``
    /// uses US conventions instead.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Swap notional.
    /// fixed_rate : float
    ///     Fixed leg rate (e.g. ``0.04`` for 4%).
    /// start_date, maturity_date : datetime.date
    ///     Effective and maturity dates of the swap schedule.
    /// discount_curve_id, forward_curve_id : str
    ///     Discount curve and floating-leg forward curve identifiers.
    /// fixed_frequency : Tenor or str
    ///     Payment frequency of the fixed leg (e.g. ``"1Y"``).
    /// fixed_day_count : DayCount
    ///     Day-count convention applied to the fixed leg.
    /// float_frequency : Tenor or str
    ///     Payment / fixing frequency of the floating leg (e.g. ``"3M"``).
    /// float_day_count : DayCount
    ///     Day-count convention applied to the floating leg.
    /// business_day_convention : BusinessDayConvention or str, optional
    ///     Schedule-date rolling convention (default Modified Following).
    #[pyo3(
        signature = (id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id, fixed_frequency, fixed_day_count, float_frequency, float_day_count, business_day_convention=None),
        text_signature = "($self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id, fixed_frequency, fixed_day_count, float_frequency, float_day_count, business_day_convention=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn add_swap_with_conventions<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        fixed_rate: f64,
        start_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: &str,
        forward_curve_id: &str,
        fixed_frequency: &Bound<'_, PyAny>,
        fixed_day_count: PyRef<'_, crate::bindings::core::dates::daycount::PyDayCount>,
        float_frequency: &Bound<'_, PyAny>,
        float_day_count: PyRef<'_, crate::bindings::core::dates::daycount::PyDayCount>,
        business_day_convention: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let notional = notional.inner;
        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;
        let fixed_frequency = crate::bindings::core::dates::tenor::extract_tenor(fixed_frequency)?;
        let float_frequency = crate::bindings::core::dates::tenor::extract_tenor(float_frequency)?;
        let fixed_day_count = fixed_day_count.inner;
        let float_day_count = float_day_count.inner;
        let resolved_business_day_convention = match business_day_convention {
            Some(obj) => {
                crate::bindings::core::dates::calendar::extract_business_day_convention(obj)?
            }
            None => finstack_quant_core::dates::BusinessDayConvention::ModifiedFollowing,
        };
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_swap_with_conventions(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                    fixed_frequency,
                    fixed_day_count,
                    float_frequency,
                    float_day_count,
                    resolved_business_day_convention,
                )
                .map_err(statements_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_swap_with_conventions(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                    fixed_frequency,
                    fixed_day_count,
                    float_frequency,
                    float_day_count,
                    resolved_business_day_convention,
                )
                .map_err(statements_to_py)?,
            ),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Add a debt instrument from its canonical v1 instrument envelope.
    ///
    /// Use this for supported capital-structure instruments not covered by the
    /// convenience constructors: bonds, convertible bonds, term loans, RCFs,
    /// interest-rate swaps, caps/floors, and swaptions. The envelope is parsed
    /// and narrowed by the canonical Rust contract before it is added.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// spec_json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing the target
    ///     instrument. Bare instrument payloads are rejected.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, id, spec_json)")]
    fn add_debt<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &str,
        spec_json: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let spec = finstack_quant_valuations::pricer::json::parse_instrument_json(spec_json)
            .map_err(core_to_py)?;
        let spec = FinancialStatementInstrument::try_from(spec).map_err(statements_to_py)?;
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.add_debt(id, spec)),
            BuilderState::Ready(b) => BuilderState::Ready(b.add_debt(id, spec)),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Set the reporting currency used for capital-structure totals.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, currency)")]
    fn reporting_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: PyRef<'_, PyCurrency>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let ccy = currency.inner;
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.reporting_currency(ccy)),
            BuilderState::Ready(b) => BuilderState::Ready(b.reporting_currency(ccy)),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Set the FX conversion policy for capital-structure cashflows.
    ///
    /// Parameters
    /// ----------
    /// policy : str
    ///     One of ``"cashflow_date"``, ``"period_end"``, ``"period_average"``.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    #[pyo3(text_signature = "($self, policy)")]
    fn fx_policy<'py>(mut slf: PyRefMut<'py, Self>, policy: &str) -> PyResult<PyRefMut<'py, Self>> {
        let policy_value = serde_json::Value::String(policy.to_string());
        let parsed: FxConversionPolicy =
            serde_json::from_value(policy_value).map_err(|e| {
                crate::errors::value_error(format!(
                    "invalid fx_policy {policy:?}: {e}; expected one of cashflow_date, period_end, period_average"
                ))
            })?;
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.fx_policy(parsed)),
            BuilderState::Ready(b) => BuilderState::Ready(b.fx_policy(parsed)),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Attach a waterfall specification (priority-of-payments, ECF sweep,
    /// PIK toggle, payment classes, and optional prepay nodes).
    ///
    /// Parameters
    /// ----------
    /// waterfall_spec : WaterfallSpec
    ///     Validated or pending waterfall configuration attached to the
    ///     capital structure. Bond / ConvertibleBond plus a sweep is rejected
    ///     at model build.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     This builder, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder has already been consumed.
    #[pyo3(text_signature = "($self, waterfall_spec)")]
    fn waterfall<'py>(
        mut slf: PyRefMut<'py, Self>,
        waterfall_spec: PyRef<'_, PyWaterfallSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let spec = waterfall_spec.inner.clone();
        let state = slf.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.waterfall(spec)),
            BuilderState::Ready(b) => BuilderState::Ready(b.waterfall(spec)),
        };
        slf.inner = Some(next);
        Ok(slf)
    }

    /// Build the model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     The completed model specification.
    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        let state = self.take_ready()?;
        let spec = state.build().map_err(statements_to_py)?;
        Ok(PyFinancialModelSpec { inner: spec })
    }
}

impl PyModelBuilder {
    fn take_any(&mut self) -> PyResult<BuilderState> {
        self.inner.take().ok_or_else(|| {
            crate::errors::value_error(
                "Builder is no longer usable: it was consumed by build()/mixed(), or a prior \
                 fallible call failed after taking ownership. Construct a new ModelBuilder.",
            )
        })
    }

    fn take_ready(&mut self) -> PyResult<ModelBuilder<finstack_quant_statements::builder::Ready>> {
        let state = self.take_any()?;
        match state {
            BuilderState::Ready(b) => Ok(b),
            BuilderState::NeedPeriods(b) => {
                self.inner = Some(BuilderState::NeedPeriods(b));
                Err(crate::errors::value_error(
                    "Must call periods() before adding nodes",
                ))
            }
        }
    }
}

/// Register builder classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModelBuilder>()?;
    m.add_class::<PyMixedNodeBuilder>()?;
    m.add_class::<PyMetricRegistry>()?;
    Ok(())
}

fn parse_scalar_values(values: Vec<(String, f64)>) -> PyResult<Vec<(PeriodId, AmountOrScalar)>> {
    values
        .into_iter()
        .map(|(p, v)| {
            let pid: PeriodId = p.parse().map_err(core_to_py)?;
            Ok((pid, AmountOrScalar::scalar(v)))
        })
        .collect()
}
