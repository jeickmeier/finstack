//! Python bindings for structural credit model specifications.

mod lgd;
mod liability_management;
mod migration;
mod pd;
mod recovery_waterfall;
mod scoring;

use std::sync::Arc;

use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::market_data::curves::helpers::parse_day_count;
use crate::bindings::core::market_data::curves::PyHazardCurve;
use crate::bindings::core::types::PyCreditRating;
use crate::bindings::pandas_utils::{serde_object_to_single_row_dataframe, serde_to_py};
use crate::errors::display_to_py;
use finstack_quant_core::math::random::Pcg64Rng;
use finstack_quant_models::credit::{
    moodys_warf_factor as rust_moodys_warf_factor, AssetDynamics, BarrierType, CreditState,
    CreditStateVariable, DynamicRecoverySpec, EndogenousHazardSpec, MertonModel, OptimalToggle,
    SimulatedPaths, ThresholdDirection, ToggleExerciseModel,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Return the Moody's WARF factor for a canonical credit rating.
///
/// # Arguments
///
/// * `rating` - Canonical `core.types.CreditRating` whose exact-notch Moody's
///   rating factor is required.
///
/// # Errors
///
/// Raises `ValueError` if the embedded credit-assumptions registry is invalid
/// or the rating has no factor in the configured Moody's table.
#[pyfunction]
fn moodys_warf_factor(rating: &PyCreditRating) -> PyResult<f64> {
    rust_moodys_warf_factor(rating.inner).map_err(display_to_py)
}

#[pyclass(
    name = "BarrierType",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone, Copy)]
pub(crate) struct PyBarrierType {
    pub(crate) inner: BarrierType,
}

impl PyBarrierType {
    pub(crate) fn from_inner(inner: BarrierType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBarrierType {
    /// Classic Merton barrier tested only at maturity.
    #[staticmethod]
    fn terminal() -> Self {
        Self {
            inner: BarrierType::Terminal,
        }
    }

    /// Black-Cox first-passage barrier with optional growth rate.
    ///
    /// # Arguments
    ///
    /// * `barrier_growth_rate` - Continuous growth rate of the default barrier
    ///   over time, as a decimal (e.g. ``0.02`` for 2% annual growth).
    #[staticmethod]
    fn first_passage(barrier_growth_rate: f64) -> Self {
        Self {
            inner: BarrierType::FirstPassage {
                barrier_growth_rate,
            },
        }
    }

    /// Deserialize a barrier type from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this barrier type to compact JSON.
    #[allow(clippy::wrong_self_convention)]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

#[pyclass(
    name = "AssetDynamics",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone, Copy)]
pub(crate) struct PyAssetDynamics {
    pub(crate) inner: AssetDynamics,
}

impl PyAssetDynamics {
    pub(crate) fn from_inner(inner: AssetDynamics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAssetDynamics {
    /// Standard geometric Brownian motion (lognormal diffusion).
    #[staticmethod]
    fn geometric_brownian() -> Self {
        Self {
            inner: AssetDynamics::GeometricBrownian,
        }
    }

    /// Merton jump-diffusion asset dynamics.
    ///
    /// # Arguments
    ///
    /// * `jump_intensity` - Poisson jump arrival intensity (jumps per year).
    /// * `jump_mean` - Mean log-jump size.
    /// * `jump_vol` - Volatility of log-jump size.
    #[staticmethod]
    fn jump_diffusion(jump_intensity: f64, jump_mean: f64, jump_vol: f64) -> Self {
        Self {
            inner: AssetDynamics::JumpDiffusion {
                jump_intensity,
                jump_mean,
                jump_vol,
            },
        }
    }

    /// CreditGrades stochastic-barrier dynamics.
    ///
    /// # Arguments
    ///
    /// * `barrier_uncertainty` - Log-barrier volatility ``lambda`` (lognormal
    ///   standard deviation of the default barrier).
    /// * `mean_recovery` - Mean recovery rate at default, as a decimal in
    ///   ``[0, 1]``.
    #[staticmethod]
    fn credit_grades(barrier_uncertainty: f64, mean_recovery: f64) -> Self {
        Self {
            inner: AssetDynamics::CreditGrades {
                barrier_uncertainty,
                mean_recovery,
            },
        }
    }

    /// Deserialize asset dynamics from canonical JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize these asset dynamics to compact JSON.
    #[allow(clippy::wrong_self_convention)]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }
}

#[pyclass(
    name = "MertonModel",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyMertonModel {
    pub(crate) inner: MertonModel,
}

#[pymethods]
impl PyMertonModel {
    /// Construct a Merton structural credit model from firm asset inputs.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Firm asset value (positive, finite)
    /// * `asset_vol` - Annualized asset volatility as a decimal
    /// * `debt_barrier` - Default barrier, typically total debt face
    /// * `risk_free_rate` - Continuously compounded risk-free rate as a decimal
    ///
    /// # Errors
    ///
    /// Raises `ValueError` when inputs are non-finite or out of range.
    #[new]
    fn new(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::new(asset_value, asset_vol, debt_barrier, risk_free_rate)
                .map_err(display_to_py)?,
        })
    }

