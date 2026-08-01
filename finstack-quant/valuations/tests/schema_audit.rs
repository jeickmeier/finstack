//! Schema tests: verify every instrument example() serializes to valid JSON
//! and roundtrips through InstrumentEnvelope serde.

use finstack_quant_valuations::instruments::json_loader::{InstrumentEnvelope, InstrumentJson};

macro_rules! test_roundtrip {
    (plain: $test_name:ident, $variant:ident, $expr:expr) => {
        #[test]
        #[allow(clippy::expect_used)]
        fn $test_name() {
            let envelope = InstrumentEnvelope {
                schema:
                    finstack_quant_valuations::instruments::json_loader::InstrumentSchema::CURRENT,
                instrument: InstrumentJson::$variant($expr),
            };
            let json = serde_json::to_string(&envelope).expect("serialize");
            let parsed: InstrumentEnvelope = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed.schema.as_str(), "finstack_quant.instrument/1");
            // Re-serialize and verify stability
            let json2 = serde_json::to_string(&parsed).expect("re-serialize");
            assert_eq!(json, json2, "roundtrip should be stable");
        }
    };
    (boxed: $test_name:ident, $variant:ident, $expr:expr) => {
        #[test]
        #[allow(clippy::expect_used)]
        fn $test_name() {
            let envelope = InstrumentEnvelope {
                schema:
                    finstack_quant_valuations::instruments::json_loader::InstrumentSchema::CURRENT,
                instrument: InstrumentJson::$variant(Box::new($expr)),
            };
            let json = serde_json::to_string(&envelope).expect("serialize");
            let parsed: InstrumentEnvelope = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed.schema.as_str(), "finstack_quant.instrument/1");
            let json2 = serde_json::to_string(&parsed).expect("re-serialize");
            assert_eq!(json, json2, "roundtrip should be stable");
        }
    };
}

mod schema_roundtrip {
    use super::*;
    use finstack_quant_valuations::instruments::*;
    use serde_json::Value;
    use std::path::Path;

    // Fixed Income
    test_roundtrip!(plain: bond, Bond, Bond::example().expect("bond"));
    test_roundtrip!(plain: convertible_bond, ConvertibleBond, ConvertibleBond::example().expect("cb"));
    test_roundtrip!(plain: inflation_linked_bond, InflationLinkedBond, InflationLinkedBond::example());
    test_roundtrip!(plain: term_loan, TermLoan, TermLoan::example().expect("tl"));
    test_roundtrip!(plain: revolving_credit, RevolvingCredit, RevolvingCredit::example().expect("rc"));
    test_roundtrip!(boxed: bond_future, BondFuture, BondFuture::example().expect("bf"));
    test_roundtrip!(plain: agency_mbs_passthrough, AgencyMbsPassthrough, AgencyMbsPassthrough::example().expect("mbs"));
    test_roundtrip!(plain: agency_tba, AgencyTba, AgencyTba::example().expect("tba"));
    test_roundtrip!(plain: agency_cmo, AgencyCmo, AgencyCmo::example().expect("cmo"));
    test_roundtrip!(plain: agency_cmo_accrual, AgencyCmo, AgencyCmo::example_accrual().expect("cmo z"));
    test_roundtrip!(plain: dollar_roll, DollarRoll, DollarRoll::example().expect("dr"));
    test_roundtrip!(plain: trs_fixed_income_index, TrsFixedIncomeIndex, FIIndexTotalReturnSwap::example().expect("fitrs"));
    test_roundtrip!(boxed: structured_credit, StructuredCredit, StructuredCredit::example());

