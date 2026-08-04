use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_portfolio::factor_model::{
    FactorContribution, PositionEsContribution, PositionFactorContribution,
    PositionResidualContribution, PositionRiskDecomposition, PositionVarContribution,
    ResidualContributionSource, RiskDecomposition,
};

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::config::decomposition_method_label;

/// Aggregate contribution of a single factor to portfolio risk.
#[pyclass(
    name = "FactorContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyFactorContribution {
    pub(crate) inner: FactorContribution,
}

impl PyFactorContribution {
    fn from_inner(inner: FactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContribution {
    /// Parse from a JSON string.
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: FactorContribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_owned()
    }

    #[getter]
    fn absolute_risk(&self) -> f64 {
        self.inner.absolute_risk
    }

    #[getter]
    fn relative_risk(&self) -> f64 {
        self.inner.relative_risk
    }

    #[getter]
    fn marginal_risk(&self) -> f64 {
        self.inner.marginal_risk
    }

    fn __repr__(&self) -> String {
        format!(
            "FactorContribution(factor_id={:?}, absolute_risk={}, relative_risk={}, marginal_risk={})",
            self.inner.factor_id.as_str(),
            self.inner.absolute_risk,
            self.inner.relative_risk,
            self.inner.marginal_risk,
        )
    }
}

/// Per-position contribution to a specific factor bucket.
#[pyclass(
    name = "PositionFactorContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionFactorContribution {
    pub(crate) inner: PositionFactorContribution,
}

impl PyPositionFactorContribution {
    fn from_inner(inner: PositionFactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionFactorContribution {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionFactorContribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_owned()
    }

    #[getter]
    fn risk_contribution(&self) -> f64 {
        self.inner.risk_contribution
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionFactorContribution(position_id={:?}, factor_id={:?}, risk_contribution={})",
            self.inner.position_id.as_str(),
            self.inner.factor_id.as_str(),
            self.inner.risk_contribution,
        )
    }
}

/// Annualized residual variance contributed by a single position.
#[pyclass(
    name = "PositionResidualContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionResidualContribution {
    pub(crate) inner: PositionResidualContribution,
}

