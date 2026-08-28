//! Pricer bindings — European, Path-Dependent, LSMC.

use super::engine::{py_mc_defaults, resolve_currency};
use super::results::PyMoneyEstimate;
use crate::errors::core_to_py;
use finstack_quant_models::monte_carlo::pricer::basis::BasisKind;
use finstack_quant_models::monte_carlo::pricer::european::EuropeanPricer;
use finstack_quant_models::monte_carlo::pricer::lsmc::LsmcPricer;
use finstack_quant_models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
use pyo3::prelude::*;

/// Convenience pricer for European options under GBM dynamics.
#[pyclass(
    name = "EuropeanPricer",
    module = "finstack_quant.models.monte_carlo",
    frozen
)]
pub struct PyEuropeanPricer {
    inner: EuropeanPricer,
}

#[pymethods]
impl PyEuropeanPricer {
    #[new]
    #[pyo3(signature = (num_paths=None, seed=None, use_parallel=None))]
    fn new(
        num_paths: Option<usize>,
        seed: Option<u64>,
        use_parallel: Option<bool>,
    ) -> PyResult<Self> {
        let defaults = &py_mc_defaults()?.european_pricer;
        let inner = EuropeanPricer::new(num_paths.unwrap_or(defaults.num_paths))
            .with_seed(seed.unwrap_or(defaults.seed))
            .with_parallel(use_parallel.unwrap_or(defaults.use_parallel));
        Ok(Self { inner })
    }