    // Rates
    test_roundtrip!(plain: interest_rate_swap, InterestRateSwap, InterestRateSwap::example_standard().expect("irs"));
    test_roundtrip!(plain: basis_swap, BasisSwap, BasisSwap::example().expect("bs"));
    test_roundtrip!(plain: xccy_swap, XccySwap, XccySwap::example());
    test_roundtrip!(plain: inflation_swap, InflationSwap, InflationSwap::example());
    test_roundtrip!(plain: yoy_inflation_swap, YoYInflationSwap, YoYInflationSwap::example());
    test_roundtrip!(plain: inflation_cap_floor, InflationCapFloor, InflationCapFloor::example());
    test_roundtrip!(plain: forward_rate_agreement, ForwardRateAgreement, ForwardRateAgreement::example().expect("fra"));
    test_roundtrip!(plain: swaption, Swaption, Swaption::example());
    test_roundtrip!(plain: bermudan_swaption, BermudanSwaption, BermudanSwaption::example());
    test_roundtrip!(plain: interest_rate_future, InterestRateFuture, InterestRateFuture::example().expect("irf"));
    test_roundtrip!(plain: cap_floor, CapFloor, CapFloor::example().expect("cap floor"));
    test_roundtrip!(plain: cms_option, CmsOption, CmsOption::example());
    test_roundtrip!(plain: cms_spread_option, CmsSpreadOption, CmsSpreadOption::example());
    test_roundtrip!(plain: cms_swap, CmsSwap, CmsSwap::example());
    test_roundtrip!(plain: ir_future_option, IrFutureOption, IrFutureOption::example().expect("irfo"));
    test_roundtrip!(plain: deposit, Deposit, Deposit::example().expect("dep"));
    test_roundtrip!(plain: repo, Repo, Repo::example());
    test_roundtrip!(plain: range_accrual, RangeAccrual, RangeAccrual::example());
    test_roundtrip!(boxed: callable_range_accrual, CallableRangeAccrual, CallableRangeAccrual::example());
    test_roundtrip!(plain: snowball, Snowball, Snowball::example_snowball());
    test_roundtrip!(plain: tarn, Tarn, Tarn::example());

    // Credit
    test_roundtrip!(plain: credit_default_swap, CreditDefaultSwap, CreditDefaultSwap::example());
    test_roundtrip!(plain: cds_index, CDSIndex, CDSIndex::example());
    test_roundtrip!(plain: cds_tranche, CDSTranche, CDSTranche::example());
    test_roundtrip!(plain: cds_option, CDSOption, CDSOption::example().expect("cdso"));

    #[test]
    #[allow(clippy::expect_used)]
    fn cds_option_schema_example_matches_canonical_json() {
        let envelope = InstrumentEnvelope {
            schema: finstack_quant_valuations::instruments::json_loader::InstrumentSchema::CURRENT,
            instrument: InstrumentJson::CDSOption(CDSOption::example().expect("cdso")),
        };
        let canonical = serde_json::to_value(envelope).expect("serialize cds option example");

        let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("schemas")
            .join("instruments")
            .join("1")
            .join("credit_derivatives")
            .join("cds_option.schema.json");
        let schema_text = std::fs::read_to_string(&schema_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
        let schema: Value = serde_json::from_str(&schema_text)
            .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));
        let checked_in = schema
            .get("examples")
            .and_then(Value::as_array)
            .and_then(|examples| examples.first())
            .unwrap_or_else(|| panic!("{} missing examples[0]", schema_path.display()));

