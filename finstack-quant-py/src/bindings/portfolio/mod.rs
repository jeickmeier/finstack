//! Python bindings for the `finstack-quant-portfolio` crate.
//!
//! Portfolio contains `Arc<dyn Instrument>` which cannot be directly wrapped,
//! so this module exposes JSON-based construction via [`PortfolioSpec`],
//! result extraction via serde round-trips, and end-to-end pipeline functions
//! that build the runtime portfolio internally.

mod allocation;
mod attribution;
mod brinson;
mod excess_return;
mod factor_brinson;
mod factor_model;
mod fi_attribution;
mod grid_attribution;
mod json_bridge;
mod liquidity;
mod materialization;
mod matrix_input;
mod optimization_spec;
mod performance;
mod pipeline;
mod replay;
mod scenario_pnl;
mod schema;
mod sensitivity;
mod spec;
pub(crate) mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `portfolio` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "portfolio")?;
    m.setattr(
        "__doc__",
        "Portfolio construction, valuation, cashflows, scenarios, and metrics.",
    )?;
    // Common base for every named finstack exception; its canonical home is
    // `finstack_quant.core`, and it is re-exported here alongside the portfolio
    // family so `except FinstackError` is reachable without a second import.
    m.add(
        "FinstackError",
        py.get_type::<crate::errors::FinstackError>(),
    )?;
    m.add(
        "PortfolioError",
        py.get_type::<crate::errors::PortfolioError>(),
    )?;
    m.add(
        "ValuationError",
        py.get_type::<crate::errors::ValuationError>(),
    )?;
    m.add("FxError", py.get_type::<crate::errors::FxError>())?;
    m.add(
        "OptimizationError",
        py.get_type::<crate::errors::OptimizationError>(),
    )?;
    // Deprecated aliases for the pre-rename names. They bind the *same* class
    // objects, so `except FinstackFxError` still catches what it always did.
    m.add(
        "FinstackValuationError",
        py.get_type::<crate::errors::ValuationError>(),
    )?;
    m.add("FinstackFxError", py.get_type::<crate::errors::FxError>())?;
    m.add(
        "FinstackOptimizationError",
        py.get_type::<crate::errors::OptimizationError>(),
    )?;
    m.add(
        "ContractValidationError",
        py.get_type::<crate::errors::ContractValidationError>(),
    )?;
    m.add(
        "UnsupportedContractVersionError",
        py.get_type::<crate::errors::UnsupportedContractVersionError>(),
    )?;
    m.add(
        "MissingContractVersionError",
        py.get_type::<crate::errors::MissingContractVersionError>(),
    )?;
    m.add(
        "MalformedContractSchemaError",
        py.get_type::<crate::errors::MalformedContractSchemaError>(),
    )?;
    m.add(
        "ContractLimitExceededError",
        py.get_type::<crate::errors::ContractLimitExceededError>(),
    )?;

    types::register(py, &m)?;
    materialization::register(py, &m)?;
    spec::register(py, &m)?;
    pipeline::register(py, &m)?;
    attribution::register(py, &m)?;
    optimization_spec::register(py, &m)?;
    allocation::register(py, &m)?;
    replay::register(py, &m)?;
    factor_model::register(py, &m)?;
    sensitivity::register(py, &m)?;
    liquidity::register(py, &m)?;
    brinson::register(py, &m)?;
    fi_attribution::register(py, &m)?;
    performance::register(py, &m)?;
    excess_return::register(py, &m)?;
    grid_attribution::register(py, &m)?;
    factor_brinson::register(py, &m)?;

    let exports = vec![
        "FinstackError",
        "PortfolioError",
        "ValuationError",
        "FxError",
        "OptimizationError",
        // Deprecated aliases; same class objects as the three names above.
        "FinstackValuationError",
        "FinstackFxError",
        "FinstackOptimizationError",
        "ContractValidationError",
        "UnsupportedContractVersionError",
        "MissingContractVersionError",
        "MalformedContractSchemaError",
        "ContractLimitExceededError",
        "Portfolio",
        "InstrumentArtifactCache",
        "MaterializationReport",
        "PortfolioValuation",
        "ScenarioPnl",
        "ScenarioPnlBatchItem",
        "PortfolioResult",
        "PortfolioMetrics",
        "PortfolioCashflows",
        "PortfolioAttribution",
        "BrinsonPeriodResult",
        "CarinoLinkedAttribution",
        "FiAttributionResult",
        "FiCarinoLinkedResult",
        "FiReconciliationReport",
        "DurationCellTable",
        "ExcessReturnResult",
        "GridAttributionResult",
        "GridCarinoLinkedResult",
        "FactorBrinsonResult",
        "LinkedReturn",
        "ReplayResult",
        "WeightAllocationResult",
        "parse_portfolio_spec",
        "build_portfolio_from_spec",
        "portfolio_result_total_value",
        "portfolio_result_get_metric",
        "aggregate_metrics",
        "aggregate_metrics_json",
        "value_portfolio",
        "aggregate_full_cashflows",
        "apply_scenario_and_revalue",
        "scenario_pnl",
        "scenario_pnl_batch",
        "scenario_pnl_batch_json",
        "attribute_portfolio_pnl",
        "allocate_weights",
        "allocate_weights_json",
        "optimize_portfolio",
        "replay_portfolio",
        "replay_portfolio_json",
        "parametric_var_decomposition",
        "parametric_es_decomposition",
        "historical_var_decomposition",
        "evaluate_risk_budget",
        "roll_effective_spread",
        "amihud_illiquidity",
        "days_to_liquidate",
        "liquidity_tier",
        "lvar_bangia",
        "almgren_chriss_impact",
        "kyle_lambda",
        "brinson_fachler",
        "brinson_fachler_json",
        "carino_link",
        "carino_link_json",
        "campisi_attribution",
        "campisi_attribution_json",
        "campisi_carino_link",
        "campisi_carino_link_json",
        "campisi_carino_link_from_snapshots",
        "campisi_carino_link_from_snapshots_json",
        "campisi_reconciliation_check",
        "campisi_reconciliation_check_json",
        "cell_returns_from_curves",
        "cell_returns_from_curves_json",
        "cell_returns_from_reference",
        "cell_returns_from_reference_json",
        "excess_returns",
        "excess_returns_json",
        "factor_brinson_attribution",
        "factor_brinson_attribution_json",
        "grid_attribution",
        "grid_attribution_json",
        "grid_carino_link",
        "grid_carino_link_json",
        "twrr_modified_dietz",
        "twrr_linked",
        "twrr_linked_json",
        "mwr_xirr",
        "SensitivityMatrix",
        "FactorPnlProfile",
        "FactorRiskDecomposition",
        "compute_factor_sensitivities",
        "compute_pnl_profiles",
        "decompose_factor_risk",
        // factor_model typed result classes (Slice 8)
        "FactorContribution",
        "PositionFactorContribution",
        "PositionResidualContribution",
        "RiskDecomposition",
        "PositionVarContribution",
        "PositionEsContribution",
        "PositionRiskDecomposition",
        "PositionBudgetEntry",
        "RiskBudgetResult",
        "FactorContributionDelta",
        "WhatIfResult",
        "StressResult",
        "StressPositionEntry",
        "TailScenarioBreakdown",
        "StressAttribution",
        "PositionAssignment",
        "UnmatchedEntry",
        "FactorAssignmentReport",
        "LevelVolContribution",
        "PositionVolContribution",
        "CreditVolReport",
        "VolHorizon",
        "DecompositionConfig",
        "factor_stress",
        "position_what_if",
        "build_stress_attribution",
        "build_credit_vol_report",
        "validate_allocation_json",
        "position_component_var",
        // optimization spec/result classes (Slice 9)
        "WeightingScheme",
        "MissingMetricPolicy",
        "Inequality",
        "TradeDirection",
        "TradeType",
        "PerPositionMetric",
        "PositionFilter",
        "MetricExpr",
        "Objective",
        "Constraint",
        "CandidatePosition",
        "TradeUniverse",
        "OptimizationStatus",
        "TradeSpec",
        "PortfolioOptimizationSpec",
        "PortfolioOptimizationResult",
        // Schema
        "schema",
    ];

    schema::register(py, &m)?;
    let all = PyList::new(py, exports)?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "portfolio",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;

    Ok(())
}
