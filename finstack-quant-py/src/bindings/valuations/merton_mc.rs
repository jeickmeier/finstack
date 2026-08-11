//! Python bindings for Merton Monte Carlo PIK bond pricing types.

use crate::bindings::valuations::credit::{
    PyDynamicRecoverySpec, PyEndogenousHazardSpec, PyMertonModel, PyToggleExerciseModel,
};
use crate::errors::display_to_py;
use finstack_quant_valuations::instruments::fixed_income::bond::pricing::engine::merton_mc::{
    BarrierCrossing, MertonMcConfig, MertonMcResult, PathStatistics, PikMode, PikSchedule,
};
use pyo3::prelude::*;
use pyo3::types::PyList;

// PikMode

/// Per-coupon PIK behavior for the Merton Monte Carlo engine.
#[pyclass(
    name = "PikMode",
    module = "finstack_quant.valuations.instruments",
    skip_from_py_object
)]
#[derive(Clone, Copy)]
pub(crate) struct PyPikMode {
    pub(crate) inner: PikMode,
}

impl PyPikMode {
    pub(crate) fn from_inner(inner: PikMode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPikMode {
    /// Coupon paid entirely in cash.
    #[staticmethod]
    fn cash() -> Self {
        Self {
            inner: PikMode::Cash,
        }
    }

    /// Coupon accreted to notional (payment-in-kind).
    #[staticmethod]
    fn pik() -> Self {
        Self {
            inner: PikMode::Pik,
        }
    }

    /// Coupon split between cash and PIK accretion.
    ///
    /// # Arguments
    ///
    /// * `cash_fraction` - Fraction paid in cash as a decimal (e.g. ``0.5`` for 50%).
    /// * `pik_fraction` - Fraction accreted to notional as a decimal.
    #[staticmethod]
    fn split(cash_fraction: f64, pik_fraction: f64) -> Self {
        Self {
            inner: PikMode::Split {
                cash_fraction,
                pik_fraction,
            },
        }
    }

    /// Defer PIK/cash decision to the toggle exercise model on the config.
    #[staticmethod]
    fn toggle() -> Self {
        Self {
            inner: PikMode::Toggle,
        }
    }

    /// Deserialize a PIK mode from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this PIK mode to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

// PikSchedule

/// Time-varying PIK schedule for the Merton Monte Carlo engine.
#[pyclass(
    name = "PikSchedule",
    module = "finstack_quant.valuations.instruments",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyPikSchedule {
    pub(crate) inner: PikSchedule,
}

#[pymethods]
impl PyPikSchedule {
    /// Apply the same PIK mode at every coupon date.
    ///
    /// # Arguments
    ///
    /// * `mode` - PIK mode applied uniformly across the schedule.
    #[staticmethod]
    fn uniform(mode: PyRef<'_, PyPikMode>) -> Self {
        Self {
            inner: PikSchedule::Uniform(mode.inner),
        }
    }

    /// Step-function PIK schedule keyed by year fraction.
    ///
    /// Each ``(t, mode)`` pair applies ``mode`` from time ``t`` onward.
    /// Entries must be sorted by ``t`` ascending.
    ///
    /// # Arguments
    ///
    /// * `steps` - List of ``(year_fraction, PikMode)`` pairs.
    #[staticmethod]
    fn stepped(steps: &Bound<'_, PyAny>) -> PyResult<Self> {
        let list = steps.cast::<PyList>()?;
        let mut out = Vec::with_capacity(list.len());
        for item in list.iter() {
            let (t, mode): (f64, PyRef<'_, PyPikMode>) = item.extract()?;
            out.push((t, mode.inner));
        }
        Ok(Self {
            inner: PikSchedule::Stepped(out),
        })
    }

    /// Look up the active PIK mode at time ``t`` (year fraction from valuation date).
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years from the valuation date.
    fn mode_at(&self, t: f64) -> PyPikMode {
        PyPikMode::from_inner(self.inner.mode_at(t))
    }

    /// Deserialize a PIK schedule from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this PIK schedule to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

// BarrierCrossing

/// Barrier-crossing detection policy for first-passage default simulation.
#[pyclass(
    name = "BarrierCrossing",
    module = "finstack_quant.valuations.instruments",
    skip_from_py_object
)]
#[derive(Clone, Copy)]
pub(crate) struct PyBarrierCrossing {
    pub(crate) inner: BarrierCrossing,
}

#[pymethods]
impl PyBarrierCrossing {
    /// Discrete monitoring: default if asset value is below the barrier at grid points.
    #[staticmethod]
    fn discrete() -> Self {
        Self {
            inner: BarrierCrossing::Discrete,
        }
    }

    /// Brownian-bridge correction for continuous barrier monitoring between grid points.
    #[staticmethod]
    fn brownian_bridge() -> Self {
        Self {
            inner: BarrierCrossing::BrownianBridge,
        }
    }

    /// Deserialize a barrier-crossing policy from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this barrier-crossing policy to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

// PathStatistics

/// Path-level statistics from a Merton Monte Carlo simulation.
#[pyclass(
    name = "PathStatistics",
    module = "finstack_quant.valuations.instruments",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyPathStatistics {
    pub(crate) inner: PathStatistics,
}

impl PyPathStatistics {
    pub(crate) fn from_inner(inner: PathStatistics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPathStatistics {
    /// Fraction of simulated paths that defaulted.
    #[getter]
    fn default_rate(&self) -> f64 {
        self.inner.default_rate
    }

    /// Average default time in years among defaulted paths.
    #[getter]
    fn avg_default_time(&self) -> f64 {
        self.inner.avg_default_time
    }

    /// Average terminal notional, reflecting PIK accretion.
    #[getter]
    fn avg_terminal_notional(&self) -> f64 {
        self.inner.avg_terminal_notional
    }

    /// Average recovery percentage among defaulted paths.
    #[getter]
    fn avg_recovery_pct(&self) -> f64 {
        self.inner.avg_recovery_pct
    }

    /// Fraction of coupon dates where PIK was elected.
    #[getter]
    fn pik_exercise_rate(&self) -> f64 {
        self.inner.pik_exercise_rate
    }
}

// MertonMcResult

/// Result from Merton Monte Carlo PIK bond pricing.
#[pyclass(
    name = "MertonMcResult",
    module = "finstack_quant.valuations.instruments",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyMertonMcResult {
    pub(crate) inner: MertonMcResult,
}

impl PyMertonMcResult {
    pub(crate) fn from_inner(inner: MertonMcResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMertonMcResult {
    /// Clean price as a percentage of par.
    #[getter]
    fn clean_price_pct(&self) -> f64 {
        self.inner.clean_price_pct
    }

    /// Dirty price as a percentage of par.
    #[getter]
    fn dirty_price_pct(&self) -> f64 {
        self.inner.dirty_price_pct
    }

    /// Expected loss as a fraction of PIK-aware risk-free present value.
    #[getter]
    fn expected_loss(&self) -> f64 {
        self.inner.expected_loss
    }

    /// Unexpected loss (standard deviation of path PVs divided by notional).
    #[getter]
    fn unexpected_loss(&self) -> f64 {
        self.inner.unexpected_loss
    }

    /// Expected shortfall at the 95% confidence level.
    #[getter]
    fn expected_shortfall_95(&self) -> f64 {
        self.inner.expected_shortfall_95
    }

    /// Average PIK fraction across all coupon dates and paths.
    #[getter]
    fn average_pik_fraction(&self) -> f64 {
        self.inner.average_pik_fraction
    }

    /// Effective spread in basis points implied by the MC price versus risk-free PV.
    #[getter]
    fn effective_spread_bp(&self) -> f64 {
        self.inner.effective_spread_bp
    }

    /// Path-level simulation statistics.
    #[getter]
    fn path_statistics(&self) -> PyPathStatistics {
        PyPathStatistics::from_inner(self.inner.path_statistics.clone())
    }

    /// Number of Monte Carlo paths used.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Standard error of the clean price estimate (percentage of par).
    #[getter]
    fn standard_error(&self) -> f64 {
        self.inner.standard_error
    }
}

// MertonMcConfig

/// Configuration for Merton Monte Carlo PIK bond pricing.
#[pyclass(
    name = "MertonMcConfig",
    module = "finstack_quant.valuations.instruments",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyMertonMcConfig {
    pub(crate) inner: MertonMcConfig,
}

#[pymethods]
impl PyMertonMcConfig {
    /// Create a Merton MC configuration with registry-sourced simulation defaults.
    ///
    /// # Arguments
    ///
    /// * `merton` - Structural credit model driving asset dynamics and default.
    #[new]
    fn new(merton: PyRef<'_, PyMertonModel>) -> Self {
        Self {
            inner: MertonMcConfig::new(merton.inner.clone()),
        }
    }

    /// Set the PIK schedule controlling per-coupon cash/PIK/toggle behavior.
    ///
    /// # Arguments
    ///
    /// * `s` - PIK schedule applied across coupon dates.
    fn pik_schedule(&self, s: PyRef<'_, PyPikSchedule>) -> Self {
        Self {
            inner: self.inner.clone().pik_schedule(s.inner.clone()),
        }
    }

    /// Set the number of Monte Carlo paths.
    ///
    /// # Arguments
    ///
    /// * `n` - Path count (must be at least 2 for meaningful statistics).
    fn num_paths(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().num_paths(n),
        }
    }

    /// Set the RNG seed for reproducibility.
    ///
    /// # Arguments
    ///
    /// * `s` - Unsigned 64-bit seed passed to the path generator.
    fn seed(&self, s: u64) -> Self {
        Self {
            inner: self.inner.clone().seed(s),
        }
    }

    /// Enable or disable antithetic variates for variance reduction.
    ///
    /// # Arguments
    ///
    /// * `a` - When ``True``, pair each path with its antithetic counterpart.
    fn antithetic(&self, a: bool) -> Self {
        Self {
            inner: self.inner.clone().antithetic(a),
        }
    }

    /// Set the number of time steps simulated per year.
    ///
    /// # Arguments
    ///
    /// * `n` - Grid density for asset evolution and barrier monitoring.
    fn time_steps_per_year(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().time_steps_per_year(n),
        }
    }

    /// Set the barrier-crossing policy for first-passage default monitoring.
    ///
    /// # Arguments
    ///
    /// * `p` - Discrete grid checks or Brownian-bridge continuous correction.
    fn barrier_crossing(&self, p: PyRef<'_, PyBarrierCrossing>) -> Self {
        Self {
            inner: self.inner.clone().barrier_crossing(p.inner),
        }
    }

    /// Set the flat recovery rate used when no dynamic recovery model is configured.
    ///
    /// # Arguments
    ///
    /// * `r` - Recovery rate as a decimal in ``[0, 1]``.
    fn default_recovery_rate(&self, r: f64) -> Self {
        let mut inner = self.inner.clone();
        inner.default_recovery_rate = r;
        Self { inner }
    }

    /// Set an endogenous (leverage-dependent) hazard rate model.
    ///
    /// # Arguments
    ///
    /// * `h` - Endogenous hazard specification applied pathwise at default.
    fn endogenous_hazard(&self, h: PyRef<'_, PyEndogenousHazardSpec>) -> Self {
        Self {
            inner: self.inner.clone().endogenous_hazard(h.inner.clone()),
        }
    }

    /// Set a dynamic (notional-dependent) recovery rate model.
    ///
    /// # Arguments
    ///
    /// * `r` - Dynamic recovery specification evaluated at accreted notional.
    fn dynamic_recovery(&self, r: PyRef<'_, PyDynamicRecoverySpec>) -> Self {
        Self {
            inner: self.inner.clone().dynamic_recovery(r.inner.clone()),
        }
    }

    /// Set the toggle exercise model for PIK/cash coupon decisions.
    ///
    /// Active only on coupon dates where the PIK schedule resolves to
    /// ``PikMode.toggle()``.
    ///
    /// # Arguments
    ///
    /// * `t` - Toggle exercise model deciding cash versus PIK at each toggle date.
    fn toggle_model(&self, t: PyRef<'_, PyToggleExerciseModel>) -> Self {
        Self {
            inner: self.inner.clone().toggle_model(t.inner.clone()),
        }
    }

    /// Deserialize a Merton MC configuration from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this configuration to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

pub(crate) const EXPORTS: &[&str] = &[
    "BarrierCrossing",
    "MertonMcConfig",
    "MertonMcResult",
    "PathStatistics",
    "PikMode",
    "PikSchedule",
];

/// Register Merton MC types on the instruments submodule.
pub(crate) fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBarrierCrossing>()?;
    m.add_class::<PyMertonMcConfig>()?;
    m.add_class::<PyMertonMcResult>()?;
    m.add_class::<PyPathStatistics>()?;
    m.add_class::<PyPikMode>()?;
    m.add_class::<PyPikSchedule>()?;
    Ok(())
}
