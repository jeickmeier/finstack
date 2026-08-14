//! Python wrappers for statement model types and enums.

use crate::errors::display_to_py;
use indexmap::IndexMap;
use pyo3::prelude::*;

/// Available forecast methods for projecting node values.
#[pyclass(
    name = "ForecastMethod",
    module = "finstack_quant.statements",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyForecastMethod {
    pub(super) inner: finstack_quant_statements::types::ForecastMethod,
}

#[pymethods]
impl PyForecastMethod {
    /// Carry the last observed value forward unchanged: ``v[t] = v[t-1]``.
    ///
    /// Takes no parameters and never changes the level, so it is the safe
    /// default for stock-like nodes (balances) that should hold flat when no
    /// explicit assumption is supplied.
    #[staticmethod]
    fn forward_fill() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::ForwardFill,
        }
    }

    /// Compound growth at a single rate: ``v[t] = v[t-1] * (1 + rate)``.
    ///
    /// ``rate`` is a **decimal fraction per period**, not a percentage — 0.05
    /// means 5% growth per period, and the period is whatever cadence the
    /// model's period list uses (quarters, months, years). Rates above 100%
    /// per period raise an evaluation warning.
    #[staticmethod]
    fn growth_pct() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::GrowthPct,
        }
    }

    /// Period-specific compound growth: ``v[t] = v[t-1] * (1 + curve[t])``.
    ///
    /// ``curve`` supplies one growth rate per forecast period, each a decimal
    /// fraction per period (0.05 = 5%), consumed in forecast-period order.
    #[staticmethod]
    fn curve_pct() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::CurvePct,
        }
    }

    /// Additive normal random walk:
    /// ``v[t] = v[t-1] + mean + std_dev * z[t]``.
    ///
    /// ``mean`` and ``std_dev`` are per-period **level** quantities in the
    /// node's own units (currency amounts for ``Money`` nodes, unitless for
    /// scalar nodes), not rates. ``z[t]`` is a deterministic standard-normal
    /// draw derived from the configured seed, so runs are reproducible.
    /// ``std_dev`` must be non-negative.
    #[staticmethod]
    fn normal() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::Normal,
        }
    }

    /// Multiplicative log-normal path:
    /// ``v[t] = v[t-1] * exp(mean - 0.5 * std_dev**2 + std_dev * z[t])``.
    ///
    /// ``mean`` and ``std_dev`` are the per-period log-return drift and
    /// volatility as decimal fractions (0.02 = 2% log drift per period). The
    /// ``-0.5 * std_dev**2`` term is the standard log-normal drift adjustment,
    /// so ``mean`` is the expected *log*-return, not the expected simple
    /// return. When the base value is zero the path falls back to independent
    /// ``exp(mean + std_dev * z[t])`` draws.
    #[staticmethod]
    fn log_normal() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::LogNormal,
        }
    }

    /// Explicit per-period overrides, forward-filling the periods in between.
    ///
    /// The method reads an ``overrides`` parameter mapping period-identifier
    /// strings (e.g. ``"2025Q2"``) to values; any forecast period without an
    /// entry inherits the most recent value. Named ``override_method`` on the
    /// Python side because the Rust variant is ``Override``.
    #[staticmethod]
    fn override_method() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::Override,
        }
    }

    /// Project an externally supplied historical series forward with trend
    /// detection.
    ///
    /// The history is passed through the spec's parameters rather than read
    /// from the model graph, so the same series can drive several nodes.
    #[staticmethod]
    fn time_series() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::TimeSeries,
        }
    }

    /// Seasonal decomposition forecast (additive or multiplicative).
    ///
    /// Requires a ``season_length`` parameter counted in **periods** (4 for a
    /// quarterly model, 12 for monthly) and at least two full seasonal cycles
    /// of history. Use additive mode when seasonal swings are constant in
    /// absolute terms and multiplicative when they scale with the level;
    /// prefer additive for series that cross zero.
    #[staticmethod]
    fn seasonal() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::Seasonal,
        }
    }

    /// Glide from the base value to a ``target`` level over the forecast
    /// horizon.
    ///
    /// Shapes (``shape`` parameter, default ``"linear"``): ``"linear"``
    /// reaches the target exactly at the final period; ``"geometric"`` is
    /// constant compound growth (CAGR-to-terminal, requires base and target
    /// non-zero with the same sign); ``"exponential"`` decays the remaining
    /// gap by half every ``half_life`` periods and never quite reaches the
    /// target. The workhorse for fading margins/ratios to a long-run
    /// assumption (NIM normalization, cost-income convergence, ROE fade).
    #[staticmethod]
    fn fade_to_target() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::FadeToTarget,
        }
    }

    /// Mean-reverting AR(1) path:
    /// ``v[t] = v[t-1] + reversion_speed * (long_run_mean - v[t-1]) + std_dev * z[t]``.
    ///
    /// ``long_run_mean`` and ``std_dev`` are in the node's own units;
    /// ``reversion_speed`` is the fraction of the gap closed each period, in
    /// ``(0, 1]``. ``z[t]`` is a deterministic standard-normal draw derived
    /// from the configured seed. Use for autocorrelated series that revert
    /// toward a through-the-cycle level (spreads, charge-off rates, NIM).
    #[staticmethod]
    fn mean_reverting() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::MeanReverting,
        }
    }

    /// Historical bootstrap: resample observed per-period changes with
    /// replacement (deterministic with seed).
    ///
    /// ``mode = "growth"`` (default) resamples period-over-period growth
    /// rates from a strictly positive ``historical`` series and compounds
    /// them; ``mode = "diff"`` resamples additive level changes and works
    /// for series that cross zero. Reproduces the empirical distribution of
    /// changes — including fat tails — without a normality assumption.
    #[staticmethod]
    fn bootstrap() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastMethod::Bootstrap,
        }
    }

    /// Return the debug representation, e.g. ``ForecastMethod(GrowthPct)``.
    fn __repr__(&self) -> String {
        format!("ForecastMethod({:?})", self.inner)
    }
}

