//! Python bindings for `finstack_quant_cashflows::builder::schedule`.

use finstack_quant_cashflows::builder::CashFlowSchedule;
use pyo3::prelude::*;

use crate::bindings::cashflows::primitives::PyCashFlow;

use super::orchestrator::PyCashFlowBuilder;

/// Wrapper for [`CashFlowSchedule`]
/// (`finstack_quant.cashflows.builder.CashFlowSchedule`).
#[pyclass(
    name = "CashFlowSchedule",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashFlowSchedule {
    /// Inner canonical schedule.
    pub(crate) inner: CashFlowSchedule,
}

impl PyCashFlowSchedule {
    /// Build from an existing Rust [`CashFlowSchedule`].
    pub(crate) fn from_inner(inner: CashFlowSchedule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashFlowSchedule {
    /// Create a new fluent cashflow builder (the only builder entry point).
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCashFlowBuilder {
        PyCashFlowBuilder::new_default()
    }

    /// Canonical ordered cashflows.
    #[pyo3(text_signature = "(self)")]
    fn get_flows(&self) -> Vec<PyCashFlow> {
        self.inner
            .get_flows()
            .iter()
            .map(|f| PyCashFlow::from_inner(*f))
            .collect()
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("CashFlowSchedule(flows={})", self.inner.get_flows().len())
    }
}