        assert_eq!(
            checked_in,
            &canonical,
            "cds_option schema example is stale; expected canonical JSON:\n{}",
            serde_json::to_string_pretty(&canonical).expect("pretty canonical json")
        );
    }

    // Equity
    test_roundtrip!(plain: equity, Equity, Equity::example());
    test_roundtrip!(plain: equity_option, EquityOption, EquityOption::example().expect("eqopt"));
    test_roundtrip!(plain: autocallable, Autocallable, Autocallable::example().expect("auto"));
    test_roundtrip!(plain: cliquet_option, CliquetOption, CliquetOption::example().expect("cliq"));
    test_roundtrip!(plain: variance_swap, VarianceSwap, VarianceSwap::example().expect("vs"));
    test_roundtrip!(plain: equity_index_future, EquityIndexFuture, EquityIndexFuture::example().expect("eif"));
    test_roundtrip!(plain: volatility_index_future, VolatilityIndexFuture, VolatilityIndexFuture::example().expect("vif"));
    test_roundtrip!(plain: volatility_index_option, VolatilityIndexOption, VolatilityIndexOption::example().expect("vio"));
    test_roundtrip!(plain: trs_equity, TrsEquity, EquityTotalReturnSwap::example().expect("etrs"));
    test_roundtrip!(plain: private_markets_fund, PrivateMarketsFund, PrivateMarketsFund::example().expect("pmf"));
    test_roundtrip!(plain: real_estate_asset, RealEstateAsset, RealEstateAsset::example().expect("rea"));
    test_roundtrip!(plain: discounted_cash_flow, DiscountedCashFlow, DiscountedCashFlow::example().expect("dcf"));
    test_roundtrip!(boxed: levered_real_estate_equity, LeveredRealEstateEquity,
        finstack_quant_valuations::instruments::equity::real_estate::LeveredRealEstateEquity::example().expect("lre"));

    // FX
    test_roundtrip!(plain: fx_spot, FxSpot, FxSpot::example().expect("fxs"));
    test_roundtrip!(plain: fx_swap, FxSwap, FxSwap::example());
    test_roundtrip!(plain: fx_forward, FxForward, FxForward::example().expect("fxf"));
    test_roundtrip!(plain: ndf, Ndf, Ndf::example());
    test_roundtrip!(plain: fx_option, FxOption, FxOption::example().expect("fxo"));
    test_roundtrip!(plain: fx_digital_option, FxDigitalOption, FxDigitalOption::example().expect("fxdo"));
    test_roundtrip!(plain: fx_touch_option, FxTouchOption, FxTouchOption::example().expect("fxto"));
    test_roundtrip!(plain: fx_barrier_option, FxBarrierOption, FxBarrierOption::example());
    test_roundtrip!(plain: fx_variance_swap, FxVarianceSwap, FxVarianceSwap::example());
    test_roundtrip!(plain: quanto_option, QuantoOption, QuantoOption::example());

    // Commodity
    test_roundtrip!(plain: commodity_option, CommodityOption, CommodityOption::example());
    test_roundtrip!(plain: commodity_asian_option, CommodityAsianOption, CommodityAsianOption::example());
    test_roundtrip!(plain: commodity_forward, CommodityForward, CommodityForward::example());
    test_roundtrip!(plain: commodity_swap, CommoditySwap, CommoditySwap::example());
    test_roundtrip!(plain: commodity_swaption, CommoditySwaption, CommoditySwaption::example());
    test_roundtrip!(plain: commodity_spread_option, CommoditySpreadOption, CommoditySpreadOption::example().expect("cso"));

    // Exotics
    test_roundtrip!(plain: asian_option, AsianOption, AsianOption::example().expect("ao"));
    test_roundtrip!(plain: barrier_option, BarrierOption, BarrierOption::example().expect("bo"));
    test_roundtrip!(plain: lookback_option, LookbackOption, LookbackOption::example().expect("lo"));
    test_roundtrip!(plain: basket, Basket, Basket::example().expect("bsk"));
}

mod generated_schema_contract {
    #![allow(clippy::expect_used)]

    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    const JSON_SCHEMA_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
    const SCHEMA_ID_HOST: &str = "https://finstack_quant.dev/";
    const DECIMAL_PATTERN: &str = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$";
    const COMMON_SCHEMA_HOST: &str = "https://finstack_quant.dev/schemas/common/1/";
    const CASHFLOW_SCHEMA_HOST: &str = "https://finstack_quant.dev/schemas/cashflow/1/";
    const COMMON_SCHEMA_FILES: &[(&str, &str)] = &[
        ("Attributes", "attributes.schema.json"),
        (
            "BusinessDayConvention",
            "business_day_convention.schema.json",
        ),
        ("Currency", "currency.schema.json"),
        ("Date", "date.schema.json"),
        ("DayCount", "day_count.schema.json"),
        ("Decimal", "decimal.schema.json"),
        ("Id", "id.schema.json"),
        ("Money", "money.schema.json"),
        (
            "InstrumentPricingOverrides",
            "instrument_pricing_overrides.schema.json",
        ),
        (
            "MetricPricingOverrides",
            "metric_pricing_overrides.schema.json",
        ),
        (
            "ScenarioPricingOverrides",
            "scenario_pricing_overrides.schema.json",
        ),
        ("Tenor", "tenor.schema.json"),
    ];
    const CASHFLOW_SCHEMA_FILES: &[(&str, &str)] = &[
        ("DefaultModelSpec", "default_model_spec.schema.json"),
        ("FeeSpec", "fee_specs.schema.json"),
        ("FixedCouponSpec", "coupon_specs.schema.json"),
        ("PrepaymentModelSpec", "prepayment_model_spec.schema.json"),
        ("RecoveryModelSpec", "recovery_model_spec.schema.json"),
        ("ScheduleParams", "schedule_params.schema.json"),
    ];

