//! Generates typed JSON Schema property definitions for all instrument types.
//!
//! For each instrument, this binary:
//! 1. Generates its JSON Schema using `schemars::schema_for!()`
//! 2. Reads the corresponding existing schema file
//! 3. Replaces `properties.instrument` with a fully typed version
//!    (discriminator `type` const + generated `spec` schema)
//! 4. Writes back the updated schema file, preserving all other fields

use finstack_quant_core::schema::{
    assemble_schema, postprocess_schema, write_schema as write_schema_file, JSON_SCHEMA_DIALECT,
};
use finstack_quant_valuations::instruments::*;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const COMMON_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/common/1/";
const DECIMAL_PATTERN: &str = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$";

#[derive(Clone, Copy)]
struct InstrumentSchemaEntry {
    name: &'static str,
    category: &'static str,
}

/// Locate the schemas directory relative to the crate root.
fn schemas_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("instruments")
        .join("1")
}

/// Locate the top-level schemas directory.
fn all_schemas_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    Path::new(&manifest_dir).join("schemas")
}

/// Locate the shared common-schema directory.
fn common_schemas_dir() -> PathBuf {
    all_schemas_dir().join("common").join("1")
}

/// Locate the canonical instrument fixture directory.
fn instrument_examples_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    Path::new(&manifest_dir)
        .join("tests")
        .join("instruments")
        .join("json_examples")
}

fn generated_instrument_fixture(name: &str) -> Value {
    let instrument = match name {
        "bermudan_swaption" => InstrumentJson::BermudanSwaption(BermudanSwaption::example()),
        "callable_range_accrual" => {
            InstrumentJson::CallableRangeAccrual(Box::new(CallableRangeAccrual::example()))
        }
        "cms_spread_option" => InstrumentJson::CmsSpreadOption(CmsSpreadOption::example()),
        "snowball" => InstrumentJson::Snowball(Snowball::example_snowball()),
        "tarn" => InstrumentJson::Tarn(Tarn::example()),
        _ => panic!("instrument schema {name} must contain examples[0]"),
    };
    serde_json::to_value(InstrumentEnvelope::new(instrument))
        .unwrap_or_else(|error| panic!("serialize generated fixture {name}: {error}"))
}

fn write_instrument_fixture(name: &str, schema: &Value) {
    let path = instrument_examples_dir().join(format!("{name}.json"));
    let example = schema
        .get("examples")
        .and_then(Value::as_array)
        .and_then(|examples| examples.first())
        .cloned()
        .unwrap_or_else(|| generated_instrument_fixture(name));
    let json = serde_json::to_string_pretty(&example)
        .unwrap_or_else(|error| panic!("serialize fixture {}: {error}", path.display()));
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write fixture {}: {error}", path.display()));
    println!("  updated {}", path.display());
}

/// Convert a snake_case name to a Title Case display name.
fn to_title(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper = first.to_uppercase().to_string();
                    upper + &chars.collect::<String>()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn common_schema_filename(def_name: &str) -> Option<&'static str> {
    match def_name {
        "Attributes" => Some("attributes.schema.json"),
        "BusinessDayConvention" => Some("business_day_convention.schema.json"),
        "Currency" => Some("currency.schema.json"),
        "DayCount" => Some("day_count.schema.json"),
        "Id" => Some("id.schema.json"),
        "Money" => Some("money.schema.json"),
        "PricingOverrides" => Some("pricing_overrides.schema.json"),
        "Tenor" => Some("tenor.schema.json"),
        _ => None,
    }
}

fn common_schema_ref(def_name: &str) -> Option<String> {
    common_schema_filename(def_name).map(|filename| format!("{COMMON_SCHEMA_BASE}{filename}"))
}

fn external_schema_ref(def_name: &str) -> Option<String> {
    common_schema_ref(def_name)
        .or_else(|| finstack_quant_cashflows::schema::definition_uri(def_name))
}