impl PyPositionResidualContribution {
    fn from_inner(inner: PositionResidualContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionResidualContribution {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionResidualContribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn residual_variance(&self) -> f64 {
        self.inner.residual_variance
    }

    /// Source kind: ``"from_credit_model"`` or ``"other"``.
    #[getter]
    fn source_kind(&self) -> &'static str {
        match &self.inner.source {
            ResidualContributionSource::FromCreditModel { .. } => "from_credit_model",
            ResidualContributionSource::Other => "other",
        }
    }

    /// Issuer ID when ``source_kind == "from_credit_model"``, ``None`` otherwise.
    #[getter]
    fn source_issuer_id(&self) -> Option<String> {
        match &self.inner.source {
            ResidualContributionSource::FromCreditModel { issuer_id } => {
                Some(issuer_id.as_str().to_owned())
            }
            ResidualContributionSource::Other => None,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionResidualContribution(position_id={:?}, residual_variance={}, source_kind={:?})",
            self.inner.position_id.as_str(),
            self.inner.residual_variance,
            self.source_kind(),
        )
    }
}

/// Portfolio-level decomposition of total risk across common factors and residuals.
#[pyclass(
    name = "RiskDecomposition",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyRiskDecomposition {
    pub(crate) inner: RiskDecomposition,
}

impl PyRiskDecomposition {
    pub(super) fn from_inner(inner: RiskDecomposition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRiskDecomposition {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: RiskDecomposition = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn total_risk(&self) -> f64 {
        self.inner.total_risk
    }

    /// Risk measure used for aggregation (serialized as a JSON-compatible string).
    #[getter]
    fn measure_json(&self) -> PyResult<String> {
        serialize_json(&self.inner.measure)
    }

    #[getter]
    fn residual_risk(&self) -> f64 {
        self.inner.residual_risk
    }

    #[getter]
    fn factor_contributions(&self) -> Vec<PyFactorContribution> {
        self.inner
            .factor_contributions
            .iter()
            .cloned()
            .map(PyFactorContribution::from_inner)
            .collect()
    }

    #[getter]
    fn position_factor_contributions(&self) -> Vec<PyPositionFactorContribution> {
        self.inner
            .position_factor_contributions
            .iter()
            .cloned()
            .map(PyPositionFactorContribution::from_inner)
            .collect()
    }

    #[getter]
    fn position_residual_contributions(&self) -> Vec<PyPositionResidualContribution> {
        self.inner
            .position_residual_contributions
            .iter()
            .cloned()
            .map(PyPositionResidualContribution::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "RiskDecomposition(total_risk={}, residual_risk={}, factors={}, position_factors={}, position_residuals={})",
            self.inner.total_risk,
            self.inner.residual_risk,
            self.inner.factor_contributions.len(),
            self.inner.position_factor_contributions.len(),
            self.inner.position_residual_contributions.len(),
        )
    }
}

/// Per-position component VaR and marginal VaR.
#[pyclass(
    name = "PositionVarContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionVarContribution {
    pub(crate) inner: PositionVarContribution,
}

impl PyPositionVarContribution {
    fn from_inner(inner: PositionVarContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionVarContribution {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionVarContribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn component_var(&self) -> f64 {
        self.inner.component_var
    }

    #[getter]
    fn relative_var(&self) -> f64 {
        self.inner.relative_var
    }

    #[getter]
    fn marginal_var(&self) -> Option<f64> {
        self.inner.marginal_var
    }

    #[getter]
    fn incremental_var(&self) -> Option<f64> {
        self.inner.incremental_var
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionVarContribution(position_id={:?}, component_var={}, relative_var={}, marginal_var={:?}, incremental_var={:?})",
            self.inner.position_id.as_str(),
            self.inner.component_var,
            self.inner.relative_var,
            self.inner.marginal_var,
            self.inner.incremental_var,
        )
    }
}

/// Per-position component ES and marginal ES.
#[pyclass(
    name = "PositionEsContribution",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionEsContribution {
    pub(crate) inner: PositionEsContribution,
}

impl PyPositionEsContribution {
    fn from_inner(inner: PositionEsContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionEsContribution {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionEsContribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn component_es(&self) -> f64 {
        self.inner.component_es
    }

    #[getter]
    fn relative_es(&self) -> f64 {
        self.inner.relative_es
    }

    #[getter]
    fn marginal_es(&self) -> Option<f64> {
        self.inner.marginal_es
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionEsContribution(position_id={:?}, component_es={}, relative_es={}, marginal_es={:?})",
            self.inner.position_id.as_str(),
            self.inner.component_es,
            self.inner.relative_es,
            self.inner.marginal_es,
        )
    }
}

/// Complete position-level risk decomposition.
#[pyclass(
    name = "PositionRiskDecomposition",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionRiskDecomposition {
    pub(crate) inner: PositionRiskDecomposition,
}

impl PyPositionRiskDecomposition {
    pub(super) fn from_inner(inner: PositionRiskDecomposition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionRiskDecomposition {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionRiskDecomposition = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn portfolio_var(&self) -> f64 {
        self.inner.portfolio_var
    }

    #[getter]
    fn portfolio_es(&self) -> f64 {
        self.inner.portfolio_es
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    #[getter]
    fn n_positions(&self) -> usize {
        self.inner.n_positions
    }

    /// Decomposition method: ``"parametric"`` or ``"historical"``.
    #[getter]
    fn method(&self) -> &'static str {
        decomposition_method_label(self.inner.method)
    }

    /// Parametric-mode numerical residual; ``None`` in historical mode.
    #[getter]
    fn euler_residual(&self) -> Option<f64> {
        self.inner.euler_residual
    }

    #[getter]
    fn var_contributions(&self) -> Vec<PyPositionVarContribution> {
        self.inner
            .var_contributions
            .iter()
            .cloned()
            .map(PyPositionVarContribution::from_inner)
            .collect()
    }

    #[getter]
    fn es_contributions(&self) -> Vec<PyPositionEsContribution> {
        self.inner
            .es_contributions
            .iter()
            .cloned()
            .map(PyPositionEsContribution::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionRiskDecomposition(portfolio_var={}, portfolio_es={}, confidence={}, n_positions={}, method={:?})",
            self.inner.portfolio_var,
            self.inner.portfolio_es,
            self.inner.confidence,
            self.inner.n_positions,
            decomposition_method_label(self.inner.method),
        )
    }
}
