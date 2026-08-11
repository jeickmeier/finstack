//! WASM bindings for the `finstack-quant-scenarios` crate.
//!
//! Exposes scenario specification parsing, validation, composition,
//! and built-in template access via JSON round-trip functions.

use crate::utils::{parse_iso_date, to_js_err};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

/// Lazily-initialised builtin template registry.  Constructed once on first
/// access, then reused for the lifetime of the WASM module.
fn builtin_registry() -> Result<&'static finstack_quant_scenarios::TemplateRegistry, JsValue> {
    static REGISTRY: OnceLock<Result<finstack_quant_scenarios::TemplateRegistry, String>> =
        OnceLock::new();
    let stored = REGISTRY.get_or_init(|| {
        finstack_quant_scenarios::TemplateRegistry::with_embedded_builtins()
            .map_err(|e| e.to_string())
    });
    stored.as_ref().map_err(to_js_err)
}

fn apply_with_context(
    spec: &finstack_quant_scenarios::ScenarioSpec,
    market: &mut finstack_quant_core::market_data::context::MarketContext,
    model: Option<&mut finstack_quant_statements::FinancialModelSpec>,
    as_of: time::Date,
) -> Result<finstack_quant_scenarios::engine::ApplicationReport, JsValue> {
    let engine = finstack_quant_scenarios::ScenarioEngine::new();
    let mut ctx = finstack_quant_scenarios::ExecutionContext {
        market,
        model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of,
    };
    engine.apply(spec, &mut ctx).map_err(to_js_err)
}

/// Parse and validate a scenario specification from JSON.
///
/// Returns the validated, re-serialized JSON.
///
/// # Errors
///
/// Rejects malformed or schema-incompatible `json_str`, a blank scenario ID,
/// multiple time-roll operations, invalid operation identifiers or numeric
/// fields, variant-specific operation violations, or serialization failure.
/// @param json_str - Canonical JSON string to validate, parse, or normalize for this API.
#[wasm_bindgen(js_name = parseScenarioSpec)]
pub fn parse_scenario_spec(json_str: &str) -> Result<String, JsValue> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    spec.validate().map_err(to_js_err)?;

    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Compose multiple scenario specs (JSON array) into a single scenario.
///
/// Specs are merged in priority order (lower number runs first).
///
/// # Errors
///
/// Rejects malformed or schema-incompatible `specs_json`, composition that
/// contains more than one time-roll operation, or failure to serialize the
/// composed specification.
/// @param specs_json - JSON array of validated ScenarioSpec objects to compose in priority order.
#[wasm_bindgen(js_name = composeScenarios)]
pub fn compose_scenarios(specs_json: &str) -> Result<String, JsValue> {
    compose_scenarios_json(specs_json).map_err(to_js_err)
}

fn compose_scenarios_json(specs_json: &str) -> Result<String, String> {
    let specs: Vec<finstack_quant_scenarios::ScenarioSpec> =
        serde_json::from_str(specs_json).map_err(|e| e.to_string())?;

    let engine = finstack_quant_scenarios::ScenarioEngine::new();
    let composed = engine.try_compose(specs).map_err(|e| e.to_string())?;

    serde_json::to_string(&composed).map_err(|e| e.to_string())
}

/// Validate a scenario specification JSON without executing it.
///
/// Returns `true` if valid, throws on error.
///
/// # Errors
///
/// Rejects malformed or schema-incompatible `json_str`, a blank scenario ID,
/// multiple time-roll operations, invalid operation identifiers or numeric
/// fields, or variant-specific operation violations.
/// @param json_str - Canonical JSON string to validate, parse, or normalize for this API.
#[wasm_bindgen(js_name = validateScenarioSpec)]
pub fn validate_scenario_spec(json_str: &str) -> Result<bool, JsValue> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    spec.validate().map_err(to_js_err)?;
    Ok(true)
}