    /// KMV calibration: recover asset value and volatility from equity inputs.
    ///
    /// # Arguments
    ///
    /// * `equity_value` - Observed market equity value
    /// * `equity_vol` - Observed equity volatility as a decimal
    /// * `total_debt` - Face value of debt used as the default barrier
    /// * `risk_free_rate` - Continuously compounded risk-free rate as a decimal
    /// * `payout_rate` - Continuous dividend / payout yield on assets as a decimal
    /// * `maturity` - Calibration horizon in years
    #[staticmethod]
    fn from_equity(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        maturity: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::from_equity(
                equity_value,
                equity_vol,
                total_debt,
                risk_free_rate,
                payout_rate,
                maturity,
            )
            .map_err(display_to_py)?,
        })
    }

    /// CDS spread calibration: find asset volatility matching a target spread.
    ///
    /// # Arguments
    ///
    /// * `cds_spread_bp` - Target CDS par spread in basis points
    /// * `recovery` - Assumed recovery rate as a decimal in ``[0, 1]``
    /// * `total_debt` - Face value of debt
    /// * `risk_free_rate` - Continuously compounded risk-free rate as a decimal
    /// * `maturity` - Calibration horizon in years
    /// * `asset_value` - Assumed initial firm asset value
    /// * `payout_rate` - Continuous payout rate on assets as a decimal
    #[staticmethod]
    fn from_cds_spread(
        cds_spread_bp: f64,
        recovery: f64,
        total_debt: f64,
        risk_free_rate: f64,
        maturity: f64,
        asset_value: f64,
        payout_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::from_cds_spread(
                cds_spread_bp,
                recovery,
                total_debt,
                risk_free_rate,
                maturity,
                asset_value,
                payout_rate,
            )
            .map_err(display_to_py)?,
        })
    }

    /// Calibrate the debt barrier to match a target cumulative default probability.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current firm asset value, strictly positive
    /// * `asset_vol` - Annualized asset volatility as a decimal, strictly
    ///   positive
    /// * `risk_free_rate` - Continuously compounded risk-free rate as a
    ///   decimal. Pass the expected physical asset return instead to
    ///   calibrate against a real-world default rate
    /// * `payout_rate` - Continuous payout rate on assets as a decimal; it
    ///   enters the calibration drift and is carried on the returned model
    /// * `target_pd` - Target cumulative default probability in ``(0, 1)``
    /// * `maturity` - Calibration horizon in years, strictly positive
    #[staticmethod]
    fn from_target_pd(
        asset_value: f64,
        asset_vol: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        target_pd: f64,
        maturity: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::from_target_pd(
                asset_value,
                asset_vol,
                risk_free_rate,
                payout_rate,
                target_pd,
                maturity,
            )
            .map_err(display_to_py)?,
        })
    }

    /// Moody's KMV default point: short-term debt plus half of long-term debt.
    ///
    /// # Arguments
    ///
    /// * `short_term_debt` - Liabilities due within one year, non-negative
    /// * `long_term_debt` - Liabilities maturing beyond one year,
    ///   non-negative; half of it enters the default point
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when either input is negative or non-finite, or
    /// when the resulting default point is zero.
    #[staticmethod]
    fn kmv_default_point(short_term_debt: f64, long_term_debt: f64) -> PyResult<f64> {
        MertonModel::kmv_default_point(short_term_debt, long_term_debt).map_err(display_to_py)
    }

    /// Construct a Merton model with explicit barrier and dynamics specifications.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current firm asset value
    /// * `asset_vol` - Asset volatility as a decimal
    /// * `debt_barrier` - Default barrier level
    /// * `risk_free_rate` - Continuously compounded risk-free rate as a decimal
    /// * `payout_rate` - Continuous payout rate on assets as a decimal
    /// * `barrier_type` - Terminal or first-passage barrier monitoring
    /// * `dynamics` - Asset return dynamics specification
    #[staticmethod]
    #[allow(clippy::too_many_arguments)]
    fn new_with_dynamics(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        barrier_type: PyRef<'_, PyBarrierType>,
        dynamics: PyRef<'_, PyAssetDynamics>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::new_with_dynamics(
                asset_value,
                asset_vol,
                debt_barrier,
                risk_free_rate,
                payout_rate,
                barrier_type.inner,
                dynamics.inner,
            )
            .map_err(display_to_py)?,
        })
    }

    /// Build a CreditGrades-style structural model from equity inputs.
    ///
    /// Calibrates asset value and volatility implied by observed equity value
    /// and equity volatility under the O'Kane CreditGrades barrier specification.
    #[staticmethod]
    fn credit_grades(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        barrier_uncertainty: f64,
        mean_recovery: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: MertonModel::credit_grades(
                equity_value,
                equity_vol,
                total_debt,
                risk_free_rate,
                barrier_uncertainty,
                mean_recovery,
            )
            .map_err(display_to_py)?,
        })
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

    /// Deserialize a structural credit model from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    /// Serialize this model to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Current firm asset value ``V_0``, in the issuer's reporting currency.
    #[getter]
    fn asset_value(&self) -> f64 {
        self.inner.asset_value()
    }

    /// Annualized asset volatility ``sigma_V`` as a decimal (``0.25`` is 25%).
    #[getter]
    fn asset_vol(&self) -> f64 {
        self.inner.asset_vol()
    }

    /// Default barrier ``B``, in the same currency as ``asset_value``.
    #[getter]
    fn debt_barrier(&self) -> f64 {
        self.inner.debt_barrier()
    }

    /// Continuously compounded risk-free rate ``r`` as a decimal.
    #[getter]
    fn risk_free_rate(&self) -> f64 {
        self.inner.risk_free_rate()
    }

    /// Continuous payout (dividend) rate ``q`` on assets, as a decimal.
    #[getter]
    fn payout_rate(&self) -> f64 {
        self.inner.payout_rate()
    }

    /// Barrier monitoring convention.
    #[getter]
    fn barrier_type(&self) -> PyBarrierType {
        PyBarrierType::from_inner(*self.inner.barrier_type())
    }

    /// Asset return dynamics specification.
    #[getter]
    fn dynamics(&self) -> PyAssetDynamics {
        PyAssetDynamics::from_inner(*self.inner.dynamics())
    }

    /// Export the model parameters as a single-row :class:`pandas.DataFrame`.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe(py, &self.inner)
    }

    /// Risk-neutral distance to default at `horizon` years.
    fn distance_to_default(&self, horizon: f64) -> f64 {
        self.inner.distance_to_default(horizon)
    }

    /// Physical-measure (Moody's KMV) distance to default at `horizon` years.
    ///
    /// # Arguments
    ///
    /// * `asset_drift` - Expected physical total return on the firm's assets
    ///   as a continuously compounded decimal, replacing the risk-free rate
    /// * `horizon` - Time horizon in years
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when `asset_drift` is not finite or the model
    /// uses driftless CreditGrades dynamics.
    fn distance_to_default_with_drift(&self, asset_drift: f64, horizon: f64) -> PyResult<f64> {
        self.inner
            .distance_to_default_with_drift(asset_drift, horizon)
            .map_err(display_to_py)
    }

    /// Risk-neutral default probability over `horizon` years.
    fn default_probability(&self, horizon: f64) -> f64 {
        self.inner.default_probability(horizon)
    }

    /// Physical-measure default probability (theoretical EDF) over `horizon` years.
    ///
    /// # Arguments
    ///
    /// * `asset_drift` - Expected physical total return on the firm's assets
    ///   as a continuously compounded decimal, replacing the risk-free rate
    /// * `horizon` - Time horizon in years
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when `asset_drift` is not finite or the model
    /// uses driftless CreditGrades dynamics.
    fn default_probability_with_drift(&self, asset_drift: f64, horizon: f64) -> PyResult<f64> {
        self.inner
            .default_probability_with_drift(asset_drift, horizon)
            .map_err(display_to_py)
    }

    /// Zero-coupon bond spread with exogenous `recovery` (decimal, not bp).
    ///
    /// # Errors
    ///
    /// Raises `ValueError` when `horizon` or `recovery` are invalid.
    fn implied_spread(&self, horizon: f64, recovery: f64) -> PyResult<f64> {
        self.inner
            .implied_spread(horizon, recovery)
            .map_err(display_to_py)
    }

    /// Merton (1974) endogenous debt spread at `horizon` years (decimal, not bp).
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when `horizon` is not positive, the barrier type
    /// is not terminal, or the implied debt value is non-positive.
    fn debt_spread(&self, horizon: f64) -> PyResult<f64> {
        self.inner.debt_spread(horizon).map_err(display_to_py)
    }

    /// ISDA-style CDS par spread at `maturity` years (decimal, not bp).
    ///
    /// # Arguments
    ///
    /// * `maturity` - CDS maturity in years, strictly positive
    /// * `recovery` - Recovery rate as a decimal in ``[0, 1]``; must equal the
    ///   model's ``mean_recovery`` under CreditGrades dynamics
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when the inputs are out of range or the implied
    /// survival curve cannot be bootstrapped.
    fn cds_par_spread(&self, maturity: f64, recovery: f64) -> PyResult<f64> {
        self.inner
            .cds_par_spread(maturity, recovery)
            .map_err(display_to_py)
    }

    /// Implied equity value and equity volatility at ``horizon`` years.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Time horizon in years (must be positive and finite)
    ///
    /// # Errors
    ///
    /// Raises ``ValueError`` when the firm is economically in default or the
    /// inversion is numerically ill-conditioned.
    fn try_implied_equity(&self, horizon: f64) -> PyResult<(f64, f64)> {
        self.inner
            .try_implied_equity(horizon)
            .map_err(display_to_py)
    }

    /// Bootstrap a piecewise-constant hazard curve from structural default probabilities.
    ///
    /// # Arguments
    ///
    /// * `id` - Curve identifier
    /// * `base_date` - Valuation date for the curve
    /// * `tenors` - Tenor grid in years (non-empty, strictly positive, distinct)
    /// * `recovery` - Recovery rate assumption as a decimal in ``[0, 1]``;
    ///   must equal the model's ``mean_recovery`` under CreditGrades dynamics
    /// * `day_count` - Day-count convention the curve uses to turn dates into
    ///   year fractions (default ``"act_365f"``)
    #[pyo3(signature = (id, base_date, tenors, recovery, day_count="act_365f"))]
    fn to_hazard_curve(
        &self,
        id: &str,
        base_date: &Bound<'_, PyAny>,
        tenors: Vec<f64>,
        recovery: f64,
        day_count: &str,
    ) -> PyResult<PyHazardCurve> {
        let base_date = py_to_date(base_date)?;
        let day_count = parse_day_count(day_count)?;
        let curve = self
            .inner
            .to_hazard_curve(id, base_date, &tenors, recovery, day_count)
            .map_err(display_to_py)?;
        Ok(PyHazardCurve::from_inner(Arc::new(curve)))
    }

    /// Simulate asset value paths using Monte Carlo.
    ///
    /// # Arguments
    ///
    /// * `num_paths` - Number of paths to simulate
    /// * `num_steps` - Number of time steps per path (must be >= 1)
    /// * `horizon` - Simulation horizon in years (must be > 0)
    /// * `seed` - RNG seed for reproducible draws
    /// * `antithetic` - When ``True``, use antithetic variates for variance reduction
    #[pyo3(signature = (num_paths, num_steps, horizon, seed, antithetic=false))]
    fn simulate_paths(
        &self,
        num_paths: usize,
        num_steps: usize,
        horizon: f64,
        seed: u64,
        antithetic: bool,
    ) -> PyResult<PySimulatedPaths> {
        let mut rng = Pcg64Rng::new(seed);
        let paths = self
            .inner
            .simulate_paths(num_paths, num_steps, horizon, &mut rng, antithetic)
            .map_err(display_to_py)?;
        Ok(PySimulatedPaths::from_inner(paths))
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("MertonModel", &self.inner)
    }
}

