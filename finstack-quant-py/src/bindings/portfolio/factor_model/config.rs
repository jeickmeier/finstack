use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_models::factor::credit::VolHorizon;
use finstack_quant_models::factor::risk::{DecompositionConfig, DecompositionMethod};

/// Serde name of a [`DecompositionMethod`] (`"parametric"` / `"historical"`).
pub(super) fn decomposition_method_label(method: DecompositionMethod) -> PyResult<String> {
    finstack_quant_core::wire::serde_label(&method).map_err(crate::errors::core_to_py)
}

/// Forecast horizon used to scale a calibrated `Sample` vol estimate.
///
/// Accepted Python constructors:
///   - ``VolHorizon.one_step()``
///   - ``VolHorizon.unconditional()``
///   - ``VolHorizon.n_steps(n)``
///   - ``VolHorizon.years(years)``
///   - ``VolHorizon.parse("one_step" | "unconditional" | '{"n_steps": N}' | '{"years": Y}')``
#[pyclass(
    name = "VolHorizon",
    module = "finstack_quant.models.factor.credit",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy)]
pub(super) struct PyVolHorizon {
    pub(crate) inner: VolHorizon,
}

impl PyVolHorizon {
    pub(crate) fn from_inner(inner: VolHorizon) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolHorizon {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn one_step(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(VolHorizon::OneStep)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn unconditional(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(VolHorizon::Unconditional)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, n)")]
    fn n_steps(_cls: &Bound<'_, PyType>, n: usize) -> Self {
        Self::from_inner(VolHorizon::NSteps(n))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, years)")]
    fn years(_cls: &Bound<'_, PyType>, years: f64) -> PyResult<Self> {
        if years.is_finite() && years >= 0.0 {
            Ok(Self::from_inner(VolHorizon::Years(years)))
        } else {
            Err(crate::errors::value_error(
                "years must be finite and non-negative",
            ))
        }
    }

    /// Parse a horizon descriptor string (matches the Rust ``VolHorizon::parse``).
    #[classmethod]
    #[pyo3(text_signature = "(cls, s)")]
    fn parse(_cls: &Bound<'_, PyType>, s: &str) -> PyResult<Self> {
        VolHorizon::parse(s)
            .map(Self::from_inner)
            .map_err(crate::errors::value_error)
    }

    /// Variant label: ``"one_step"`` / ``"unconditional"`` / ``"n_steps"`` / ``"years"``.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            VolHorizon::OneStep => "one_step",
            VolHorizon::Unconditional => "unconditional",
            VolHorizon::NSteps(_) => "n_steps",
            VolHorizon::Years(_) => "years",
        }
    }

    /// Step count when ``kind == "n_steps"``, ``None`` otherwise.
    #[getter]
    fn n(&self) -> Option<usize> {
        match self.inner {
            VolHorizon::NSteps(n) => Some(n),
            _ => None,
        }
    }

    /// Fractional-year horizon when ``kind == "years"``, ``None`` otherwise.
    #[getter]
    fn years_value(&self) -> Option<f64> {
        match self.inner {
            VolHorizon::Years(years) => Some(years),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            VolHorizon::OneStep => "VolHorizon.one_step()".to_owned(),
            VolHorizon::Unconditional => "VolHorizon.unconditional()".to_owned(),
            VolHorizon::NSteps(n) => format!("VolHorizon.n_steps({n})"),
            VolHorizon::Years(years) => format!("VolHorizon.years({years})"),
        }
    }
}

/// Configuration for position-level VaR decomposition.
#[pyclass(
    name = "DecompositionConfig",
    module = "finstack_quant.models.factor.risk",
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyDecompositionConfig {
    pub(crate) inner: DecompositionConfig,
}

impl PyDecompositionConfig {
    pub(crate) fn from_inner(inner: DecompositionConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDecompositionConfig {
    /// Standard 95% parametric configuration.
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn parametric_95(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(DecompositionConfig::parametric_95())
    }

    /// Standard 99% parametric configuration.
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn parametric_99(_cls: &Bound<'_, PyType>) -> Self {
        Self::from_inner(DecompositionConfig::parametric_99())
    }

    /// Historical-mode configuration at the given confidence.
    #[classmethod]
    #[pyo3(text_signature = "(cls, confidence)")]
    fn historical(_cls: &Bound<'_, PyType>, confidence: f64) -> Self {
        Self::from_inner(DecompositionConfig::historical(confidence))
    }

    /// Enable incremental VaR computation (expensive).
    #[pyo3(text_signature = "(self)")]
    fn with_incremental(&self) -> Self {
        Self::from_inner(self.inner.clone().with_incremental())
    }

    /// Pin the RNG seed for simulation-path decompositions.
    #[pyo3(text_signature = "(self, seed)")]
    fn with_seed(&self, seed: u64) -> Self {
        Self::from_inner(self.inner.clone().with_seed(seed))
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    /// Decomposition method: ``"parametric"`` or ``"historical"``.
    #[getter]
    fn method(&self) -> PyResult<String> {
        decomposition_method_label(self.inner.method)
    }

    #[getter]
    fn compute_incremental(&self) -> bool {
        self.inner.compute_incremental
    }

    #[getter]
    fn seed(&self) -> Option<u64> {
        self.inner.seed
    }

    fn __repr__(&self) -> String {
        format!(
            "DecompositionConfig(confidence={}, method={:?}, compute_incremental={}, seed={:?})",
            self.inner.confidence,
            decomposition_method_label(self.inner.method).unwrap_or_else(|_| "?".to_string()),
            self.inner.compute_incremental,
            self.inner.seed,
        )
    }
}