/// List all built-in template identifiers.
///
/// Returns a JSON array of template ID strings.
///
/// # Errors
///
/// Rejects if the embedded template registry cannot be parsed and validated,
/// or if its template identifiers cannot be serialized to JavaScript.
#[wasm_bindgen(js_name = listBuiltinTemplates)]
pub fn list_builtin_templates() -> Result<JsValue, JsValue> {
    let registry = builtin_registry()?;
    let ids: Vec<String> = registry.list().iter().map(|m| m.id.clone()).collect();
    crate::utils::to_js_value(&ids)
}

/// Get metadata for all built-in templates as a JSON string.
///
/// # Errors
///
/// Rejects if the embedded template registry cannot be parsed and validated,
/// or if its metadata cannot be serialized to JSON.
#[wasm_bindgen(js_name = listBuiltinTemplateMetadata)]
pub fn list_builtin_template_metadata() -> Result<String, JsValue> {
    let registry = builtin_registry()?;
    let metadata: Vec<&finstack_quant_scenarios::TemplateMetadata> = registry.list();
    serde_json::to_string(&metadata).map_err(to_js_err)
}

/// Build a scenario spec from a built-in template.
///
/// Returns JSON-serialized `ScenarioSpec`.
///
/// # Errors
///
/// Rejects a failure to load the embedded registry, an unknown `template_id`,
/// a template whose resolved scenario fails validation, or failure to serialize
/// the scenario.
/// @param template_id - Identifier of a built-in scenario template in the embedded registry.
#[wasm_bindgen(js_name = buildFromTemplate)]
pub fn build_from_template(template_id: &str) -> Result<String, JsValue> {
    let registry = builtin_registry()?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;

    let spec = entry.builder().build().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// List component IDs for a built-in composite template.
///
/// Returns a JS array of component ID strings.
///
/// # Errors
///
/// Rejects a failure to load the embedded registry, an unknown `template_id`,
/// or component identifiers that cannot be serialized to JavaScript.
/// @param template_id - Identifier of a built-in scenario template in the embedded registry.
#[wasm_bindgen(js_name = listTemplateComponents)]
pub fn list_template_components(template_id: &str) -> Result<JsValue, JsValue> {
    let registry = builtin_registry()?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;

    let ids: Vec<String> = entry
        .component_ids()
        .into_iter()
        .map(str::to_string)
        .collect();
    crate::utils::to_js_value(&ids)
}

/// Build a specific component from a built-in composite template.
///
/// # Errors
///
/// Rejects a failure to load the embedded registry, an unknown `template_id`
/// or `component_id`, a component scenario that fails validation, or failure to
/// serialize the scenario.
/// @param template_id - Identifier of a built-in scenario template in the embedded registry.
/// @param component_id - Identifier of a component within the selected composite template.
#[wasm_bindgen(js_name = buildTemplateComponent)]
pub fn build_template_component(template_id: &str, component_id: &str) -> Result<String, JsValue> {
    let registry = builtin_registry()?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;
    let builder = entry.component(component_id).ok_or_else(|| {
        to_js_err(format!(
            "Unknown component '{component_id}' in template '{template_id}'"
        ))
    })?;
    let spec = builder.build().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build a scenario spec from fields.
///
/// # Errors
///
/// Rejects malformed or schema-incompatible `operations_json`, an unsupported
/// `resolution_mode`, a blank scenario ID, multiple time-roll operations,
/// invalid operation identifiers or numeric fields, variant-specific operation
/// violations, or failure to serialize the scenario.
/// @param id - Stable identifier used to name and retrieve the supplied domain object.
/// @param operations_json - JSON array of scenario operation specifications in execution order.
/// @param name - Optional human-readable scenario name.
/// @param description - Optional human-readable description of the scenario purpose.
/// @param priority - Execution priority; lower values run earlier during composition.
/// @param resolution_mode - Optional hierarchy conflict policy:
///   `"most_specific_wins"` (default) or `"cumulative"`.
#[wasm_bindgen(js_name = buildScenarioSpec)]
pub fn build_scenario_spec(
    id: &str,
    operations_json: &str,
    name: Option<String>,
    description: Option<String>,
    priority: i32,
    resolution_mode: Option<String>,
) -> Result<String, JsValue> {
    let operations: Vec<finstack_quant_scenarios::OperationSpec> =
        serde_json::from_str(operations_json).map_err(to_js_err)?;
    let resolution_mode = resolution_mode
        .map(|value| serde_json::from_value(serde_json::Value::String(value)))
        .transpose()
        .map_err(to_js_err)?
        .unwrap_or_default();
    let spec = finstack_quant_scenarios::ScenarioSpec {
        id: id.to_string(),
        name,
        description,
        operations,
        priority,
        resolution_mode,
    };
    spec.validate().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Apply a scenario to a market context and financial model.
///
/// Returns a JavaScript object with `market` and `model` (the mutated
/// contexts as objects, not JSON strings), `operations_applied`,
/// `user_operations`, `expanded_operations`, `changes` (a
/// `ScenarioChangeManifest`), `warnings`, `meta` (a `ResultsMeta` audit stamp
/// carrying the numeric mode, rounding context, and FX policy; omitted when
/// absent), and `time_roll` (a `RollForwardReport`, only present when the
/// scenario contained a `time_roll_forward` operation).
///
/// This entry point supplies no instrument portfolio and no holiday calendar
/// to the engine: instrument-scoped operations (`instrument_price_pct_by_*`,
/// `instrument_spread_bp_by_*`, correlation shocks) are inert and produce a
/// warning, and `time_roll_forward` in `business_days` mode adjusts without
/// holiday information.
///
/// # Errors
///
/// Rejects malformed scenario, market, or model JSON, an invalid ISO `as_of`
/// date, an invalid scenario operation, missing market objects or hierarchy
/// context, statement-model execution failures, failure to encode the mutated
/// contexts, or failure to serialize the application envelope to JavaScript.
/// @param scenario_json - JSON-serialized ScenarioSpec to validate and apply.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param model_json - JSON-serialized FinancialModelSpec that scenario operations may mutate.
/// @param as_of - ISO-8601 valuation date used to resolve date-dependent market data.
#[wasm_bindgen(js_name = applyScenario)]
pub fn apply_scenario(
    scenario_json: &str,
    market_json: &str,
    model_json: &str,
    as_of: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let mut market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let mut model: finstack_quant_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let date = parse_iso_date(as_of)?;
    let report = apply_with_context(&spec, &mut market, Some(&mut model), date)?;
    let out =
        finstack_quant_scenarios::ApplicationEnvelope::from_contexts(report, &market, Some(&model))
            .map_err(to_js_err)?;
    crate::utils::to_js_value(&out)
}

/// Apply a scenario to a market context only (no model mutations).
///
/// Returns the same envelope shape as [`apply_scenario`] minus `model`;
/// the same caveats apply (no instrument portfolio, no holiday calendar).
///
/// # Errors
///
/// Rejects malformed scenario or market JSON, an invalid ISO `as_of` date, an
/// invalid scenario operation, missing market objects or hierarchy context,
/// failure to encode the mutated market, or failure to serialize the
/// application envelope to JavaScript.
/// @param scenario_json - JSON-serialized ScenarioSpec to validate and apply.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param as_of - ISO-8601 valuation date used to resolve date-dependent market data.
#[wasm_bindgen(js_name = applyScenarioToMarket)]
pub fn apply_scenario_to_market(
    scenario_json: &str,
    market_json: &str,
    as_of: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let mut market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let date = parse_iso_date(as_of)?;
    let report = apply_with_context(&spec, &mut market, None, date)?;
    let out = finstack_quant_scenarios::ApplicationEnvelope::from_contexts(report, &market, None)
        .map_err(to_js_err)?;
    crate::utils::to_js_value(&out)
}

/// Compute horizon total return under a scenario.
///
/// Applies a scenario specification to project an instrument forward, then
/// decomposes the resulting P&L using factor-based attribution.
///
/// # Arguments
///
/// * `instrument_json` - Canonical `finstack_quant.instrument/1` envelope.
/// * `market_json` - JSON-serialized `MarketContext`.
/// * `as_of` - Valuation date (ISO 8601).
/// * `scenario_json` - JSON-serialized `ScenarioSpec`.
/// * `method` - Attribution method: "parallel", "waterfall", "metrics_based", "taylor".
///
/// # Returns
///
/// The `HorizonResult` as a structured JavaScript object, matching the Python
/// binding's typed `HorizonResult`.
///
/// # Errors
///
/// Rejects malformed instrument, market, scenario, or configuration JSON; an
/// invalid ISO `as_of` date; an unsupported attribution `method`; an unknown
/// `calendar_id`; invalid, unsupported, or unresolved scenario operations;
/// missing market data; pricing or attribution failures; or failure to
/// serialize the horizon result to JavaScript.
/// @param config_json - Optional FinstackConfig JSON for horizon analysis; omit to use defaults.
/// @param calendar_id - Optional holiday calendar (e.g. "nyse", "target") used to
///   business-day adjust `time_roll_forward` targets under `business_days` mode.
///   Omit for a weekends-only calendar; unknown identifiers throw.
#[wasm_bindgen(js_name = computeHorizonReturn)]
pub fn compute_horizon_return(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    scenario_json: &str,
    method: Option<String>,
    config_json: Option<String>,
    calendar_id: Option<String>,
) -> Result<JsValue, JsValue> {
    use finstack_quant_attribution::AttributionMethod;
    use std::sync::Arc;

    // Parse instrument
    let boxed =
        finstack_quant_valuations::pricer::json::parse_boxed_instrument_json(instrument_json, None)
            .map_err(to_js_err)?;
    let instrument: Arc<dyn finstack_quant_valuations::instruments::Instrument> = Arc::from(boxed);

    // Parse market
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;

    // Parse date
    let date = parse_iso_date(as_of)?;

    // Parse scenario
    let scenario: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;

    // Parse method
    let method_str = method.as_deref().unwrap_or("parallel");
    let attribution_method = match method_str {
        "parallel" => AttributionMethod::Parallel,
        "waterfall" => {
            AttributionMethod::Waterfall(finstack_quant_attribution::default_waterfall_order())
        }
        "metrics_based" => AttributionMethod::MetricsBased,
        "taylor" => AttributionMethod::Taylor(
            finstack_quant_attribution::TaylorAttributionConfig::default(),
        ),
        other => {
            return Err(to_js_err(format!(
                "Unknown attribution method '{other}'. Expected: parallel, waterfall, metrics_based, taylor"
            )));
        }
    };

    // Parse config
    let finstack_config: finstack_quant_core::config::FinstackConfig = match config_json.as_deref()
    {
        Some(json) => serde_json::from_str(json).map_err(to_js_err)?,
        None => finstack_quant_core::config::FinstackConfig::default(),
    };

    let mut analyzer = finstack_quant_scenarios::horizon::HorizonAnalysis::new(
        attribution_method,
        finstack_config,
    );
    if let Some(id) = calendar_id.as_deref() {
        analyzer = analyzer.with_calendar_id(id);
    }
    let result = analyzer
        .compute(&instrument, &market, date, &scenario)
        .map_err(to_js_err)?;

    crate::utils::to_js_value(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_builtin_template_metadata_is_non_empty_json_array() {
        let json = list_builtin_template_metadata().expect("metadata");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse metadata");
        let arr = v.as_array().expect("array");
        assert!(!arr.is_empty());
    }

    #[test]
    fn build_from_template_and_build_template_component_succeed_for_builtin() {
        let meta = list_builtin_template_metadata().expect("metadata");
        let items: Vec<serde_json::Value> = serde_json::from_str(&meta).expect("parse");
        let first_id = items[0]["id"].as_str().expect("template id");
        let built = build_from_template(first_id).expect("build_from_template");
        assert!(!built.is_empty());

        let component_json =
            build_template_component("gfc_2008", "gfc_2008_rates").expect("component");
        assert!(!component_json.is_empty());
    }

    #[test]
    fn build_validate_parse_compose_roundtrip_empty_operations() {
        let spec_json =
            build_scenario_spec("test_id", "[]", Some("Test".to_string()), None, 0, None)
                .expect("build_scenario_spec");
        assert!(validate_scenario_spec(&spec_json).expect("validate"));
        let parsed = parse_scenario_spec(&spec_json).expect("parse");
        let before: serde_json::Value = serde_json::from_str(&spec_json).expect("before");
        let after: serde_json::Value = serde_json::from_str(&parsed).expect("after");
        assert_eq!(before, after);

        let composed = compose_scenarios("[]").expect("compose");
        assert!(validate_scenario_spec(&composed).expect("composed valid"));
    }

    #[test]
    fn build_scenario_with_name_and_description() {
        let spec_json = build_scenario_spec(
            "stress_1",
            "[]",
            Some("Stress scenario".to_string()),
            Some("A description".to_string()),
            10,
            Some("cumulative".to_string()),
        )
        .expect("build");
        let parsed: serde_json::Value = serde_json::from_str(&spec_json).expect("json");
        assert_eq!(parsed["id"], "stress_1");
        assert_eq!(parsed["priority"], 10);
        assert_eq!(parsed["resolution_mode"], "cumulative");
    }

    #[test]
    fn compose_multiple_scenarios() {
        let s1 = build_scenario_spec("s1", "[]", None, None, 0, None).expect("s1");
        let s2 = build_scenario_spec("s2", "[]", None, None, 1, None).expect("s2");
        let arr = format!("[{s1},{s2}]");
        let composed = compose_scenarios(&arr).expect("compose");
        assert!(validate_scenario_spec(&composed).expect("valid"));
    }

    #[test]
    fn compose_scenarios_rejects_duplicate_time_rolls() {
        use finstack_quant_core::market_data::hierarchy::ResolutionMode;
        use finstack_quant_scenarios::{OperationSpec, ScenarioSpec, TimeRollMode};

        let specs = serde_json::to_string(&vec![
            ScenarioSpec {
                id: "roll_1m".into(),
                name: None,
                description: None,
                operations: vec![OperationSpec::TimeRollForward {
                    period: "1M".into(),
                    apply_shocks: true,
                    roll_mode: TimeRollMode::BusinessDays,
                }],
                priority: 0,
                resolution_mode: ResolutionMode::Cumulative,
            },
            ScenarioSpec {
                id: "roll_3m".into(),
                name: None,
                description: None,
                operations: vec![OperationSpec::TimeRollForward {
                    period: "3M".into(),
                    apply_shocks: true,
                    roll_mode: TimeRollMode::BusinessDays,
                }],
                priority: 1,
                resolution_mode: ResolutionMode::Cumulative,
            },
        ])
        .expect("serialize specs");

        let err =
            compose_scenarios_json(&specs).expect_err("duplicate time rolls should be rejected");
        assert!(err.contains("TimeRollForward"), "unexpected error: {err}");
    }

    #[test]
    fn build_all_builtin_templates() {
        let meta = list_builtin_template_metadata().expect("metadata");
        let items: Vec<serde_json::Value> = serde_json::from_str(&meta).expect("parse");
        for item in &items {
            let id = item["id"].as_str().expect("id");
            let built = build_from_template(id).expect("build");
            assert!(!built.is_empty(), "template {id} produced empty output");
        }
    }
}