#[pyclass(
    name = "SimulatedPaths",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PySimulatedPaths {
    pub(crate) inner: SimulatedPaths,
}

impl PySimulatedPaths {
    pub(crate) fn from_inner(inner: SimulatedPaths) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimulatedPaths {
    /// Time grid from 0 to the simulation horizon.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times.clone()
    }

    /// Asset values in row-major order.
    #[getter]
    fn asset_values(&self) -> Vec<f64> {
        self.inner.asset_values.clone()
    }

    /// Number of simulated paths.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Number of time steps between grid points.
    #[getter]
    fn num_steps(&self) -> usize {
        self.inner.num_steps
    }

    /// Return one asset value by path and time-grid index.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Zero-based path index
    /// * `time_idx` - Zero-based time-grid index (includes ``t = 0``)
    fn get(&self, path_idx: usize, time_idx: usize) -> Option<f64> {
        self.inner.get(path_idx, time_idx)
    }

    /// Return the contiguous asset-value row for one path.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Zero-based path index
    fn path(&self, path_idx: usize) -> Option<Vec<f64>> {
        self.inner.path(path_idx).map(<[f64]>::to_vec)
    }

    /// Materialize nested path storage as a list of path rows.
    fn to_nested(&self) -> Vec<Vec<f64>> {
        self.inner.to_nested()
    }