    /// Independent Monte Carlo path count used by this pricer.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths()
    }
    /// Seed value used for path generation.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed()
    }
    /// Whether path generation runs on the rayon pool.
    #[getter]
    fn use_parallel(&self) -> bool {
        self.inner.use_parallel()
    }

    /// Price a European call option under GBM.
    ///
    /// Releases the GIL during the Monte Carlo run so other Python threads
    /// can make progress.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=None, currency=None))]
    fn price_call(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: Option<usize>,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let ccy = resolve_currency(currency)?;
        let num_steps = num_steps.unwrap_or(py_mc_defaults()?.european_pricer.num_steps);
        let pricer = &self.inner;
        py.detach(|| {
            pricer.price_gbm_call(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Price a European put option under GBM.
    ///
    /// Releases the GIL during the Monte Carlo run so other Python threads
    /// can make progress.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=None, currency=None))]
    fn price_put(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: Option<usize>,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let ccy = resolve_currency(currency)?;
        let num_steps = num_steps.unwrap_or(py_mc_defaults()?.european_pricer.num_steps);
        let pricer = &self.inner;
        py.detach(|| {
            pricer.price_gbm_put(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanPricer(num_paths={}, seed={}, use_parallel={})",
            self.inner.num_paths(),
            self.inner.seed(),
            self.inner.use_parallel(),
        )
    }
}

/// Path-dependent Monte Carlo pricer for exotic payoffs (Asian, barrier, etc.).
#[pyclass(
    name = "PathDependentPricer",
    module = "finstack_quant.models.monte_carlo",
    frozen
)]
pub struct PyPathDependentPricer {
    inner: PathDependentPricer,
}

#[pymethods]
impl PyPathDependentPricer {
    #[new]
    #[pyo3(signature = (num_paths=None, seed=None, use_parallel=None))]
    fn new(
        num_paths: Option<usize>,
        seed: Option<u64>,
        use_parallel: Option<bool>,
    ) -> PyResult<Self> {
        let defaults = &py_mc_defaults()?.path_dependent_pricer;
        let config = PathDependentPricerConfig::new(num_paths.unwrap_or(defaults.num_paths))
            .with_seed(seed.unwrap_or(defaults.seed))
            .with_parallel(use_parallel.unwrap_or(defaults.use_parallel));
        Ok(Self {
            inner: PathDependentPricer::new(config),
        })
    }

    /// Price an Asian call under GBM dynamics.
    ///
    /// Releases the GIL during the Monte Carlo run.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=None, currency=None))]
    fn price_asian_call(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: Option<usize>,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let ccy = resolve_currency(currency)?;
        let num_steps = num_steps.unwrap_or(py_mc_defaults()?.path_dependent_pricer.num_steps);
        let pricer = &self.inner;
        py.detach(|| {
            pricer.price_gbm_asian_call(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Price an Asian put under GBM dynamics.
    ///
    /// Releases the GIL during the Monte Carlo run.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, num_steps=None, currency=None))]
    fn price_asian_put(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        num_steps: Option<usize>,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let ccy = resolve_currency(currency)?;
        let num_steps = num_steps.unwrap_or(py_mc_defaults()?.path_dependent_pricer.num_steps);
        let pricer = &self.inner;
        py.detach(|| {
            pricer.price_gbm_asian_put(spot, strike, rate, div_yield, vol, expiry, num_steps, ccy)
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Independent Monte Carlo path count used by this pricer.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.config().num_paths
    }
    /// Seed value used for path generation.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.config().seed
    }

    fn __repr__(&self) -> String {
        format!(
            "PathDependentPricer(paths={}, seed={}, parallel={})",
            self.inner.config().num_paths,
            self.inner.config().seed,
            self.inner.config().use_parallel,
        )
    }
}

/// Longstaff-Schwartz Monte Carlo pricer for American options.
#[pyclass(
    name = "LsmcPricer",
    module = "finstack_quant.models.monte_carlo",
    frozen
)]
pub struct PyLsmcPricer {
    inner: LsmcPricer,
    num_steps: usize,
    basis: BasisKind,
    basis_degree: usize,
}

#[pymethods]
impl PyLsmcPricer {
    #[new]
    #[pyo3(signature = (
        num_paths=None,
        seed=None,
        use_parallel=None,
        num_steps=None,
        basis=None,
        basis_degree=None,
        antithetic=None,
    ))]
    fn new(
        num_paths: Option<usize>,
        seed: Option<u64>,
        use_parallel: Option<bool>,
        num_steps: Option<usize>,
        basis: Option<&str>,
        basis_degree: Option<usize>,
        antithetic: Option<bool>,
    ) -> PyResult<Self> {
        let defaults = &py_mc_defaults()?.lsmc;
        let basis = BasisKind::parse(basis.unwrap_or(defaults.basis.as_str()))
            .map_err(crate::errors::value_error)?;
        let num_steps = num_steps.unwrap_or(defaults.num_steps);
        let inner = LsmcPricer::gbm_american(
            num_paths.unwrap_or(defaults.num_paths),
            num_steps,
            seed.unwrap_or(defaults.seed),
            use_parallel.unwrap_or(defaults.use_parallel),
            antithetic.unwrap_or(defaults.antithetic),
        )
        .map_err(core_to_py)?;
        Ok(Self {
            inner,
            num_steps,
            basis,
            basis_degree: basis_degree.unwrap_or(defaults.basis_degree),
        })
    }

    /// Independent Monte Carlo path count used by this pricer.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.config().num_paths
    }
    /// Seed value used for path generation.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.config().seed
    }
    /// Whether path generation runs on the rayon pool.
    #[getter]
    fn use_parallel(&self) -> bool {
        self.inner.config().use_parallel
    }
    /// Whether each path is paired with its sign-flipped counterpart.
    #[getter]
    fn antithetic(&self) -> bool {
        self.inner.config().antithetic
    }
    #[getter]
    fn basis(&self) -> &'static str {
        self.basis.as_str()
    }
    #[getter]
    fn basis_degree(&self) -> usize {
        self.basis_degree
    }

    /// Price an American put under GBM dynamics.
    ///
    /// Releases the GIL during the Monte Carlo run.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, currency=None))]
    fn price_american_put(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let currency = resolve_currency(currency)?;
        let pricer = &self.inner;
        let num_steps = self.num_steps;
        py.detach(|| {
            pricer.price_gbm_american_put(
                spot,
                strike,
                rate,
                div_yield,
                vol,
                expiry,
                num_steps,
                currency,
                self.basis,
                self.basis_degree,
            )
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Price an American call under GBM dynamics.
    ///
    /// Releases the GIL during the Monte Carlo run.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry, currency=None))]
    fn price_american_call(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let currency = resolve_currency(currency)?;
        let pricer = &self.inner;
        let num_steps = self.num_steps;
        py.detach(|| {
            pricer.price_gbm_american_call(
                spot,
                strike,
                rate,
                div_yield,
                vol,
                expiry,
                num_steps,
                currency,
                self.basis,
                self.basis_degree,
            )
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Two-pass unbiased American put price (training fit + out-of-sample pricing).
    ///
    /// Mitigates the in-sample upward bias of single-pass LSMC: the regression
    /// is fit on a training path set seeded by the pricer's `seed`, and the
    /// frozen exercise policy is replayed against an *independent* path set
    /// seeded by `pricing_seed`. `pricing_seed` must differ from `seed`;
    /// matching seeds reintroduce the bias and are rejected.
    ///
    /// Releases the GIL during both Monte Carlo passes.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        spot, strike, rate, div_yield, vol, expiry, pricing_seed, currency=None,
    ))]
    fn price_american_put_unbiased(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        pricing_seed: u64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let currency = resolve_currency(currency)?;
        let pricer = &self.inner;
        let num_steps = self.num_steps;
        py.detach(|| {
            pricer.price_gbm_american_put_unbiased(
                spot,
                strike,
                rate,
                div_yield,
                vol,
                expiry,
                num_steps,
                currency,
                self.basis,
                self.basis_degree,
                pricing_seed,
            )
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    /// Two-pass unbiased American call price.
    ///
    /// See [`Self::price_american_put_unbiased`] for the bias-mitigation
    /// rationale and the meaning of `pricing_seed`.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        spot, strike, rate, div_yield, vol, expiry, pricing_seed, currency=None,
    ))]
    fn price_american_call_unbiased(
        &self,
        py: Python<'_>,
        spot: f64,
        strike: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
        expiry: f64,
        pricing_seed: u64,
        currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyMoneyEstimate> {
        let currency = resolve_currency(currency)?;
        let pricer = &self.inner;
        let num_steps = self.num_steps;
        py.detach(|| {
            pricer.price_gbm_american_call_unbiased(
                spot,
                strike,
                rate,
                div_yield,
                vol,
                expiry,
                num_steps,
                currency,
                self.basis,
                self.basis_degree,
                pricing_seed,
            )
        })
        .map(PyMoneyEstimate::from_inner)
        .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "LsmcPricer(paths={}, seed={}, use_parallel={}, antithetic={}, steps={}, basis={}, basis_degree={})",
            self.inner.config().num_paths,
            self.inner.config().seed,
            self.inner.config().use_parallel,
            self.inner.config().antithetic,
            self.num_steps,
            self.basis.as_str(),
            self.basis_degree,
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEuropeanPricer>()?;
    m.add_class::<PyPathDependentPricer>()?;
    m.add_class::<PyLsmcPricer>()?;
    Ok(())
}
