//! WASM bindings for CDS-family instrument example payloads.
//!
//! Mirrors `finstack-quant-py/src/bindings/valuations/credit_derivatives.rs`. This
//! file exists so the Rust source tree matches the Python wrapper layout.
//!
//! Pricing / validation / serialization for CDS instruments is provided by the
//! generic `priceInstrument` and
//! `validateInstrumentJson` entry points exposed from the JS facade at
//! `valuations.instruments`; this module only owns the example-payload
//! factories.

use crate::utils::to_js_err;
use finstack_quant_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_quant_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_quant_valuations::instruments::credit_derivatives::cds_tranche::CDSTranche;
use finstack_quant_valuations::instruments::{InstrumentEnvelope, InstrumentJson};
use wasm_bindgen::prelude::*;

fn serialize_example(instrument: InstrumentJson) -> Result<String, JsValue> {
    serde_json::to_string(&InstrumentEnvelope::new(instrument)).map_err(to_js_err)
}

/// Example `CreditDefaultSwap` canonical instrument envelope.
///
/// # Errors
///
/// Throws a JavaScript exception if the example envelope cannot be serialized
/// to JSON.
#[wasm_bindgen(js_name = creditDefaultSwapExampleJson)]
pub fn credit_default_swap_example_json() -> Result<String, JsValue> {
    serialize_example(InstrumentJson::CreditDefaultSwap(
        CreditDefaultSwap::example(),
    ))
}

/// Example `CDSIndex` canonical instrument envelope.
///
/// # Errors
///
/// Throws a JavaScript exception if the example envelope cannot be serialized
/// to JSON.
#[wasm_bindgen(js_name = cdsIndexExampleJson)]
pub fn cds_index_example_json() -> Result<String, JsValue> {
    serialize_example(InstrumentJson::CDSIndex(CDSIndex::example()))
}

/// Example `CDSTranche` canonical instrument envelope.
///
/// # Errors
///
/// Throws a JavaScript exception if the example envelope cannot be serialized
/// to JSON.
#[wasm_bindgen(js_name = cdsTrancheExampleJson)]
pub fn cds_tranche_example_json() -> Result<String, JsValue> {
    serialize_example(InstrumentJson::CDSTranche(CDSTranche::example()))
}

/// Example `CDSOption` canonical instrument envelope.
///
/// # Errors
///
/// Throws a JavaScript exception if the example option cannot be constructed
/// or its envelope cannot be serialized to JSON.
#[wasm_bindgen(js_name = cdsOptionExampleJson)]
pub fn cds_option_example_json() -> Result<String, JsValue> {
    let option = CDSOption::example().map_err(to_js_err)?;
    serialize_example(InstrumentJson::CDSOption(option))
}