    /// Identify this value in notebooks and logs.
    ///
    /// The inner type is not `Serialize`, so the headline shape is rendered
    /// directly; the path arrays themselves are summarised by length.
    fn __repr__(&self) -> String {
        format!(
            "SimulatedPaths(num_paths={}, num_steps={})",
            self.inner.num_paths, self.inner.num_steps
        )
    }
}

#[pyclass(
    name = "DynamicRecoverySpec",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyDynamicRecoverySpec {
    pub(crate) inner: DynamicRecoverySpec,
}

#[pymethods]
impl PyDynamicRecoverySpec {
    #[staticmethod]
    fn constant(recovery: f64) -> PyResult<Self> {
        Ok(Self {
            inner: DynamicRecoverySpec::constant(recovery).map_err(display_to_py)?,
        })
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

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    fn recovery_at_notional(&self, notional: f64) -> f64 {
        self.inner.recovery_at_notional(notional)
    }

    /// Base (reference) recovery rate ``R_0`` as a decimal in ``[0, 1]``.
    #[getter]
    fn base_recovery(&self) -> f64 {
        self.inner.base_recovery()
    }

    /// Base (reference) notional ``N_0`` the recovery mapping is anchored to.
    #[getter]
    fn base_notional(&self) -> f64 {
        self.inner.base_notional()
    }

    /// Notional-to-recovery mapping, in canonical JSON form.
    ///
    /// ``"constant"`` / ``"inverse_linear"`` for the parameterless models, or a
    /// single-key mapping (``inverse_power``, ``floored_inverse``,
    /// ``linear_decline``) carrying that model's parameters.
    #[getter]
    fn model<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, self.inner.model())
    }

    /// Export the recovery specification as a single-row :class:`pandas.DataFrame`.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe(py, &self.inner)
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("DynamicRecoverySpec", &self.inner)
    }
}