/// Forecast configuration for a statement model node.
#[pyclass(
    name = "ForecastSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyForecastSpec {
    pub(super) inner: finstack_quant_statements::types::ForecastSpec,
}

#[pymethods]
impl PyForecastSpec {
    /// Build a forecast spec from a method plus its method-specific parameters.
    ///
    /// Parameters
    /// ----------
    /// method : ForecastMethod
    ///     Projection rule applied to forecast periods.
    /// params_json : str | None
    ///     JSON object of method-specific parameters (``rate``, ``curve``,
    ///     ``mean``, ``std_dev``, ``seed``, ``overrides``, ``season_length``,
    ///     ``target``, ``shape``, ``half_life``, ``long_run_mean``,
    ///     ``reversion_speed``, ``historical``, ``mode``, ``phi``, ...).
    ///     Rates are decimal fractions per period. Every method also accepts
    ///     optional ``min`` / ``max`` bounds that clamp generated values to a
    ///     band (in the node's own units). ``None`` means no parameters,
    ///     which is only valid for ``forward_fill``.
    #[new]
    #[pyo3(signature = (method, params_json=None), text_signature = "(method, params_json=None)")]
    fn new(method: PyRef<'_, PyForecastMethod>, params_json: Option<&str>) -> PyResult<Self> {
        let params = parse_params_json(params_json)?;
        Ok(Self {
            inner: finstack_quant_statements::types::ForecastSpec {
                method: method.inner,
                params,
            },
        })
    }

