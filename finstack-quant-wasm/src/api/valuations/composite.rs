//! WASM façade for composite initialization, rebalancing, decomposition, and history.

use crate::utils::{to_js_err, to_js_error, to_js_value};
use finstack_quant_valuations::instruments::composite::{
    CompositeHistoryEngine, CompositeInstrument, CompositeMarketObservation,
    CompositeRebalanceResult, CompositeSpec,
};
use finstack_quant_valuations::instruments::{InstrumentEnvelope, InstrumentJson};
use finstack_quant_valuations::metrics::MetricId;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct CompositeRebalanceWire {
    instrument: InstrumentEnvelope,
    trades: Vec<finstack_quant_valuations::instruments::composite::CompositeTrade>,
}

fn parse_spec(json: &str) -> Result<CompositeSpec, JsValue> {
    serde_json::from_str(json).map_err(to_js_err)
}

fn parse_composite(json: &str) -> Result<CompositeInstrument, JsValue> {
    match finstack_quant_valuations::pricer::json::parse_instrument_json(json)
        .map_err(|error| to_js_error(&error))?
    {
        InstrumentJson::Composite(instrument) => Ok(*instrument),
        other => Err(JsValue::from_str(&format!(
            "expected composite instrument envelope, found '{}'",
            other.type_tag()
        ))),
    }
}

fn parse_observations(json: Option<&str>) -> Result<Vec<CompositeMarketObservation>, JsValue> {
    serde_json::from_str(json.unwrap_or("[]")).map_err(to_js_err)
}

fn parse_metrics(metrics: Option<JsValue>) -> Result<Vec<MetricId>, JsValue> {
    match metrics {
        None => Ok(Vec::new()),
        Some(value) if value.is_null() || value.is_undefined() => Ok(Vec::new()),
        Some(value) => serde_wasm_bindgen::from_value::<Vec<String>>(value)
            .map(|values| values.into_iter().map(MetricId::custom).collect())
            .map_err(to_js_err),
    }
}

fn rebalance_value(result: CompositeRebalanceResult) -> Result<JsValue, JsValue> {
    to_js_value(&CompositeRebalanceWire {
        instrument: InstrumentEnvelope::new(InstrumentJson::Composite(Box::new(result.instrument))),
        trades: result.trades,
    })
}

/// Resolve an unresolved composite specification into a priceable instrument.
///
/// @param spec_json - Bare canonical `CompositeSpec` JSON.
/// @param market_json - Complete canonical market-context JSON at the effective date.
/// @param as_of - ISO-8601 effective date for the resolved holdings state.
/// @param history_json - Optional chronological `CompositeMarketObservation[]` JSON available through `asOf`.
/// @returns Object containing a canonical composite instrument envelope and primitive establishment trades.
///
/// # Arguments
///
/// * `spec_json` - Bare serialized composite definition with embedded instruments.
/// * `market_json` - Complete market context used to resolve dynamic quantities.
/// * `as_of` - ISO-8601 state effective date; no later history is permitted.
/// * `history_json` - Optional strictly increasing dated observations for volatility or expression inputs.
///
/// # Errors
///
/// Throws for malformed JSON, invalid specifications, missing market/history
/// inputs, unsupported metrics/notionals, or non-finite resolved quantities.
#[wasm_bindgen(js_name = initializeComposite)]
pub fn initialize_composite(
    spec_json: &str,
    market_json: &str,
    as_of: &str,
    history_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let spec = parse_spec(spec_json)?;
    let market = super::pricing::parse_market_json(market_json)?;
    let date = finstack_quant_core::dates::parse_iso_date(as_of).map_err(to_js_err)?;
    let history = parse_observations(history_json.as_deref())?;
    let result = spec
        .initialize(&market, date, &history)
        .map_err(|error| to_js_error(&error))?;
    rebalance_value(result)
}

/// Explicitly rebalance a resolved composite without mutating the prior state.
///
/// @param instrument_json - Canonical resolved composite instrument envelope.
/// @param market_json - Complete canonical market-context JSON at the rebalance date.
/// @param as_of - ISO-8601 effective date for the new state.
/// @param history_json - Optional chronological observation JSON available through `asOf`.
/// @returns Object containing the new envelope and net primitive quantity deltas.
///
/// # Arguments
///
/// * `instrument_json` - Existing resolved composite envelope used as the trade baseline.
/// * `market_json` - Complete market context used to resolve new quantities.
/// * `as_of` - ISO-8601 effective date for the distinct returned state.
/// * `history_json` - Optional strictly increasing dated observations for dynamic inputs.
///
/// # Errors
///
/// Throws for malformed inputs, a non-composite envelope, invalid history,
/// missing market data, or quantity-resolution failures.
#[wasm_bindgen(js_name = rebalanceComposite)]
pub fn rebalance_composite(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    history_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let instrument = parse_composite(instrument_json)?;
    let market = super::pricing::parse_market_json(market_json)?;
    let date = finstack_quant_core::dates::parse_iso_date(as_of).map_err(to_js_err)?;
    let history = parse_observations(history_json.as_deref())?;
    let result = instrument
        .rebalance(&market, date, &history)
        .map_err(|error| to_js_error(&error))?;
    rebalance_value(result)
}