#[pyclass(
    name = "EndogenousHazardSpec",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyEndogenousHazardSpec {
    pub(crate) inner: EndogenousHazardSpec,
}

#[pymethods]
impl PyEndogenousHazardSpec {
    #[staticmethod]
    fn power_law(base_hazard: f64, base_leverage: f64, exponent: f64) -> PyResult<Self> {
        Ok(Self {
            inner: EndogenousHazardSpec::power_law(base_hazard, base_leverage, exponent)
                .map_err(display_to_py)?,
        })
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

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    fn hazard_at_leverage(&self, leverage: f64) -> f64 {
        self.inner.hazard_at_leverage(leverage)
    }

    fn hazard_after_pik_accrual(&self, accreted_notional: f64, asset_value: f64) -> f64 {
        self.inner
            .hazard_after_pik_accrual(accreted_notional, asset_value)
    }

    /// Base (reference) hazard rate ``lambda_0``, annualized.
    #[getter]
    fn base_hazard_rate(&self) -> f64 {
        self.inner.base_hazard_rate()
    }

    /// Base (reference) leverage level ``L_0`` the hazard mapping is anchored to.
    #[getter]
    fn base_leverage(&self) -> f64 {
        self.inner.base_leverage()
    }

    /// Leverage-to-hazard mapping, in canonical JSON form.
    ///
    /// A single-key mapping (``power_law``, ``exponential``, ``tabular``)
    /// carrying that model's parameters.
    #[getter]
    fn leverage_hazard_map<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, self.inner.leverage_hazard_map())
    }

    /// Export the hazard specification as a single-row :class:`pandas.DataFrame`.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe(py, &self.inner)
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("EndogenousHazardSpec", &self.inner)
    }
}