/// Read an existing schema file, merge the generated instrument schema, and write back.
fn update_schema_file(name: &str, category: &str, mut generated_schema: Value) {
    let base = schemas_dir();
    let path = base.join(category).join(format!("{name}.schema.json"));

    // Read existing file
    let existing: Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
    } else {
        panic!(
            "Schema file does not exist: {}. All schema files should already exist.",
            path.display()
        );
    };

    let existing_obj = existing
        .as_object()
        .expect("existing schema must be an object");
    postprocess_schema(&mut generated_schema, external_schema_ref);

    // Extract the generated schema's properties and required fields for embedding
    // into the spec sub-schema. Generated refs are document-root pointers
    // (`#/$defs/...`), so `$defs` must stay at the top level of the schema.
    let mut spec_schema = Map::new();

    if let Some(props) = generated_schema.get("properties") {
        spec_schema.insert("properties".to_string(), props.clone());
    }
    if let Some(req) = generated_schema.get("required") {
        spec_schema.insert("required".to_string(), req.clone());
    }
    if let Some(t) = generated_schema.get("type") {
        spec_schema.insert("type".to_string(), t.clone());
    }
    if let Some(additional) = generated_schema.get("additionalProperties") {
        spec_schema.insert("additionalProperties".to_string(), additional.clone());
    }
    for keyword in ["oneOf", "anyOf", "allOf", "not"] {
        if let Some(composition) = generated_schema.get(keyword) {
            spec_schema.insert(keyword.to_string(), composition.clone());
        }
    }
    let title = to_title(name);

    // Build the new properties.instrument value
    let instrument_value = json!({
        "description": format!("The {title} instrument definition"),
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "type": {
                "const": name,
                "type": "string"
            },
            "spec": Value::Object(spec_schema)
        },
        "required": ["type", "spec"]
    });

    // Build the output, preserving order of existing keys
    let mut output = Map::new();

    // Preserve known top-level keys from the existing file
    let preserve_keys = [
        "$id",
        "additionalProperties",
        "description",
        "examples",
        "title",
        "type",
    ];

    for key in &preserve_keys {
        if let Some(val) = existing_obj.get(*key) {
            output.insert((*key).to_string(), val.clone());
        }
    }
    output.insert(
        "$schema".to_string(),
        Value::String(JSON_SCHEMA_DIALECT.to_string()),
    );
    output
        .entry("additionalProperties".to_string())
        .or_insert(Value::Bool(false));

    // Carry forward generated `$defs` at document root to match schemars refs.
    if let Some(defs) = generated_schema.get("$defs") {
        output.insert("$defs".to_string(), defs.clone());
    }

    // Build properties: keep existing non-instrument properties, replace instrument
    let mut properties = Map::new();
    if let Some(existing_props) = existing_obj.get("properties").and_then(|v| v.as_object()) {
        for (k, v) in existing_props {
            if k != "instrument" {
                properties.insert(k.clone(), v.clone());
            }
        }
    }
    properties.insert(
        "schema".to_string(),
        json!({
            "const": "finstack_quant.instrument/1",
            "description": "Schema version identifier",
            "type": "string"
        }),
    );
    properties.insert("instrument".to_string(), instrument_value);
    output.insert("properties".to_string(), Value::Object(properties));

    output.insert("required".to_string(), json!(["schema", "instrument"]));

    let output = Value::Object(output);
    write_schema_file(&path, &output)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    write_instrument_fixture(name, &output);

    println!("  updated {}", path.display());
}

