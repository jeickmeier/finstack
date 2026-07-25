//! Python bindings for `finstack_quant_cashflows::builder`.
//!
//! One Python module (`finstack_quant.cashflows.builder`) mirrors the Rust
//! `builder` re-export surface: spec types, the fluent `CashFlowBuilder`, and
//! `CashFlowSchedule`.

pub(crate) mod orchestrator;
pub(crate) mod schedule;
pub(crate) mod specs;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Register the `finstack_quant.cashflows.builder` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "builder")?;
    module.setattr(
        "__doc__",
        "Composable cashflow builder: coupon/fee/amortization specs, CashFlowBuilder, CashFlowSchedule.",
    )?;

    specs::add_classes(&module)?;
    module.add_class::<orchestrator::PyCashFlowBuilder>()?;
    module.add_class::<orchestrator::PyPrincipalEvent>()?;
    module.add_class::<schedule::PyCashFlowSchedule>()?;

    let all = PyList::new(
        py,
        [
            "AmortizationSpec",
            "CashFlowBuilder",
            "CashFlowSchedule",
            "CouponType",
            "DefaultModelSpec",
            "FeeAccrualBasis",
            "FeeBase",
            "FeeSpec",
            "FixedCouponSpec",
            "FloatingCouponSpec",
            "FloatingRateFallback",
            "FloatingRateSpec",
            "Notional",
            "OvernightCompoundingMethod",
            "OvernightIndexConstraintApplication",
            "PrepaymentModelSpec",
            "PrincipalEvent",
            "RecoveryModelSpec",
            "RollRule",
            "ScheduleParams",
            "StepUpCouponSpec",
        ],
    )?;
    module.setattr("__all__", all)?;

    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &module,
        "builder",
        "finstack_quant.cashflows",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