    fn schema_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
    }

    fn instrument_schema_root() -> PathBuf {
        schema_root().join("instruments").join("1")
    }

    fn common_schema_root() -> PathBuf {
        schema_root().join("common").join("1")
    }

    fn cashflow_schema_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../cashflows/schemas/cashflow/1")
    }

    fn common_schema_uri(filename: &str) -> String {
        format!("{COMMON_SCHEMA_HOST}{filename}")
    }

    fn cashflow_schema_uri(filename: &str) -> String {
        format!("{CASHFLOW_SCHEMA_HOST}{filename}")
    }

    fn common_schema_resources() -> Vec<(String, jsonschema::Resource)> {
        COMMON_SCHEMA_FILES
            .iter()
            .map(|(_, filename)| {
                let schema = read_schema(&common_schema_root().join(filename));
                let resource = jsonschema::Resource::from_contents(schema)
                    .unwrap_or_else(|err| panic!("build common schema resource {filename}: {err}"));
                (common_schema_uri(filename), resource)
            })
            .collect()
    }

    fn cashflow_schema_resources() -> Vec<(String, jsonschema::Resource)> {
        CASHFLOW_SCHEMA_FILES
            .iter()
            .map(|(_, filename)| {
                let schema = read_schema(&cashflow_schema_root().join(filename));
                let resource = jsonschema::Resource::from_contents(schema).unwrap_or_else(|err| {
                    panic!("build cashflow schema resource {filename}: {err}")
                });
                (cashflow_schema_uri(filename), resource)
            })
            .collect()
    }

    fn schema_resource(schema: Value, context: &str) -> (String, jsonschema::Resource) {
        let id = schema
            .get("$id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{context} is missing $id"))
            .to_string();
        let resource = jsonschema::Resource::from_contents(schema)
            .unwrap_or_else(|err| panic!("build schema resource {context}: {err}"));
        (id, resource)
    }

    fn external_schema_resources() -> Vec<(String, jsonschema::Resource)> {
        let mut resources = common_schema_resources();
        resources.extend(cashflow_schema_resources());
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);
        for path in schema_files {
            let context = path.display().to_string();
            resources.push(schema_resource(read_schema(&path), &context));
        }
        resources
    }

    fn generated_standalone_schema_paths() -> Vec<PathBuf> {
        let mut paths: Vec<_> = [
            ("calibration/1", "calibration"),
            ("results/1", "valuation_result"),
            ("market/1", "market_quote"),
        ]
        .into_iter()
        .map(|(subdir, filename)| {
            schema_root()
                .join(subdir)
                .join(format!("{filename}.schema.json"))
        })
        .collect();
        paths.extend(
            CASHFLOW_SCHEMA_FILES
                .iter()
                .map(|(_, filename)| cashflow_schema_root().join(filename)),
        );
        paths
    }

    fn read_schema(path: &Path) -> Value {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        serde_json::from_str(&content)
            .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
    }

    fn resolve_local_ref<'a>(schema: &'a Value, value: &'a Value) -> &'a Value {
        let mut resolved = value;
        while let Some(reference) = resolved.get("$ref").and_then(Value::as_str) {
            let pointer = reference
                .strip_prefix('#')
                .unwrap_or_else(|| panic!("expected a local schema reference, got {reference}"));
            resolved = schema
                .pointer(pointer)
                .unwrap_or_else(|| panic!("unresolved local schema reference {reference}"));
        }
        resolved
    }

    fn instrument_variants(schema: &Value) -> &[Value] {
        let instrument = resolve_local_ref(schema, &schema["properties"]["instrument"]);
        instrument["oneOf"]
            .as_array()
            .map(Vec::as_slice)
            .expect("instrument schema should declare derived oneOf variants")
    }

    fn variant_const<'a>(variant: &'a Value, property: &str) -> Option<&'a str> {
        variant["properties"][property]["const"].as_str()
    }

    fn collect_schema_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| panic!("read_dir entry {}: {err}", dir.display()))
                    .path()
            })
            .collect();
        entries.sort();

        for path in entries {
            if path.is_dir() {
                collect_schema_files(&path, out);
            } else if path.file_name().and_then(|name| name.to_str())
                != Some("instrument.schema.json")
                && path.extension().and_then(|ext| ext.to_str()) == Some("json")
            {
                out.push(path);
            }
        }
    }

    fn contains_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key(key) || map.values().any(|child| contains_key(child, key))
            }
            Value::Array(items) => items.iter().any(|child| contains_key(child, key)),
            _ => false,
        }
    }

    fn collect_refs(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                    out.insert(reference.to_string());
                }
                for child in map.values() {
                    collect_refs(child, out);
                }
            }
            Value::Array(items) => {
                for child in items {
                    collect_refs(child, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn common_schema_files_exist_and_use_canonical_ids() {
        for (_, filename) in COMMON_SCHEMA_FILES {
            let path = common_schema_root().join(filename);
            let schema = read_schema(&path);
            assert_eq!(
                schema.get("$id").and_then(Value::as_str),
                Some(common_schema_uri(filename).as_str()),
                "{} has the wrong $id",
                path.display()
            );
            assert_eq!(
                schema.get("$schema").and_then(Value::as_str),
                Some(JSON_SCHEMA_2020_12),
                "{} has the wrong $schema dialect",
                path.display()
            );
        }
    }

    #[test]
    fn generated_schemas_use_common_refs_for_moved_defs() {
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);
        schema_files.extend(generated_standalone_schema_paths());
        let mut schemas_with_common_refs = 0usize;

        for path in schema_files {
            let schema = read_schema(&path);
            let mut refs = BTreeSet::new();
            collect_refs(&schema, &mut refs);
            let common_refs: Vec<_> = refs
                .iter()
                .filter(|reference| reference.starts_with(COMMON_SCHEMA_HOST))
                .collect();
            if !common_refs.is_empty() {
                schemas_with_common_refs += 1;
            }

            if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
                for (def_name, _) in COMMON_SCHEMA_FILES {
                    assert!(
                        !defs.contains_key(*def_name),
                        "{} retains moved common $defs entry {def_name}",
                        path.display()
                    );
                }
            }
        }

        assert!(
            schemas_with_common_refs > 0,
            "no generated schema references the common schema library"
        );
    }

    #[test]
    fn generated_instrument_schemas_use_cashflow_refs_for_moved_defs() {
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);
        let mut schemas_with_cashflow_refs = 0usize;

        for path in schema_files {
            let schema = read_schema(&path);
            let mut refs = BTreeSet::new();
            collect_refs(&schema, &mut refs);
            if refs
                .iter()
                .any(|reference| reference.starts_with(CASHFLOW_SCHEMA_HOST))
            {
                schemas_with_cashflow_refs += 1;
            }

            if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
                for (def_name, _) in CASHFLOW_SCHEMA_FILES {
                    assert!(
                        !defs.contains_key(*def_name),
                        "{} retains moved cashflow $defs entry {def_name}",
                        path.display()
                    );
                }
            }
        }

        assert!(
            schemas_with_cashflow_refs > 0,
            "no generated instrument schema references standalone cashflow schemas"
        );
    }

    fn is_date_like_property(name: &str) -> bool {
        name == "date"
            || name.ends_with("_date")
            || name == "maturity"
            || name.ends_with("_maturity")
            || name == "expiry"
            || name.ends_with("_expiry")
    }

    fn schema_accepts_string(value: &Value) -> bool {
        match value.get("type") {
            Some(Value::String(schema_type)) => schema_type == "string",
            Some(Value::Array(schema_types)) => schema_types.iter().any(|schema_type| {
                schema_type
                    .as_str()
                    .is_some_and(|schema_type| schema_type == "string")
            }),
            _ => false,
        }
    }

    fn collect_noncanonical_decimal_patterns(value: &Value, path: &str, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(pattern) = map.get("pattern").and_then(Value::as_str) {
                    if pattern != DECIMAL_PATTERN || !schema_accepts_string(value) {
                        out.push(format!("{path} pattern={pattern:?}"));
                    }
                }

                for (key, child) in map {
                    collect_noncanonical_decimal_patterns(child, &format!("{path}/{key}"), out);
                }
            }
            Value::Array(items) => {
                for (idx, child) in items.iter().enumerate() {
                    collect_noncanonical_decimal_patterns(child, &format!("{path}/{idx}"), out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn decimal_string_properties_use_canonical_pattern() {
        let mut schema_files = Vec::new();
        collect_schema_files(&schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            let mut noncanonical = Vec::new();
            collect_noncanonical_decimal_patterns(&schema, "", &mut noncanonical);

            assert!(
                noncanonical.is_empty(),
                "{} has non-canonical decimal string patterns: {}",
                path.display(),
                noncanonical.join(", ")
            );
        }
    }

    fn collect_missing_date_formats(value: &Value, path: &str, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(properties) = map.get("properties").and_then(Value::as_object) {
                    for (property, schema) in properties {
                        let property_path = format!("{path}/properties/{property}");
                        if is_date_like_property(property)
                            && schema_accepts_string(schema)
                            && schema.get("format").and_then(Value::as_str) != Some("date")
                        {
                            out.push(property_path.clone());
                        }
                        collect_missing_date_formats(schema, &property_path, out);
                    }
                }

                for (key, child) in map {
                    if key != "properties" {
                        collect_missing_date_formats(child, &format!("{path}/{key}"), out);
                    }
                }
            }
            Value::Array(items) => {
                for (idx, child) in items.iter().enumerate() {
                    collect_missing_date_formats(child, &format!("{path}/{idx}"), out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn date_like_string_properties_declare_date_format() {
        let mut schema_files = Vec::new();
        collect_schema_files(&schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            let mut missing = Vec::new();
            collect_missing_date_formats(&schema, "", &mut missing);

            assert!(
                missing.is_empty(),
                "{} has date-like string properties without format=date: {}",
                path.display(),
                missing.join(", ")
            );
        }
    }

    #[test]
    fn schemas_use_canonical_id_host() {
        let mut schema_files = Vec::new();
        collect_schema_files(&schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            let Some(id) = schema.get("$id").and_then(Value::as_str) else {
                continue;
            };
            assert!(
                id.starts_with(SCHEMA_ID_HOST),
                "{} has non-canonical $id host: {id}",
                path.display()
            );
        }
    }

    fn json_pointer_unescape(segment: &str) -> String {
        segment.replace("~1", "/").replace("~0", "~")
    }

    fn collect_local_def_refs(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                    if let Some(rest) = reference.strip_prefix("#/$defs/") {
                        if let Some(segment) = rest.split('/').next() {
                            out.insert(json_pointer_unescape(segment));
                        }
                    }
                }
                for child in map.values() {
                    collect_local_def_refs(child, out);
                }
            }
            Value::Array(items) => {
                for child in items {
                    collect_local_def_refs(child, out);
                }
            }
            _ => {}
        }
    }

    fn reachable_local_defs(schema: &Value) -> BTreeSet<String> {
        let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
            return BTreeSet::new();
        };

        let mut root = schema.clone();
        if let Some(root_obj) = root.as_object_mut() {
            root_obj.remove("$defs");
        }

        let mut discovered = BTreeSet::new();
        collect_local_def_refs(&root, &mut discovered);

        let mut reachable = BTreeSet::new();
        while let Some(next) = discovered.iter().next().cloned() {
            discovered.remove(&next);
            if !reachable.insert(next.clone()) {
                continue;
            }
            if let Some(definition) = defs.get(&next) {
                collect_local_def_refs(definition, &mut discovered);
            }
        }

        reachable
    }

    #[test]
    fn generated_schemas_do_not_emit_unreachable_defs() {
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);
        schema_files.extend(generated_standalone_schema_paths());

        for path in schema_files {
            let schema = read_schema(&path);
            let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
                continue;
            };
            let reachable = reachable_local_defs(&schema);
            let all_defs: BTreeSet<_> = defs.keys().cloned().collect();
            let unreachable: Vec<_> = all_defs.difference(&reachable).cloned().collect();

            assert!(
                unreachable.is_empty(),
                "{} has unreachable $defs: {}",
                path.display(),
                unreachable.join(", ")
            );
        }
    }

    #[test]
    fn generated_schemas_declare_2020_12_when_using_modern_keywords() {
        let mut schema_files = Vec::new();
        collect_schema_files(&schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            if contains_key(&schema, "$defs") || contains_key(&schema, "prefixItems") {
                assert_eq!(
                    schema.get("$schema").and_then(Value::as_str),
                    Some(JSON_SCHEMA_2020_12),
                    "{} uses modern JSON Schema keywords but declares the wrong dialect",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn generated_instrument_schemas_are_typed() {
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            let marker = resolve_local_ref(&schema, &schema["properties"]["schema"]);
            assert!(
                marker["oneOf"].as_array().is_some_and(|variants| variants
                    .iter()
                    .any(|variant| { variant["const"] == "finstack_quant.instrument/1" })),
                "{} is missing the standard schema const",
                path.display()
            );
            assert!(
                schema.pointer("/properties/schema_version").is_none(),
                "{} uses schema_version on a public instrument envelope",
                path.display()
            );
            for variant in instrument_variants(&schema) {
                assert!(
                    variant_const(variant, "type").is_some(),
                    "{} is missing an instrument type discriminator",
                    path.display()
                );
                let spec = resolve_local_ref(&schema, &variant["properties"]["spec"]);
                assert!(
                    spec.get("properties").is_some(),
                    "{} is missing typed spec properties",
                    path.display()
                );
                assert!(
                    spec.get("required").is_some(),
                    "{} is missing typed spec required fields",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn generated_schedule_params_use_canonical_field_names() {
        let path = cashflow_schema_root().join("schedule_params.schema.json");
        let schema = read_schema(&path);
        assert!(
            schema.pointer("/properties/frequency").is_some(),
            "schedule params should expose frequency"
        );
        assert!(
            schema.pointer("/properties/day_count").is_some(),
            "schedule params should expose day_count"
        );
        assert!(
            schema.pointer("/properties/freq").is_none(),
            "schedule params should not expose stale freq"
        );
        assert!(
            schema.pointer("/properties/dc").is_none(),
            "schedule params should not expose stale dc"
        );
        assert!(
            schema.pointer("/properties/bdc").is_none(),
            "schedule params should not expose stale bdc"
        );
    }

    #[test]
    fn schema_version_is_only_used_for_internal_payload_schemas() {
        let mut schema_files = Vec::new();
        collect_schema_files(&schema_root(), &mut schema_files);

        let mut public_schema_version_paths = Vec::new();
        for path in schema_files {
            let schema = read_schema(&path);
            if schema.pointer("/properties/schema_version").is_some()
                && !path.ends_with("results/1/valuation_result.schema.json")
            {
                public_schema_version_paths.push(path.display().to_string());
            }
        }

        assert!(
            public_schema_version_paths.is_empty(),
            "unexpected public schema_version fields: {}",
            public_schema_version_paths.join(", ")
        );
    }

    #[test]
    fn generated_instrument_union_refs_all_typed_schemas() {
        let schema = read_schema(&instrument_schema_root().join("instrument.schema.json"));
        let instrument = resolve_local_ref(&schema, &schema["properties"]["instrument"]);
        assert!(
            instrument.pointer("/properties/type/enum").is_none(),
            "instrument union should not keep the legacy shallow type enum"
        );
        let actual = instrument_variants(&schema)
            .iter()
            .filter_map(|variant| variant_const(variant, "type"))
            .collect::<BTreeSet<_>>();
        let expected = finstack_quant_valuations::instruments::json_loader::registry_tags()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual, expected,
            "instrument union should contain every registered instrument discriminator"
        );
    }

    #[test]
    fn instrument_union_rejects_invalid_typed_spec_directly() {
        let schema = read_schema(&instrument_schema_root().join("instrument.schema.json"));
        let validator = jsonschema::options()
            .with_resources(external_schema_resources().into_iter())
            .build(&schema)
            .expect("compile instrument union schema");
        let invalid = serde_json::json!({
            "schema": "finstack_quant.instrument/1",
            "instrument": {
                "type": "bond",
                "spec": {}
            }
        });

        assert!(
            validator.validate(&invalid).is_err(),
            "instrument union should reject invalid specs for a known discriminator"
        );
    }

    #[test]
    fn generated_instrument_schema_examples_validate() {
        let mut schema_files = Vec::new();
        collect_schema_files(&instrument_schema_root(), &mut schema_files);

        for path in schema_files {
            let schema = read_schema(&path);
            let validator = jsonschema::options()
                .with_resources(external_schema_resources().into_iter())
                .build(&schema)
                .unwrap_or_else(|err| panic!("compile {}: {err}", path.display()));
            let Some(examples) = schema.get("examples").and_then(Value::as_array) else {
                continue;
            };

            for example in examples {
                if let Err(error) = validator.validate(example) {
                    panic!("example in {} failed validation: {error}", path.display());
                }
            }
        }
    }
}

mod instrument_schema_drift {
    use serde_json::Value;
    use std::path::Path;

    fn schema_path(category: &str, name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("schemas")
            .join("instruments")
            .join("1")
            .join(category)
            .join(format!("{name}.schema.json"))
    }

    fn checked_in_schema(category: &str, name: &str) -> Value {
        let path = schema_path(category, name);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        serde_json::from_str(&content)
            .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
    }

    fn assert_f64(schema: &Value, path: &str, expected: f64, context: &str) {
        assert_eq!(
            schema.pointer(path).and_then(Value::as_f64),
            Some(expected),
            "{context}: expected {path} to equal {expected}"
        );
    }

    fn assert_margin_metadata(schema: &Value, context: &str, has_csa_fields: bool) {
        let asset_class = schema
            .pointer("/$defs/CollateralAssetClass")
            .unwrap_or_else(|| panic!("{context}: missing CollateralAssetClass"));
        let asset_classes = asset_class["oneOf"]
            .as_array()
            .expect("derived collateral variants")
            .iter()
            .filter_map(|variant| variant["const"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            asset_classes,
            [
                "cash",
                "government_bonds",
                "agency_bonds",
                "covered_bonds",
                "corporate_bonds",
                "equity",
                "gold",
                "mutual_funds",
            ],
            "{context}: CollateralAssetClass variants changed"
        );

        for field in ["concentration_limit", "fx_haircut_addon", "haircut"] {
            let base = format!("/$defs/CollateralEligibility/properties/{field}");
            assert_f64(schema, &format!("{base}/minimum"), 0.0, context);
            assert_f64(schema, &format!("{base}/maximum"), 1.0, context);
        }
        assert_f64(
            schema,
            "/$defs/EligibleCollateralSchedule/properties/default_haircut/minimum",
            0.0,
            context,
        );
        assert_f64(
            schema,
            "/$defs/EligibleCollateralSchedule/properties/default_haircut/maximum",
            1.0,
            context,
        );
        for field in ["min_remaining_years", "max_remaining_years"] {
            assert_f64(
                schema,
                &format!("/$defs/MaturityConstraints/properties/{field}/minimum"),
                0.0,
                context,
            );
        }

        if has_csa_fields {
            assert_eq!(
                schema
                    .pointer("/$defs/ImParameters/properties/mpor_days/minimum")
                    .and_then(Value::as_u64),
                Some(1),
                "{context}: IM MPOR must be at least one day"
            );
            assert_eq!(
                schema
                    .pointer(
                        "/$defs/MarginCallTiming/properties/notification_deadline_hours/maximum",
                    )
                    .and_then(Value::as_u64),
                Some(23),
                "{context}: notification deadline must be a valid hour"
            );
        }
    }

    #[test]
    fn intended_margin_metadata_is_frozen_in_affected_instrument_schemas() {
        for (category, name, has_csa_fields) in [
            ("credit_derivatives", "cds_index", true),
            ("credit_derivatives", "credit_default_swap", true),
            ("equity", "levered_real_estate_equity", true),
            ("equity", "trs_equity", true),
            ("fixed_income", "structured_credit", true),
            ("fixed_income", "trs_fixed_income_index", true),
            ("rates", "interest_rate_swap", true),
            ("rates", "repo", false),
        ] {
            let schema = checked_in_schema(category, name);
            assert_margin_metadata(&schema, &format!("{category}/{name}"), has_csa_fields);
        }
    }
}
