//! Python bindings for the `finstack-quant-statements-analytics` crate.
//!
//! Exposes financial statement analysis: sensitivity, variance, scenario sets,
//! backtesting, goal seek, introspection, DCF valuation, corporate analysis
//! pipeline, reports, comparable-company analysis, ECL, the
//! corkscrew and credit-scorecard extensions, and the roll-forward / vintage /
//! real-estate templates.

mod analysis;
mod comps;
mod corkscrew;
mod ecl;
mod scorecards;
mod templates_common;
mod templates_real_estate;
mod templates_roll_forward;
mod templates_vintage;
mod typed;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Render a unit-variant enum's canonical serde discriminant.
///
/// Single source for status strings (e.g. `CorkscrewStatus::Success` ->
/// `"success"`): the serde `rename_all` on the Rust enum is the wire
/// contract, so the bindings never re-declare the casing by hand.
pub(crate) fn serde_variant_str<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

/// Register the `statements_analytics` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "statements_analytics")?;
    m.setattr(
        "__doc__",
        "Statement analysis: sensitivity, variance, scenarios, backtesting, goal seek, DCF, corporate, reports, introspection, comparable-company analysis, ECL, corkscrew/scorecard extensions, and roll-forward/vintage/real-estate templates.",
    )?;

    analysis::register(py, &m)?;
    typed::register(py, &m)?;
    ecl::register(py, &m)?;
    comps::register(py, &m)?;
    scorecards::register(py, &m)?;
    corkscrew::register(py, &m)?;
    templates_vintage::register(py, &m)?;
    templates_roll_forward::register(py, &m)?;
    templates_real_estate::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "SensitivityConfig",
            "VarianceConfig",
            "ScenarioSet",
            "SensitivityResult",
            "TornadoEntry",
            "VarianceRow",
            "VarianceReport",
            "ScenarioResults",
            "ScenarioDiff",
            "BridgeStep",
            "BridgeChart",
            "run_sensitivity",
            "generate_tornado_entries",
            "run_variance",
            "evaluate_scenario_set",
            "scenario_diff",
            "variance_bridge",
            "backtest_forecast",
            "goal_seek",
            "evaluate_dcf",
            "dcf_sensitivity",
            "evaluate_lbo",
            "wacc",
            "run_corporate_analysis",
            "pl_summary_report_text",
            "credit_assessment",
            "credit_assessment_report_text",
            "DependencyTracer",
            "explain_formula",
            "explain_formula_text",
            "run_checks",
            "run_three_statement_checks",
            "run_credit_underwriting_checks",
            "render_check_report_text",
            "render_check_report_html",
            "Exposure",
            "classify_stage",
            "compute_ecl",
            "compute_ecl_weighted",
            "percentile_rank",
            "z_score",
            "peer_stats",
            "regression_fair_value",
            "compute_multiple",
            "score_relative_value",
            "ScorecardMetric",
            "ScorecardConfig",
            "ScorecardReport",
            "CreditScorecardExtension",
            "validate_scorecard_config",
            "AccountType",
            "CorkscrewAccount",
            "CorkscrewConfig",
            "CorkscrewReport",
            "CorkscrewExtension",
            "add_vintage_buildup",
            "add_roll_forward",
            "add_roll_forward_with_opening",
            "RentStepSpec",
            "FreeRentWindowSpec",
            "RenewalSpec",
            "LeaseGrowthConvention",
            "LeaseSpec",
            "RentRollOutputNodes",
            "ManagementFeeBase",
            "ManagementFeeSpec",
            "PropertyTemplateNodes",
            "add_noi_buildup",
            "add_ncf_buildup",
            "add_rent_roll",
            "add_property_operating_statement",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "statements_analytics",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;

    Ok(())
}