#[pyclass(
    name = "CreditState",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyCreditState {
    inner: CreditState,
}

#[pymethods]
impl PyCreditState {
    #[new]
    #[pyo3(signature = (hazard_rate=0.0, distance_to_default=None, leverage=0.0, accreted_notional=0.0, coupon_due=0.0, asset_value=None))]
    fn new(
        hazard_rate: f64,
        distance_to_default: Option<f64>,
        leverage: f64,
        accreted_notional: f64,
        coupon_due: f64,
        asset_value: Option<f64>,
    ) -> Self {
        Self {
            inner: CreditState {
                hazard_rate,
                distance_to_default,
                leverage,
                accreted_notional,
                coupon_due,
                asset_value,
            },
        }
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Annualized instantaneous default intensity at this observation.
    #[getter]
    fn hazard_rate(&self) -> f64 {
        self.inner.hazard_rate
    }

    /// Distance-to-default in standard deviations, or ``None`` when unavailable.
    #[getter]
    fn distance_to_default(&self) -> Option<f64> {
        self.inner.distance_to_default
    }

    /// Leverage ratio (debt / assets).
    #[getter]
    fn leverage(&self) -> f64 {
        self.inner.leverage
    }

    /// Accreted (PIK-augmented) notional outstanding.
    #[getter]
    fn accreted_notional(&self) -> f64 {
        self.inner.accreted_notional
    }

    /// Cash coupon amount due at this decision date.
    #[getter]
    fn coupon_due(&self) -> f64 {
        self.inner.coupon_due
    }

    /// Fair value of the firm's assets, or ``None`` when unavailable.
    #[getter]
    fn asset_value(&self) -> Option<f64> {
        self.inner.asset_value
    }

    /// Export the observed credit state as a single-row :class:`pandas.DataFrame`.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_object_to_single_row_dataframe(py, &self.inner)
    }

    /// Deserialize from JSON produced by `to_json`.
    ///
    /// Completes the wire round-trip, which is also what makes this type
    /// picklable (see `__reduce__`).
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: CreditState = serde_json::from_str(json).map_err(display_to_py)?;
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
        crate::bindings::repr_support::repr_from_serde("CreditState", &self.inner)
    }
}