fn update_instrument_union_schema_file(entries: &[InstrumentSchemaEntry]) {
    let path = schemas_dir().join("instrument.schema.json");
    let existing: Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
    } else {
        json!({
            "$id": "https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json",
            "title": "Finstack Quant Instrument",
            "description": "Tagged union of all supported financial instruments"
        })
    };
    let existing_obj = existing
        .as_object()
        .expect("instrument union schema must be an object");

    let mut output = Map::new();
    for key in ["$id", "description", "title"] {
        if let Some(value) = existing_obj.get(key) {
            output.insert(key.to_string(), value.clone());
        }
    }
    output.insert(
        "$schema".to_string(),
        Value::String(JSON_SCHEMA_DIALECT.to_string()),
    );
    let mut entries = entries.to_vec();
    entries.sort_by_key(|entry| entry.name);
    let instrument_ref = |entry: &InstrumentSchemaEntry, fragment: &str| {
        json!({
            "$ref": format!(
                "https://finstack_quant.dev/schemas/instrument/1/{}/{}.schema.json{fragment}",
                entry.category, entry.name
            )
        })
    };
    output.insert(
        "$defs".to_string(),
        json!({
            "InstrumentJson": {
                "description": "Canonical tagged instrument payload without the persistence envelope.",
                "oneOf": entries
                    .iter()
                    .map(|entry| instrument_ref(entry, "#/properties/instrument"))
                    .collect::<Vec<_>>()
            }
        }),
    );
    output.insert(
        "oneOf".to_string(),
        Value::Array(
            entries
                .iter()
                .map(|entry| instrument_ref(entry, ""))
                .collect(),
        ),
    );

    write_schema_file(&path, &Value::Object(output))
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("  updated {}", path.display());
}

/// Update a standalone (non-instrument) schema file, replacing the top-level
/// typed properties with the schemars-generated schema.
fn update_standalone_schema_file(name: &str, subdir: &str, filename: &str, generated: Value) {
    let base = all_schemas_dir();
    let path = base.join(subdir).join(format!("{filename}.schema.json"));
    let mut generated = generated;
    postprocess_schema(&mut generated, external_schema_ref);

    let existing: Value = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
    } else {
        // Create minimal placeholder if file doesn't exist
        json!({
            "$id": format!("https://finstack_quant.dev/schemas/{subdir}/{filename}.schema.json"),
            "$schema": JSON_SCHEMA_DIALECT,
            "title": to_title(name),
            "description": format!("{} specification", to_title(name)),
            "type": "object"
        })
    };

    let output = assemble_schema(
        &existing,
        &generated,
        &["$id", "title", "description", "examples"],
        &[
            "type",
            "properties",
            "required",
            "$defs",
            "additionalProperties",
            "oneOf",
            "anyOf",
        ],
    )
    .unwrap_or_else(|error| panic!("assemble {name} schema: {error}"));
    write_schema_file(&path, &output)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));

    println!("  updated {}", path.display());
}

/// Update a shared common schema file from a schemars-generated type schema.
fn update_common_schema_file(title: &str, description: &str, filename: &str, generated: Value) {
    let dir = common_schemas_dir();
    let path = dir.join(filename);
    let mut schema = generated;
    postprocess_schema(&mut schema, external_schema_ref);
    let metadata = json!({
        "$id": format!("{COMMON_SCHEMA_BASE}{filename}"),
        "title": title,
        "description": description,
    });
    let output = assemble_schema(
        &metadata,
        &schema,
        &["$id", "title", "description"],
        &[
            "type",
            "format",
            "pattern",
            "additionalProperties",
            "properties",
            "required",
            "oneOf",
            "anyOf",
            "enum",
            "$defs",
        ],
    )
    .unwrap_or_else(|error| panic!("assemble {title} schema: {error}"));
    write_schema_file(&path, &output)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("  updated {}", path.display());
}

fn update_manual_common_schema_file(filename: &str, schema: Value) {
    let dir = common_schemas_dir();
    let path = dir.join(filename);
    write_schema_file(&path, &schema)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("  updated {}", path.display());
}

/// Generate a standalone schema for a type and update the corresponding file.
macro_rules! gen_standalone_schema {
    ($name:literal, $ty:ty, $subdir:literal, $filename:literal) => {{
        let schema = schemars::schema_for!($ty);
        let schema_value =
            serde_json::to_value(&schema).expect(concat!("serialize schema for ", $name));
        update_standalone_schema_file($name, $subdir, $filename, schema_value);
    }};
}

/// Generate a common schema for a canonical shared type.
macro_rules! gen_common_schema {
    ($title:literal, $description:literal, $ty:ty, $filename:literal) => {{
        let schema = schemars::schema_for!($ty);
        let schema_value =
            serde_json::to_value(&schema).expect(concat!("serialize schema for ", $title));
        update_common_schema_file($title, $description, $filename, schema_value);
    }};
}