    /// Forward-fill spec: hold the last observed value flat, no parameters.
    #[staticmethod]
    fn forward_fill() -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::forward_fill(),
        }
    }

    /// Constant compound-growth spec: ``v[t] = v[t-1] * (1 + rate)``.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Growth rate as a **decimal fraction per period** (0.05 = 5% per
    ///     period, on the model's own period cadence). Negative values decay
    ///     the series.
    #[staticmethod]
    fn growth(rate: f64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::growth(rate),
        }
    }

    /// Period-by-period growth spec: ``v[t] = v[t-1] * (1 + curve[t])``.
    ///
    /// Parameters
    /// ----------
    /// curve : list[float]
    ///     One growth rate per forecast period, each a decimal fraction per
    ///     period (0.05 = 5%), consumed in forecast-period order.
    #[staticmethod]
    fn curve(curve: Vec<f64>) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::curve(curve),
        }
    }

    /// Additive normal random-walk spec:
    /// ``v[t] = v[t-1] + mean + std_dev * z[t]``.
    ///
    /// Parameters
    /// ----------
    /// mean : float
    ///     Per-period additive drift, in the node's own units (currency
    ///     amount for ``Money`` nodes, unitless for scalar nodes).
    /// std_dev : float
    ///     Per-period additive volatility in the same units; must be
    ///     non-negative. Zero gives a deterministic drift path.
    /// seed : int
    ///     Seed for the deterministic standard-normal draws. The evaluator
    ///     mixes a stable hash of the node id into it, so two nodes sharing a
    ///     seed still receive independent shocks.
    #[staticmethod]
    fn normal(mean: f64, std_dev: f64, seed: u64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::normal(mean, std_dev, seed),
        }
    }

    /// Multiplicative log-normal spec:
    /// ``v[t] = v[t-1] * exp(mean - 0.5 * std_dev**2 + std_dev * z[t])``.
    ///
    /// Parameters
    /// ----------
    /// mean : float
    ///     Per-period **log-return** drift as a decimal fraction (0.02 = 2%
    ///     log drift per period). The ``-0.5 * std_dev**2`` convexity term is
    ///     applied by the engine, so this is not the expected simple return.
    /// std_dev : float
    ///     Per-period log-return volatility as a decimal fraction; must be
    ///     non-negative.
    /// seed : int
    ///     Seed for the deterministic standard-normal draws.
    #[staticmethod]
    fn lognormal(mean: f64, std_dev: f64, seed: u64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::lognormal(mean, std_dev, seed),
        }
    }

    /// Linear fade-to-target spec:
    /// ``v[t] = base + (target - base) * t / N``.
    ///
    /// Values glide from the last observed value to ``target`` in equal
    /// steps, reaching it exactly at the final forecast period. For the
    /// ``"geometric"`` (CAGR-to-terminal) or ``"exponential"`` (half-life)
    /// shapes, construct ``ForecastSpec(ForecastMethod.fade_to_target(),
    /// params_json)`` with ``shape`` and, for exponential, ``half_life``.
    ///
    /// Parameters
    /// ----------
    /// target : float
    ///     Terminal level in the node's own units (a currency amount for
    ///     monetary nodes, a decimal ratio for scalar nodes such as margins).
    #[staticmethod]
    fn fade_to_target(target: f64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::fade_to_target(target),
        }
    }

    /// Mean-reverting AR(1) spec:
    /// ``v[t] = v[t-1] + reversion_speed * (long_run_mean - v[t-1]) + std_dev * z[t]``.
    ///
    /// Parameters
    /// ----------
    /// long_run_mean : float
    ///     Level the series reverts toward, in the node's own units
    ///     (currency amount for monetary nodes, decimal ratio for scalars).
    /// reversion_speed : float
    ///     Fraction of the gap closed each period, in ``(0, 1]``; 1.0
    ///     reverts fully every period, values near 0 revert slowly.
    /// std_dev : float
    ///     Per-period additive shock volatility in the node's own units;
    ///     must be non-negative. Zero gives deterministic geometric decay of
    ///     the gap.
    /// seed : int
    ///     Seed for the deterministic standard-normal draws. The evaluator
    ///     mixes a stable hash of the node id into it, so two nodes sharing
    ///     a seed still receive independent shocks.
    #[staticmethod]
    fn mean_reverting(long_run_mean: f64, reversion_speed: f64, std_dev: f64, seed: u64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::mean_reverting(
                long_run_mean,
                reversion_speed,
                std_dev,
                seed,
            ),
        }
    }

    /// Growth-mode bootstrap spec: resample historical growth rates and
    /// compound them from the base value.
    ///
    /// The history must be strictly positive in growth mode. For additive
    /// resampling of level changes (series that cross zero), construct
    /// ``ForecastSpec(ForecastMethod.bootstrap(), params_json)`` with
    /// ``mode = "diff"``.
    ///
    /// Parameters
    /// ----------
    /// historical : list[float]
    ///     At least 2 historical values in the node's own units, oldest
    ///     first; consecutive pairs define the resampled growth rates.
    /// seed : int
    ///     Seed for the deterministic resampling draws.
    #[staticmethod]
    fn bootstrap(historical: Vec<f64>, seed: u64) -> Self {
        Self {
            inner: finstack_quant_statements::types::ForecastSpec::bootstrap(historical, seed),
        }
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a forecast spec from its canonical JSON form.
    ///
    /// Unknown fields are rejected, so a typo in ``method`` or ``params``
    /// fails loudly rather than being silently dropped.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this forecast spec to canonical JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Return the debug representation with the method and parameter count.
    fn __repr__(&self) -> String {
        format!(
            "ForecastSpec(method={:?}, params={})",
            self.inner.method,
            self.inner.params.len()
        )
    }
}