#[pyclass(
    name = "ToggleExerciseModel",
    module = "finstack_quant.models.credit",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyToggleExerciseModel {
    pub(crate) inner: ToggleExerciseModel,
}

#[pymethods]
impl PyToggleExerciseModel {
    #[staticmethod]
    fn threshold(variable: &str, threshold: f64, direction: &str) -> PyResult<Self> {
        let variable = variable
            .parse::<CreditStateVariable>()
            .map_err(display_to_py)?;
        let direction = direction
            .parse::<ThresholdDirection>()
            .map_err(display_to_py)?;
        Ok(Self {
            inner: ToggleExerciseModel::threshold(variable, threshold, direction),
        })
    }

    #[staticmethod]
    fn optimal(
        nested_paths: usize,
        equity_discount_rate: f64,
        asset_vol: f64,
        risk_free_rate: f64,
        horizon: f64,
    ) -> Self {
        Self {
            inner: ToggleExerciseModel::OptimalExercise(OptimalToggle {
                nested_paths,
                equity_discount_rate,
                asset_vol,
                risk_free_rate,
                horizon,
            }),
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

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(display_to_py)?,
        })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Which exercise rule this model carries.
    ///
    /// One of ``"threshold"``, ``"stochastic"`` or ``"optimal_exercise"`` —
    /// the canonical serde tag, so it also names the single key in the
    /// ``to_json`` payload.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            ToggleExerciseModel::Threshold(_) => "threshold",
            ToggleExerciseModel::Stochastic(_) => "stochastic",
            ToggleExerciseModel::OptimalExercise(_) => "optimal_exercise",
        }
    }

    /// Parameters of the active rule, as a mapping in canonical JSON form.
    ///
    /// The keys depend on :attr:`kind`: ``state_variable`` / ``threshold`` /
    /// ``direction`` for a threshold rule, ``state_variable`` / ``intercept`` /
    /// ``sensitivity`` for a stochastic one, and the nested-Monte-Carlo
    /// settings for optimal exercise.
    #[getter]
    fn params<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match &self.inner {
            ToggleExerciseModel::Threshold(spec) => serde_to_py(py, spec),
            ToggleExerciseModel::Stochastic(spec) => serde_to_py(py, spec),
            ToggleExerciseModel::OptimalExercise(spec) => serde_to_py(py, spec),
        }
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("ToggleExerciseModel", &self.inner)
    }
}

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "credit")?;
    module.setattr(
        "__doc__",
        "Product-independent credit models, scoring, migration, PD, LGD, recovery, and liability-management analytics.",
    )?;
    let qualified_name = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &module,
        "credit",
        "finstack_quant.models",
    )?;
    module.add_class::<PyAssetDynamics>()?;
    module.add_class::<PyBarrierType>()?;
    module.add_class::<PyMertonModel>()?;
    module.add_class::<PySimulatedPaths>()?;
    module.add_class::<PyDynamicRecoverySpec>()?;
    module.add_class::<PyEndogenousHazardSpec>()?;
    module.add_class::<PyCreditState>()?;
    module.add_class::<PyToggleExerciseModel>()?;
    module.add_function(wrap_pyfunction!(moodys_warf_factor, &module)?)?;
    scoring::register(py, &module)?;
    pd::register(py, &module)?;
    lgd::register(py, &module)?;
    migration::register(py, &module)?;
    recovery_waterfall::register(py, &module)?;
    liability_management::register(py, &module)?;
    let all = PyList::new(
        py,
        [
            "AssetDynamics",
            "BarrierType",
            "CreditState",
            "DynamicRecoverySpec",
            "EndogenousHazardSpec",
            "lgd",
            "liability_management",
            "MertonModel",
            "migration",
            "moodys_warf_factor",
            "pd",
            "recovery_waterfall",
            "scoring",
            "SimulatedPaths",
            "ToggleExerciseModel",
        ],
    )?;
    module.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &module, &qualified_name)?;
    Ok(())
}