/// Generate schema for a type and update the corresponding schema file.
macro_rules! gen_schema {
    ($entries:ident, $name:literal, $ty:ty, $category:literal) => {{
        let schema = schemars::schema_for!($ty);
        let schema_value =
            serde_json::to_value(&schema).expect(concat!("serialize schema for ", $name));
        update_schema_file($name, $category, schema_value);
        $entries.push(InstrumentSchemaEntry {
            name: $name,
            category: $category,
        });
    }};
}

fn main() {
    println!("Generating common schemas...\n");
    gen_common_schema!(
        "Attributes",
        "User-defined tags and key-value metadata for classification.",
        finstack_quant_core::types::Attributes,
        "attributes.schema.json"
    );
    gen_common_schema!(
        "Diagnostic",
        "One structured finding emitted while loading a persisted contract.",
        finstack_quant_core::contract::Diagnostic,
        "diagnostic.schema.json"
    );
    gen_common_schema!(
        "Validation Report",
        "Bounded structured diagnostics emitted by persisted-contract validation.",
        finstack_quant_core::contract::ValidationReport,
        "validation_report.schema.json"
    );
    gen_common_schema!(
        "Business Day Convention",
        "Business day adjustment convention.",
        finstack_quant_core::dates::BusinessDayConvention,
        "business_day_convention.schema.json"
    );
    gen_common_schema!(
        "Currency",
        "ISO 4217 currency code.",
        finstack_quant_core::currency::Currency,
        "currency.schema.json"
    );
    gen_common_schema!(
        "Day Count",
        "Day-count convention.",
        finstack_quant_core::dates::DayCount,
        "day_count.schema.json"
    );
    gen_common_schema!(
        "Money",
        "Currency-tagged monetary amount.",
        finstack_quant_core::money::Money,
        "money.schema.json"
    );
    let pricing_overrides_schema =
        finstack_quant_valuations::instruments::pricing_overrides::pricing_overrides_wire_schema();
    let pricing_overrides_schema_value = serde_json::to_value(&pricing_overrides_schema)
        .expect("serialize schema for Pricing Overrides");
    update_common_schema_file(
        "Pricing Overrides",
        "Per-instrument pricing and sensitivity override knobs.",
        "pricing_overrides.schema.json",
        pricing_overrides_schema_value,
    );
    gen_common_schema!(
        "Tenor",
        "A parsed financial tenor.",
        finstack_quant_core::dates::Tenor,
        "tenor.schema.json"
    );
    update_common_schema_file(
        "Id",
        "Opaque string identifier.",
        "id.schema.json",
        json!({ "type": "string" }),
    );
    update_manual_common_schema_file(
        "date.schema.json",
        json!({
            "$id": format!("{COMMON_SCHEMA_BASE}date.schema.json"),
            "$schema": JSON_SCHEMA_DIALECT,
            "title": "Date",
            "description": "ISO 8601 calendar date string.",
            "type": "string",
            "format": "date"
        }),
    );
    update_manual_common_schema_file(
        "decimal.schema.json",
        json!({
            "$id": format!("{COMMON_SCHEMA_BASE}decimal.schema.json"),
            "$schema": JSON_SCHEMA_DIALECT,
            "title": "Decimal",
            "description": "Decimal number encoded as a JSON string.",
            "type": "string",
            "pattern": DECIMAL_PATTERN
        }),
    );
    println!("\nDone! Updated common schema files.");

    println!("Generating instrument schemas...\n");
    let mut instrument_entries = Vec::new();

    // --- Fixed Income ---
    gen_schema!(instrument_entries, "bond", Bond, "fixed_income");
    gen_schema!(
        instrument_entries,
        "convertible_bond",
        ConvertibleBond,
        "fixed_income"
    );
    gen_schema!(
        instrument_entries,
        "inflation_linked_bond",
        InflationLinkedBond,
        "fixed_income"
    );
    gen_schema!(instrument_entries, "term_loan", TermLoan, "fixed_income");
    gen_schema!(
        instrument_entries,
        "revolving_credit",
        RevolvingCredit,
        "fixed_income"
    );
    gen_schema!(
        instrument_entries,
        "bond_future",
        BondFuture,
        "fixed_income"
    );
    gen_schema!(
        instrument_entries,
        "agency_mbs_passthrough",
        AgencyMbsPassthrough,
        "fixed_income"
    );
    gen_schema!(instrument_entries, "agency_tba", AgencyTba, "fixed_income");
    gen_schema!(instrument_entries, "agency_cmo", AgencyCmo, "fixed_income");
    gen_schema!(
        instrument_entries,
        "dollar_roll",
        DollarRoll,
        "fixed_income"
    );
    gen_schema!(
        instrument_entries,
        "trs_fixed_income_index",
        FIIndexTotalReturnSwap,
        "fixed_income"
    );
    gen_schema!(
        instrument_entries,
        "structured_credit",
        StructuredCredit,
        "fixed_income"
    );

    // --- Rates ---
    gen_schema!(
        instrument_entries,
        "interest_rate_swap",
        InterestRateSwap,
        "rates"
    );
    gen_schema!(instrument_entries, "basis_swap", BasisSwap, "rates");
    gen_schema!(instrument_entries, "xccy_swap", XccySwap, "rates");
    gen_schema!(instrument_entries, "inflation_swap", InflationSwap, "rates");
    gen_schema!(
        instrument_entries,
        "yoy_inflation_swap",
        YoYInflationSwap,
        "rates"
    );
    gen_schema!(
        instrument_entries,
        "inflation_cap_floor",
        InflationCapFloor,
        "rates"
    );
    gen_schema!(
        instrument_entries,
        "forward_rate_agreement",
        ForwardRateAgreement,
        "rates"
    );
    gen_schema!(instrument_entries, "swaption", Swaption, "rates");
    gen_schema!(
        instrument_entries,
        "bermudan_swaption",
        BermudanSwaption,
        "rates"
    );
    gen_schema!(
        instrument_entries,
        "interest_rate_future",
        InterestRateFuture,
        "rates"
    );
    gen_schema!(instrument_entries, "cap_floor", CapFloor, "rates");
    gen_schema!(instrument_entries, "cms_option", CmsOption, "rates");
    gen_schema!(
        instrument_entries,
        "cms_spread_option",
        CmsSpreadOption,
        "rates"
    );
    gen_schema!(instrument_entries, "cms_swap", CmsSwap, "rates");
    gen_schema!(
        instrument_entries,
        "ir_future_option",
        IrFutureOption,
        "rates"
    );
    gen_schema!(instrument_entries, "deposit", Deposit, "rates");
    gen_schema!(instrument_entries, "repo", Repo, "rates");
    gen_schema!(instrument_entries, "range_accrual", RangeAccrual, "rates");
    gen_schema!(
        instrument_entries,
        "callable_range_accrual",
        CallableRangeAccrual,
        "rates"
    );
    gen_schema!(instrument_entries, "snowball", Snowball, "rates");
    gen_schema!(instrument_entries, "tarn", Tarn, "rates");

    // --- Credit Derivatives ---
    gen_schema!(
        instrument_entries,
        "credit_default_swap",
        CreditDefaultSwap,
        "credit_derivatives"
    );
    gen_schema!(
        instrument_entries,
        "cds_index",
        CDSIndex,
        "credit_derivatives"
    );
    gen_schema!(
        instrument_entries,
        "cds_tranche",
        CDSTranche,
        "credit_derivatives"
    );
    gen_schema!(
        instrument_entries,
        "cds_option",
        CDSOption,
        "credit_derivatives"
    );

    // --- Equity ---
    gen_schema!(instrument_entries, "equity", Equity, "equity");
    gen_schema!(instrument_entries, "equity_option", EquityOption, "equity");
    gen_schema!(instrument_entries, "autocallable", Autocallable, "equity");
    gen_schema!(
        instrument_entries,
        "cliquet_option",
        CliquetOption,
        "equity"
    );
    gen_schema!(instrument_entries, "variance_swap", VarianceSwap, "equity");
    gen_schema!(
        instrument_entries,
        "equity_index_future",
        EquityIndexFuture,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "volatility_index_future",
        VolatilityIndexFuture,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "volatility_index_option",
        VolatilityIndexOption,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "trs_equity",
        EquityTotalReturnSwap,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "private_markets_fund",
        PrivateMarketsFund,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "real_estate_asset",
        RealEstateAsset,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "discounted_cash_flow",
        DiscountedCashFlow,
        "equity"
    );
    gen_schema!(
        instrument_entries,
        "levered_real_estate_equity",
        LeveredRealEstateEquity,
        "equity"
    );

    // --- FX ---
    gen_schema!(instrument_entries, "fx_spot", FxSpot, "fx");
    gen_schema!(instrument_entries, "fx_swap", FxSwap, "fx");
    gen_schema!(instrument_entries, "fx_forward", FxForward, "fx");
    gen_schema!(instrument_entries, "ndf", Ndf, "fx");
    gen_schema!(instrument_entries, "fx_option", FxOption, "fx");
    gen_schema!(
        instrument_entries,
        "fx_digital_option",
        FxDigitalOption,
        "fx"
    );
    gen_schema!(instrument_entries, "fx_touch_option", FxTouchOption, "fx");
    gen_schema!(
        instrument_entries,
        "fx_barrier_option",
        FxBarrierOption,
        "fx"
    );
    gen_schema!(instrument_entries, "fx_variance_swap", FxVarianceSwap, "fx");
    gen_schema!(instrument_entries, "quanto_option", QuantoOption, "fx");

    // --- Commodity ---
    gen_schema!(
        instrument_entries,
        "commodity_option",
        CommodityOption,
        "commodity"
    );
    gen_schema!(
        instrument_entries,
        "commodity_asian_option",
        CommodityAsianOption,
        "commodity"
    );
    gen_schema!(
        instrument_entries,
        "commodity_forward",
        CommodityForward,
        "commodity"
    );
    gen_schema!(
        instrument_entries,
        "commodity_swap",
        CommoditySwap,
        "commodity"
    );
    gen_schema!(
        instrument_entries,
        "commodity_swaption",
        CommoditySwaption,
        "commodity"
    );
    gen_schema!(
        instrument_entries,
        "commodity_spread_option",
        CommoditySpreadOption,
        "commodity"
    );

    // --- Exotics ---
    gen_schema!(instrument_entries, "asian_option", AsianOption, "exotics");
    gen_schema!(
        instrument_entries,
        "barrier_option",
        BarrierOption,
        "exotics"
    );
    gen_schema!(
        instrument_entries,
        "lookback_option",
        LookbackOption,
        "exotics"
    );
    gen_schema!(instrument_entries, "basket", Basket, "exotics");

    let registry_names: BTreeSet<&str> = registry_tags().iter().map(|(tag, _)| *tag).collect();
    let generated_names: BTreeSet<&str> =
        instrument_entries.iter().map(|entry| entry.name).collect();
    assert_eq!(
        generated_names, registry_names,
        "schema generator entries must match the canonical instrument registry exactly"
    );
    let registry_count = registry_names.len();
    update_instrument_union_schema_file(&instrument_entries);

    println!("\nDone! Updated {registry_count} instrument schema files.");

    // =========================================================================
    // Non-instrument schemas owned by valuations (calibration, market, results)
    // =========================================================================
    println!("\nGenerating non-instrument schemas...\n");

    // The on-disk v2 schema is frozen for historical/parity tests; the current
    // Rust `CalibrationEnvelope` reflects the v3 shape (flat market_data /
    // prior_market lists, no initial_market), so the generator only targets v3.
    gen_standalone_schema!(
        "calibration",
        finstack_quant_valuations::calibration::api::schema::CalibrationEnvelope,
        "calibration/3",
        "calibration"
    );
    gen_standalone_schema!(
        "valuation_result",
        finstack_quant_valuations::results::ValuationResult,
        "results/1",
        "valuation_result"
    );

    // Market quotes
    gen_standalone_schema!(
        "market_quote",
        finstack_quant_valuations::market::quotes::market_quote::MarketQuote,
        "market/1",
        "market_quote"
    );

    println!("\nDone! Updated all schemas.");
}