fn parse_params_json(params_json: Option<&str>) -> PyResult<IndexMap<String, serde_json::Value>> {
    match params_json {
        Some(json) => serde_json::from_str(json).map_err(display_to_py),
        None => Ok(IndexMap::new()),
    }
}

/// Node computation type.
#[pyclass(
    name = "NodeType",
    module = "finstack_quant.statements",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyNodeType {
    pub(super) inner: finstack_quant_statements::types::NodeType,
}

#[pymethods]
impl PyNodeType {
    /// Value node: explicit per-period values only (actuals, assumptions).
    ///
    /// The node has no formula and no forecast, so periods without an
    /// explicit value stay unset.
    #[staticmethod]
    fn value() -> Self {
        Self {
            inner: finstack_quant_statements::types::NodeType::Value,
        }
    }

    /// Calculated node: derived from its formula in every period.
    #[staticmethod]
    fn calculated() -> Self {
        Self {
            inner: finstack_quant_statements::types::NodeType::Calculated,
        }
    }

    /// Mixed node: explicit value, else forecast, else formula.
    ///
    /// This is the crate's core precedence invariant — **Value > Forecast >
    /// Formula** — resolved independently for each period, so a node can be
    /// an actual in historical periods, a forecast in projected periods, and
    /// a formula wherever neither is supplied.
    #[staticmethod]
    fn mixed() -> Self {
        Self {
            inner: finstack_quant_statements::types::NodeType::Mixed,
        }
    }

    /// The canonical snake_case wire discriminant (``"value"``,
    /// ``"calculated"``, or ``"mixed"``), derived from the Rust serde
    /// rename so the string can never drift from the JSON schema.
    #[getter]
    fn kind(&self) -> String {
        crate::bindings::statements_analytics::serde_variant_str(&self.inner)
    }

    /// Return the canonical snake_case discriminant (same as :attr:`kind`).
    fn __str__(&self) -> String {
        self.kind()
    }

    /// Return the debug representation, e.g. ``NodeType(Mixed)``.
    fn __repr__(&self) -> String {
        format!("NodeType({:?})", self.inner)
    }
}

/// Type-safe identifier for a node in a financial model.
#[pyclass(
    name = "NodeId",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyNodeId {
    pub(super) inner: finstack_quant_statements::types::NodeId,
}

#[pymethods]
impl PyNodeId {
    /// Wrap a raw node identifier string.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Node identifier as it appears in the model graph and in formulas
    ///     (e.g. ``"revenue"``). The value is stored verbatim — no case
    ///     folding or trimming — so identifiers are matched exactly.
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn new(id: &str) -> Self {
        Self {
            inner: finstack_quant_statements::types::NodeId::new(id),
        }
    }