/// Return path-level plus net/gross primitive value and additive risk.
///
/// @param instrument_json - Canonical resolved composite instrument envelope.
/// @param market_json - Complete canonical market-context JSON used for primitive pricing and FX.
/// @param as_of - ISO-8601 valuation date.
/// @param metrics - Optional additive metric identifier array.
/// @returns Plain object containing primitive paths and net/gross aggregates.
///
/// # Arguments
///
/// * `instrument_json` - Resolved composite whose frozen quantities are decomposed.
/// * `market_json` - Complete valuation and FX context.
/// * `as_of` - ISO-8601 valuation date.
/// * `metrics` - Optional JavaScript array of additive metric identifiers.
///
/// # Errors
///
/// Throws for invalid input, non-additive metrics, missing market data, or
/// primitive valuation failures.
#[wasm_bindgen(js_name = compositePrimitiveExposures)]
pub fn composite_primitive_exposures(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    metrics: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let instrument = parse_composite(instrument_json)?;
    let market = super::pricing::parse_market_json(market_json)?;
    let date = finstack_quant_core::dates::parse_iso_date(as_of).map_err(to_js_err)?;
    let metrics = parse_metrics(metrics)?;
    let report = instrument
        .primitive_exposure_report(&market, date, &metrics)
        .map_err(|error| to_js_error(&error))?;
    to_js_value(&report)
}

/// Flatten current holdings or a state transition into executable primitive deltas.
///
/// @param instrument_json - Canonical target composite instrument envelope.
/// @param previous_instrument_json - Optional canonical prior composite envelope; omit for establishment trades.
/// @returns Primitive trade array with signed quantity deltas.
///
/// # Arguments
///
/// * `instrument_json` - Target resolved composite state.
/// * `previous_instrument_json` - Optional prior resolved state used to calculate net deltas.
///
/// # Errors
///
/// Throws for malformed or non-composite envelopes, conflicting primitive
/// definitions, or invalid frozen states.
#[wasm_bindgen(js_name = compositeExecutionTrades)]
pub fn composite_execution_trades(
    instrument_json: &str,
    previous_instrument_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let instrument = parse_composite(instrument_json)?;
    let previous = previous_instrument_json
        .as_deref()
        .map(parse_composite)
        .transpose()?;
    let trades = instrument
        .execution_trades(previous.as_ref())
        .map_err(|error| to_js_error(&error))?;
    to_js_value(&trades)
}

/// Initialize a specification at the first snapshot and calculate dated history.
///
/// @param spec_json - Bare canonical `CompositeSpec` JSON.
/// @param observations_json - Strictly increasing output observation array JSON.
/// @param warmup_json - Optional strictly earlier observation array used only for weighting inputs.
/// @param metrics - Optional additive primitive metric identifier array.
/// @returns Chronological rows containing value, cashflows, P&L, return, index, exposures, state dates, and trades.
///
/// # Arguments
///
/// * `spec_json` - Unresolved composite specification.
/// * `observations_json` - Complete chronological market snapshots to report.
/// * `warmup_json` - Optional complete snapshots strictly preceding the output period.
/// * `metrics` - Optional additive primitive metrics reported on every row.
///
/// # Errors
///
/// Throws for empty, duplicate, or unordered observations; overlapping warmup;
/// initialization/rebalance failures; or missing market/history inputs.
#[wasm_bindgen(js_name = compositeHistoryFromSpec)]
pub fn composite_history_from_spec(
    spec_json: &str,
    observations_json: &str,
    warmup_json: Option<String>,
    metrics: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let spec = parse_spec(spec_json)?;
    let observations = parse_observations(Some(observations_json))?;
    let warmup = parse_observations(warmup_json.as_deref())?;
    let metrics = parse_metrics(metrics)?;
    let rows = CompositeHistoryEngine::run_from_spec(&spec, &warmup, &observations, &metrics)
        .map_err(|error| to_js_error(&error))?;
    to_js_value(&rows)
}

/// Calculate dated history from an already-resolved composite state.
///
/// @param instrument_json - Canonical resolved composite instrument envelope.
/// @param observations_json - Strictly increasing complete market observation array JSON.
/// @param metrics - Optional additive primitive metric identifier array.
/// @returns Chronological composite history rows.
///
/// # Arguments
///
/// * `instrument_json` - Resolved immutable initial holdings state.
/// * `observations_json` - Complete chronological market snapshots to report.
/// * `metrics` - Optional additive primitive metrics reported on every row.
///
/// # Errors
///
/// Throws for invalid states, empty/duplicate/unordered observations, missing
/// market data, or valuation and rebalance failures.
#[wasm_bindgen(js_name = compositeHistory)]
pub fn composite_history(
    instrument_json: &str,
    observations_json: &str,
    metrics: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let instrument = parse_composite(instrument_json)?;
    let observations = parse_observations(Some(observations_json))?;
    let metrics = parse_metrics(metrics)?;
    let rows = CompositeHistoryEngine::run(&instrument, &observations, &metrics)
        .map_err(|error| to_js_error(&error))?;
    to_js_value(&rows)
}
