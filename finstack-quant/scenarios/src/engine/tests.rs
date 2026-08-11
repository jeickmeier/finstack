use super::*;
use crate::spec::{OperationSpec, TimeRollMode};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::hierarchy::{
    HierarchyTarget, MarketDataHierarchy, ResolutionMode,
};
use finstack_quant_statements::FinancialModelSpec;
use time::macros::date;

#[test]
fn try_compose_rejects_two_time_rolls() {
    let engine = ScenarioEngine::new();
    let s1 = ScenarioSpec {
        id: "roll_6m".into(),
        name: Some("Roll 6M".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "6M".into(),
            apply_shocks: true,
            roll_mode: TimeRollMode::default(),
        }],
        priority: 1,
        resolution_mode: ResolutionMode::Cumulative,
    };
    let s2 = ScenarioSpec {
        id: "roll_1y".into(),
        name: Some("Roll 1Y".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1Y".into(),
            apply_shocks: true,
            roll_mode: TimeRollMode::default(),
        }],
        priority: 2,
        resolution_mode: ResolutionMode::Cumulative,
    };

    let err = engine
        .try_compose(vec![s1, s2])
        .expect_err("duplicate TimeRollForward must error at compose time");
    let msg = format!("{err}");
    assert!(msg.contains("TimeRollForward"));
}

#[test]
fn try_compose_preserves_source_ids_and_names() {
    let engine = ScenarioEngine::new();
    let scenarios = vec![
        ScenarioSpec {
            id: "rates_up".into(),
            name: Some("Rates Up".into()),
            description: None,
            operations: vec![OperationSpec::StmtForecastPercent {
                node_id: "Revenue".into(),
                pct: 1.0,
            }],
            priority: 2,
            resolution_mode: ResolutionMode::MostSpecificWins,
        },
        ScenarioSpec {
            id: "credit_down".into(),
            name: None,
            description: None,
            operations: vec![OperationSpec::StmtForecastPercent {
                node_id: "Expenses".into(),
                pct: -1.0,
            }],
            priority: 1,
            resolution_mode: ResolutionMode::Cumulative,
        },
    ];

    let strict = engine.try_compose(scenarios).expect("valid compose");

    assert_eq!(strict.id.as_str(), "credit_down+rates_up");
    assert_eq!(strict.name.as_deref(), Some("credit_down + Rates Up"));
    assert_eq!(strict.operations.len(), 2);
    assert_eq!(strict.resolution_mode, ResolutionMode::Cumulative);
}

#[test]
fn apply_rejects_hierarchy_op_without_hierarchy() {
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);
    let scenario = ScenarioSpec {
        id: "h_no_attach".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::HierarchyEquityPricePct {
            target: HierarchyTarget {
                path: vec!["equities".into(), "us".into()],
                tag_filter: None,
            },
            pct: -10.0,
        }],
        priority: 0,
        resolution_mode: Default::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date!(2025 - 01 - 01),
    };
    let err = engine
        .apply(&scenario, &mut ctx)
        .expect_err("hierarchy op without hierarchy must error");
    assert!(err.to_string().contains("hierarchy"));
}

#[test]
fn apply_emits_warning_when_hierarchy_target_matches_no_curves() {
    // Empty hierarchy attached, but the target path has no curves.
    let hierarchy = MarketDataHierarchy::default();
    let mut market = MarketContext::new();
    market.set_hierarchy(hierarchy);
    let mut model = FinancialModelSpec::new("test", vec![]);
    let scenario = ScenarioSpec {
        id: "h_empty".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::HierarchyEquityPricePct {
            target: HierarchyTarget {
                path: vec!["equities".into(), "us".into()],
                tag_filter: None,
            },
            pct: -10.0,
        }],
        priority: 0,
        resolution_mode: Default::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date!(2025 - 01 - 01),
    };
    let report = engine
        .apply(&scenario, &mut ctx)
        .expect("apply should succeed");

    assert_eq!(report.operations_applied, 0);
    assert_eq!(report.expanded_operations, 0);
    assert!(
        report.warnings.iter().any(|w| matches!(
            w,
            Warning::HierarchyNoMatch { op_kind, .. } if op_kind == "HierarchyEquityPricePct"
        )),
        "expected HierarchyNoMatch warning, got {:?}",
        report.warnings
    );
}

#[test]
fn market_only_context_applies_without_statement_model() {
    let mut market = MarketContext::new();
    let scenario = ScenarioSpec {
        id: "market_only_roll".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1D".into(),
            apply_shocks: false,
            roll_mode: TimeRollMode::CalendarDays,
        }],
        priority: 0,
        resolution_mode: Default::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: None,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date!(2025 - 01 - 01),
    };

    let report = engine
        .apply(&scenario, &mut ctx)
        .expect("market-only scenario should not require a statement model");

    assert_eq!(report.operations_applied, 1);
    assert!(report.changes.as_of_changed);
    assert!(report.changes.all_dirty);
    assert_eq!(ctx.as_of, date!(2025 - 01 - 02));
}

#[test]
fn application_report_requires_change_manifest() {
    let incomplete_json = r#"{
        "operations_applied": 0,
        "user_operations": 0,
        "expanded_operations": 0,
        "warnings": [],
        "meta": null
    }"#;
    let error = serde_json::from_str::<ApplicationReport>(incomplete_json)
        .expect_err("changes is required by the canonical report contract");
    assert!(error.to_string().contains("changes"));

    let report = ApplicationReport {
        operations_applied: 0,
        user_operations: 0,
        expanded_operations: 0,
        changes: ScenarioChangeManifest::default(),
        warnings: vec![],
        meta: None,
        time_roll: None,
    };
    let encoded = serde_json::to_value(&report).expect("report should serialize");
    assert_eq!(encoded["changes"]["market_targets"], serde_json::json!([]));
    assert_eq!(encoded["changes"]["all_dirty"], serde_json::json!(false));
}

#[test]
fn statement_operation_without_model_errors_clearly() {
    let mut market = MarketContext::new();
    let scenario = ScenarioSpec {
        id: "missing_model".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::StmtForecastPercent {
            node_id: "Revenue".into(),
            pct: -10.0,
        }],
        priority: 0,
        resolution_mode: Default::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: None,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date!(2025 - 01 - 01),
    };

    let err = engine
        .apply(&scenario, &mut ctx)
        .expect_err("statement operation should require a statement model");

    assert!(matches!(
        err,
        crate::error::Error::MissingStatementModel { .. }
    ));
}