    /// Return the underlying identifier string.
    ///
    /// Returns
    /// -------
    /// str
    ///     The identifier exactly as supplied at construction.
    #[pyo3(text_signature = "($self)")]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Return the debug representation, e.g. ``NodeId("revenue")``.
    fn __repr__(&self) -> String {
        format!("NodeId({:?})", self.inner.as_str())
    }

    /// Return the bare identifier string (no quoting or wrapper).
    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Numeric evaluation mode.
#[pyclass(
    name = "NumericMode",
    module = "finstack_quant.statements",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyNumericMode {
    pub(super) inner: finstack_quant_statements::evaluator::NumericMode,
}

#[pymethods]
impl PyNumericMode {
    /// f64 floating-point evaluation — the mode the evaluator actually emits.
    ///
    /// Statement arithmetic runs in f64, so identity checks carry rounding
    /// proportional to the magnitudes involved; that is why check tolerances
    /// blend an absolute floor with a relative component.
    #[staticmethod]
    fn float64() -> Self {
        Self {
            inner: finstack_quant_statements::evaluator::NumericMode::Float64,
        }
    }

    /// Return the debug representation, e.g. ``NumericMode(Float64)``.
    fn __repr__(&self) -> String {
        format!("NumericMode({:?})", self.inner)
    }
}

/// Top-level financial model specification.
#[pyclass(
    name = "FinancialModelSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFinancialModelSpec {
    pub(crate) inner: finstack_quant_statements::FinancialModelSpec,
}

#[pymethods]
impl PyFinancialModelSpec {
    /// Start a staged model build.
    ///
    /// The canonical entry point, mirroring Rust's
    /// `FinancialModelSpec::builder(id)` and the ``Type.builder()`` form every
    /// other builder-backed type uses. Constructing
    /// :class:`~finstack_quant.statements.ModelBuilder` directly is equivalent.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Stable model identifier.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     A fresh builder awaiting ``periods(...)``.
    #[staticmethod]
    #[pyo3(text_signature = "(id)")]
    fn builder(id: &str) -> crate::bindings::statements::builder::PyModelBuilder {
        crate::bindings::statements::builder::PyModelBuilder::start(id)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let mut inner: finstack_quant_statements::FinancialModelSpec =
            serde_json::from_str(json).map_err(display_to_py)?;
        inner.validate_semantics().map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Unique model identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Number of periods in the model timeline.
    ///
    /// Counted in **periods** on the model's own cadence (quarters, months,
    /// years), not months. Their declared order *is* the evaluation timeline.
    #[getter]
    fn period_count(&self) -> usize {
        self.inner.periods.len()
    }

    /// Number of nodes (line items / metrics) declared in the model.
    #[getter]
    fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    /// Node identifiers in declaration order.
    #[pyo3(text_signature = "($self)")]
    fn node_ids(&self) -> Vec<String> {
        self.inner.nodes.keys().map(|k| k.to_string()).collect()
    }

    /// Whether the model has a node with the given ID.
    #[pyo3(text_signature = "($self, node_id)")]
    fn has_node(&self, node_id: &str) -> bool {
        self.inner.has_node(node_id)
    }

    /// Wire-format schema version of this model spec.
    ///
    /// Only version ``1`` is accepted today; the field exists so persisted
    /// models can be migrated rather than silently misread.
    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version.into()
    }

    /// Return the debug representation with the id, period and node counts.
    fn __repr__(&self) -> String {
        format!(
            "FinancialModelSpec(id={:?}, periods={}, nodes={})",
            self.inner.id,
            self.inner.periods.len(),
            self.inner.nodes.len()
        )
    }
}

/// Register type classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyForecastMethod>()?;
    m.add_class::<PyForecastSpec>()?;
    m.add_class::<PyNodeType>()?;
    m.add_class::<PyNodeId>()?;
    m.add_class::<PyNumericMode>()?;
    m.add_class::<PyFinancialModelSpec>()?;
    Ok(())
}
