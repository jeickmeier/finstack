//! Hand-derived numeric FRTB SBA charges for the risk classes that previously
//! had no numeric coverage at all: CSR securitisation (CTP), CSR
//! securitisation (non-CTP), and commodity.
//!
//! Every test drives [`FrtbSbaEngine`] end-to-end (`builder()` ->
//! `calculate()`) and asserts the resulting charge against a value derived by
//! hand. Each derivation is written out in the comment above the assertion
//! with every risk weight and correlation quoted as a literal, so a change to
//! any prescribed parameter in `regulatory::frtb::params` fails the test
//! rather than silently moving capital.
//!
//! # Standards reference
//!
//! - Basel Committee on Banking Supervision, *Minimum capital requirements for
//!   market risk* (BCBS **d457**), published 14 January 2019, corrected version
//!   25 February 2019; consolidated as Basel Framework chapter **MAR21**,
//!   "Standardised approach: sensitivities-based method", version effective
//!   1 January 2023.
//! - Aggregation: MAR21.4 (intra-bucket `K_b`), MAR21.5 (curvature `psi` and
//!   the curvature aggregation), MAR21.6 (inter-bucket, the alternative
//!   capped-`S_b` fallback, and the three correlation scenarios), MAR21.7
//!   (maximum across scenarios).
//! - Correlation scenarios (MAR21.6): `rho_high = min(1.25 * rho, 1)`,
//!   `rho_low = max(2 * rho - 1, 0.75 * rho)`.
//!
//! # Basel-derived vs behaviour-pinning tests
//!
//! Tests whose name ends in `_matches_mar21_derivation` use only parameters
//! that were verified against MAR21 as published. Tests whose name ends in
//! `_pins_current_implementation` embed at least one parameter that is
//! **known to deviate** from MAR21; the deviation is named in the test's
//! comment and documented in full in the "Known deviations from MAR21"
//! section of the corresponding `regulatory::frtb::params` module. Those tests
//! exist to make the current capital numbers explicit and reviewable — they
//! are not a claim that the numbers are regulatory-correct, and they are
//! expected to be updated (deliberately, with sign-off) when the deviations
//! are closed.
//!
//! # Scale convention
//!
//! Delta risk weights are quoted **in percent** exactly as published (`30.0`
//! means 30%), and the engine expects sensitivities scaled to match — see the
//! table in the `regulatory::frtb` module documentation. The literals below
//! therefore multiply directly.
//!
//! # Curvature inputs
//!
//! `CVR+` / `CVR-` are supplied by the caller already shocked and already net
//! of the delta-hedged component, in the loss-positive convention. The engine
//! applies **no** further risk weight to curvature inputs; the tests below pin
//! that behaviour.

use finstack_quant_core::currency::Currency;
use finstack_quant_margin::regulatory::frtb::{
    CorrelationScenario, FrtbRiskClass, FrtbSbaEngine, FrtbSbaResult, FrtbSensitivities,
};

/// Relative tolerance for comparing a computed charge with the hand-derived
/// value. The arithmetic is a handful of `f64` multiplications and one square
/// root, so agreement is far tighter than this; the tolerance exists only to
/// avoid pinning the last bit of an IEEE-754 result.
const REL_TOL: f64 = 1e-9;

/// Assert `actual` matches `expected` to [`REL_TOL`] relative precision.
fn assert_charge(actual: f64, expected: f64, what: &str) {
    let tol = expected.abs() * REL_TOL;
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: expected {expected}, got {actual} (diff {})",
        actual - expected
    );
}

/// Build an engine restricted to one risk class under the Medium (prescribed)
/// correlation scenario, so the hand derivation has no scenario ambiguity.
fn medium_engine(risk_class: FrtbRiskClass) -> FrtbSbaEngine {
    FrtbSbaEngine::builder()
        .risk_classes(vec![risk_class])
        .scenarios(vec![CorrelationScenario::Medium])
        .build()
        .expect("engine builds for a single risk class and scenario")
}

fn delta_charge_of(result: &FrtbSbaResult, rc: FrtbRiskClass) -> f64 {
    result.delta_by_risk_class.get(&rc).copied().unwrap_or(0.0)
}

fn vega_charge_of(result: &FrtbSbaResult, rc: FrtbRiskClass) -> f64 {
    result.vega_by_risk_class.get(&rc).copied().unwrap_or(0.0)
}

fn curvature_charge_of(result: &FrtbSbaResult, rc: FrtbRiskClass) -> f64 {
    result
        .curvature_by_risk_class
        .get(&rc)
        .copied()
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Commodity
// ---------------------------------------------------------------------------

#[test]
fn commodity_delta_matches_mar21_derivation() {
    let engine = medium_engine(FrtbRiskClass::Commodity);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    // Bucket 1 = solid combustibles, RW = 30% (MAR21.82, Table 11).
    sens.add_commodity_delta("COAL_A", 1, "1Y", 100_000.0);
    sens.add_commodity_delta("COAL_B", 1, "1Y", 50_000.0);
    // Bucket 3 = electricity and carbon trading, RW = 60% (MAR21.82, Table 11).
    sens.add_commodity_delta("POWER", 3, "1Y", 50_000.0);

    let result = engine.calculate(&sens).expect("commodity delta calculates");

    // Derivation (MAR21.4 / MAR21.6). Every parameter used here matches
    // MAR21 as published:
    //
    //   Weighted sensitivities (WS_k = s_k * RW_b), MAR21.82 Table 11:
    //     WS(COAL_A) = 100_000 * 30.0 = 3_000_000
    //     WS(COAL_B) =  50_000 * 30.0 = 1_500_000
    //     WS(POWER)  =  50_000 * 60.0 = 3_000_000
    //
    //   Intra-bucket (MAR21.83): rho_kl = rho_cty * rho_tenor * rho_basis.
    //   The two bucket-1 factors are different commodities within bucket 1
    //   (rho_cty = 55%, MAR21.83 Table 12 bucket 1), share a tenor
    //   (rho_tenor = 1) and a delivery location (rho_basis = 1), so
    //   rho = 55%:
    //     K_1^2 = 3.0e6^2 + 1.5e6^2 + 2 * 0.55 * 3.0e6 * 1.5e6
    //           = 9.00e12 + 2.25e12 + 4.95e12 = 1.62e13
    //     K_1   = sqrt(1.62e13) = 4_024_922.3594996217
    //     S_1   = 3.0e6 + 1.5e6 = 4_500_000
    //   Bucket 3 holds a single factor: K_3 = S_3 = 3_000_000.
    //
    //   Inter-bucket (MAR21.85(1)): gamma = 20% because buckets 1 and 3 are
    //   both within buckets 1-10:
    //     Delta^2 = K_1^2 + K_3^2 + 2 * 0.20 * S_1 * S_3
    //             = 1.62e13 + 9.0e12 + 0.4 * 4.5e6 * 3.0e6
    //             = 2.52e13 + 5.4e12 = 3.06e13
    //     Delta   = sqrt(3.06e13) = 5_531_726.674375732
    //
    // Note: the implementation applies a flat 55% intra-bucket correlation to
    // every commodity bucket rather than the per-bucket MAR21.83 Table 12
    // vector. Bucket 1's published value *is* 55%, and bucket 3 contributes a
    // single factor, so this portfolio is unaffected. See the deviation note
    // in `params/commodity.rs`.
    let expected = 5_531_726.674_375_732;
    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::Commodity),
        expected,
        "commodity delta",
    );
    // No vega/curvature/DRC/RRAO in this portfolio, so the total is the delta
    // charge under the single configured scenario.
    assert_charge(result.total, expected, "commodity delta total");
}

#[test]
fn commodity_vega_matches_mar21_derivation() {
    let engine = medium_engine(FrtbRiskClass::Commodity);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    // Deliberately non-unit sensitivities: the vega risk weight genuinely
    // multiplies here, so the assertion moves if COMMODITY_VEGA_RISK_WEIGHT
    // changes. (The pre-existing unit-input tests in `vega.rs` fed 100.0 and
    // asserted 100.0, which would have passed with no weight applied at all.)
    sens.commodity_vega
        .insert(("COAL_A".to_string(), 1, "1Y".to_string()), 200_000.0);
    sens.commodity_vega
        .insert(("COAL_B".to_string(), 1, "1Y".to_string()), 100_000.0);
    // Bucket 5 = non-precious metals.
    sens.commodity_vega
        .insert(("COPPER".to_string(), 5, "1Y".to_string()), 100_000.0);

    let result = engine.calculate(&sens).expect("commodity vega calculates");

    // Derivation (MAR21.4 / MAR21.6 with the vega risk weight):
    //
    //   Commodity vega risk weight = 100%. MAR21.92 footnote 24 sets
    //   RW_k = min(RW_sigma * sqrt(LH_risk class) / sqrt(10), 100%) with
    //   RW_sigma = 55%; MAR21.92 Table 13 gives the commodity liquidity
    //   horizon LH = 120 days, so 0.55 * sqrt(120/10) = 0.55 * sqrt(12)
    //   = 1.9053, which binds at the 100% cap. Table 13 publishes the
    //   resulting 100% directly.
    //
    //     WS(COAL_A) = 200_000 * 1.00 = 200_000
    //     WS(COAL_B) = 100_000 * 1.00 = 100_000
    //     WS(COPPER) = 100_000 * 1.00 = 100_000
    //
    //   Intra-bucket rho = 55% (MAR21.83 Table 12 bucket 1; same tenor and
    //   location so the other two factors are 1):
    //     K_1^2 = 200_000^2 + 100_000^2 + 2 * 0.55 * 200_000 * 100_000
    //           = 4.0e10 + 1.0e10 + 2.2e10 = 7.2e10
    //     K_1   = sqrt(7.2e10) = 268_328.15729997476, S_1 = 300_000
    //     K_5   = 100_000, S_5 = 100_000
    //
    //   Inter-bucket gamma = 20% (MAR21.85(1)):
    //     Vega^2 = 7.2e10 + 1.0e10 + 2 * 0.20 * 300_000 * 100_000
    //            = 8.2e10 + 1.2e10 = 9.4e10
    //     Vega   = sqrt(9.4e10) = 306_594.1943351178
    let expected = 306_594.194_335_117_8;
    let charge = vega_charge_of(&result, FrtbRiskClass::Commodity);
    assert_charge(charge, expected, "commodity vega");
    assert_charge(result.total, expected, "commodity vega total");

    // Guard against the failure mode called out in the FRTB coverage audit:
    // a 100% weight makes a unit-input test indistinguishable from applying
    // no weight and no aggregation at all.
    let raw_sum: f64 = sens.commodity_vega.values().sum();
    assert!(
        (charge - raw_sum).abs() > 1.0,
        "commodity vega charge must be an aggregation, not a plain sum of sensitivities"
    );
}

#[test]
fn commodity_curvature_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::Commodity);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    // (CVR+, CVR-), loss-positive, supplied already shocked.
    sens.commodity_curvature
        .insert(("COAL_A".to_string(), 1), (500_000.0, -100_000.0));
    sens.commodity_curvature
        .insert(("COAL_B".to_string(), 1), (200_000.0, -40_000.0));
    sens.commodity_curvature
        .insert(("COPPER".to_string(), 5), (300_000.0, -50_000.0));

    let result = engine
        .calculate(&sens)
        .expect("commodity curvature calculates");

    // DEVIATION: MAR21.100 states that curvature rho_kl and gamma_bc are the
    // **squares** of the corresponding delta parameters. The implementation
    // squares the inter-bucket gamma but passes the intra-bucket rho through
    // unsquared (0.55 instead of 0.55^2 = 0.3025). This test pins the current
    // behaviour; see the deviation note in `params/commodity.rs`.
    //
    // Derivation of the current behaviour (MAR21.5 aggregation shape):
    //
    //   Bucket 1, up side, rho = 55% (unsquared, see deviation above):
    //     K_1+^2 = max(500k,0)^2 + max(200k,0)^2
    //              + 2 * 0.55 * 500k * 200k * psi(500k, 200k)
    //            = 2.50e11 + 4.0e10 + 1.10e11 = 4.00e11
    //     K_1+   = sqrt(4.00e11) = 632_455.5320336758, S_1+ = 700_000
    //   Bucket 1, down side: both CVR- are negative, so every max(CVR,0)^2
    //   term is zero and psi(-,-) = 0 kills the off-diagonal, giving
    //   K_1- = 0. The up side therefore binds, and MAR21.5 caps the bucket
    //   sum: S_1 = clamp(700_000, -K_1, K_1) = 632_455.5320336758.
    //
    //   Bucket 5 holds a single factor: K_5 = S_5 = 300_000.
    //
    //   Inter-bucket, gamma = 20% squared per MAR21.100:
    //     Curv^2 = K_1^2 + K_5^2 + 2 * 0.20^2 * S_1 * S_5 * psi(S_1, S_5)
    //            = 4.00e11 + 9.0e10 + 0.08 * 632_455.5320336758 * 300_000
    //            = 4.90e11 + 1.5178932768880619e10
    //            = 5.051789327688806e11
    //     Curv   = 710_759.4056843766
    let expected = 710_759.405_684_376_6;
    assert_charge(
        curvature_charge_of(&result, FrtbRiskClass::Commodity),
        expected,
        "commodity curvature",
    );
    assert_charge(result.total, expected, "commodity curvature total");
}

// ---------------------------------------------------------------------------
// CSR securitisation - correlation trading portfolio (CTP)
// ---------------------------------------------------------------------------

#[test]
fn csr_sec_ctp_delta_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    // Bucket 1, RW = 4% and bucket 3, RW = 8% — both match MAR21.59 Table 6
    // as published. Two distinct tranche names share bucket 1 and a tenor, so
    // the intra-bucket name correlation applies.
    sens.csr_sec_ctp_delta
        .insert(("CTP_A".to_string(), 1, "5Y".to_string()), 100_000.0);
    sens.csr_sec_ctp_delta
        .insert(("CTP_B".to_string(), 1, "5Y".to_string()), 50_000.0);
    sens.csr_sec_ctp_delta
        .insert(("CTP_C".to_string(), 3, "5Y".to_string()), 25_000.0);

    let result = engine
        .calculate(&sens)
        .expect("CSR sec CTP delta calculates");

    // DEVIATION: MAR21.60 derives the CTP intra-bucket correlation exactly as
    // MAR21.54 does for CSR non-securitisation — rho_name (35% for different
    // names in buckets 1-15) * rho_tenor (65%) * rho_basis (99.00% for CTP) —
    // and MAR21.61 makes the CTP inter-bucket gamma identical to MAR21.57
    // (gamma_rating * gamma_sector, a matrix). The implementation instead uses
    // a flat 30% intra-bucket and a flat 40% inter-bucket correlation. This
    // test pins the current behaviour; see `params/csr.rs`.
    //
    // Derivation of the current behaviour (MAR21.4 / MAR21.6 shape):
    //
    //     WS(CTP_A) = 100_000 * 4.0 = 400_000
    //     WS(CTP_B) =  50_000 * 4.0 = 200_000
    //     WS(CTP_C) =  25_000 * 8.0 = 200_000
    //
    //   Intra-bucket, rho = rho_name * rho_tenor = 0.30 * 1.0 = 0.30 (the two
    //   bucket-1 factors share the 5Y tenor):
    //     K_1^2 = 400_000^2 + 200_000^2 + 2 * 0.30 * 400_000 * 200_000
    //           = 1.60e11 + 4.0e10 + 4.8e10 = 2.48e11
    //     K_1   = sqrt(2.48e11) = 497_995.98391954927, S_1 = 600_000
    //     K_3   = 200_000, S_3 = 200_000
    //
    //   Inter-bucket, gamma = 0.40:
    //     Delta^2 = 2.48e11 + 4.0e10 + 2 * 0.40 * 600_000 * 200_000
    //             = 2.88e11 + 9.6e10 = 3.84e11
    //     Delta   = sqrt(3.84e11) = 619_677.3353931867
    let expected = 619_677.335_393_186_7;
    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::CsrSecCtp),
        expected,
        "CSR sec CTP delta",
    );
    assert_charge(result.total, expected, "CSR sec CTP delta total");
}

#[test]
fn csr_sec_ctp_vega_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.csr_sec_ctp_vega
        .insert(("CTP_A".to_string(), 1, "1Y".to_string()), 300_000.0);
    sens.csr_sec_ctp_vega
        .insert(("CTP_B".to_string(), 1, "1Y".to_string()), 100_000.0);
    sens.csr_sec_ctp_vega
        .insert(("CTP_C".to_string(), 5, "1Y".to_string()), 200_000.0);

    let result = engine
        .calculate(&sens)
        .expect("CSR sec CTP vega calculates");

    // The vega risk weight used here is Basel-correct: MAR21.92 Table 13
    // gives CSR securitisation (CTP) a 120-day liquidity horizon, so
    // RW = min(0.55 * sqrt(12), 100%) = 100%. The intra/inter correlations
    // are the flat 30%/40% deviation described in
    // `csr_sec_ctp_delta_pins_current_implementation`.
    //
    //     WS(CTP_A) = 300_000, WS(CTP_B) = 100_000, WS(CTP_C) = 200_000
    //
    //   Intra-bucket rho = 30%:
    //     K_1^2 = 300_000^2 + 100_000^2 + 2 * 0.30 * 300_000 * 100_000
    //           = 9.0e10 + 1.0e10 + 1.8e10 = 1.18e11
    //     K_1   = sqrt(1.18e11) = 343_511.2807463534, S_1 = 400_000
    //     K_5   = 200_000, S_5 = 200_000
    //
    //   Inter-bucket gamma = 40%:
    //     Vega^2 = 1.18e11 + 4.0e10 + 2 * 0.40 * 400_000 * 200_000
    //            = 1.58e11 + 6.4e10 = 2.22e11
    //     Vega   = sqrt(2.22e11) = 471_168.75957558985
    let expected = 471_168.759_575_589_85;
    let charge = vega_charge_of(&result, FrtbRiskClass::CsrSecCtp);
    assert_charge(charge, expected, "CSR sec CTP vega");
    assert_charge(result.total, expected, "CSR sec CTP vega total");

    let raw_sum: f64 = sens.csr_sec_ctp_vega.values().sum();
    assert!(
        (charge - raw_sum).abs() > 1.0,
        "CSR sec CTP vega charge must be an aggregation, not a plain sum"
    );
}

#[test]
fn csr_sec_ctp_curvature_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.csr_sec_ctp_curvature
        .insert(("CTP_A".to_string(), 1), (400_000.0, -50_000.0));
    sens.csr_sec_ctp_curvature
        .insert(("CTP_B".to_string(), 1), (200_000.0, -20_000.0));
    sens.csr_sec_ctp_curvature
        .insert(("CTP_C".to_string(), 3), (300_000.0, -30_000.0));

    let result = engine
        .calculate(&sens)
        .expect("CSR sec CTP curvature calculates");

    // Pins the flat 30%/40% correlation deviation *and* the MAR21.100
    // unsquared-intra-rho deviation described above.
    //
    //   Bucket 1 up side (rho = 30%):
    //     K_1+^2 = 400k^2 + 200k^2 + 2 * 0.30 * 400k * 200k
    //            = 1.60e11 + 4.0e10 + 4.8e10 = 2.48e11
    //     K_1+   = 497_995.98391954927, S_1+ = 600_000
    //   Bucket 1 down side: both CVR- negative -> K_1- = 0, so the up side
    //   binds and S_1 is capped at K_1 = 497_995.98391954927.
    //   Bucket 3: K_3 = S_3 = 300_000.
    //
    //   Inter-bucket (gamma squared, MAR21.100):
    //     Curv^2 = 2.48e11 + 9.0e10
    //              + 2 * 0.40^2 * 497_995.98391954927 * 300_000
    //            = 3.38e11 + 4.780761285627673e10 = 3.858076128562767e11
    //     Curv   = 621_134.1356392165
    let expected = 621_134.135_639_216_5;
    assert_charge(
        curvature_charge_of(&result, FrtbRiskClass::CsrSecCtp),
        expected,
        "CSR sec CTP curvature",
    );
    assert_charge(result.total, expected, "CSR sec CTP curvature total");
}

// ---------------------------------------------------------------------------
// CSR securitisation - non-CTP
// ---------------------------------------------------------------------------

#[test]
fn csr_sec_nonctp_delta_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    // Buckets 1 (RW = 0.9%) and 5 (RW = 0.8%) are deliberately chosen because
    // both match MAR21.64 Table 8 as published. Several other non-CTP buckets
    // in the implementation's table do not — see `params/csr.rs`.
    sens.csr_sec_nonctp_delta
        .insert(("RMBS_A".to_string(), 1, "5Y".to_string()), 1_000_000.0);
    sens.csr_sec_nonctp_delta
        .insert(("RMBS_B".to_string(), 1, "5Y".to_string()), 500_000.0);
    sens.csr_sec_nonctp_delta
        .insert(("CMBS_C".to_string(), 5, "5Y".to_string()), 1_000_000.0);

    let result = engine
        .calculate(&sens)
        .expect("CSR sec non-CTP delta calculates");

    // DEVIATION: MAR21.68 gives the non-CTP intra-bucket correlation as
    // rho_tranche (40% for different tranches) * rho_tenor (80%) *
    // rho_basis (99.90%), and MAR21.70 sets the inter-bucket gamma to **0%**
    // across buckets 1-24. The implementation uses a flat 30% intra-bucket
    // and 20% inter-bucket. This test pins the current behaviour; see
    // `params/csr.rs`.
    //
    // Derivation of the current behaviour:
    //
    //     WS(RMBS_A) = 1_000_000 * 0.9 =   900_000
    //     WS(RMBS_B) =   500_000 * 0.9 =   450_000
    //     WS(CMBS_C) = 1_000_000 * 0.8 =   800_000
    //
    //   Intra-bucket rho = 30% (same tenor -> rho_tenor factor is 1):
    //     K_1^2 = 900_000^2 + 450_000^2 + 2 * 0.30 * 900_000 * 450_000
    //           = 8.10e11 + 2.025e11 + 2.43e11 = 1.2555e12
    //     K_1   = sqrt(1.2555e12) = 1_120_490.963818986, S_1 = 1_350_000
    //     K_5   = 800_000, S_5 = 800_000
    //
    //   Inter-bucket gamma = 20%:
    //     Delta^2 = 1.2555e12 + 6.4e11 + 2 * 0.20 * 1_350_000 * 800_000
    //             = 1.8955e12 + 4.32e11 = 2.3275e12
    //     Delta   = sqrt(2.3275e12) = 1_525_614.6302392357
    let expected = 1_525_614.630_239_235_7;
    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
        expected,
        "CSR sec non-CTP delta",
    );
    assert_charge(result.total, expected, "CSR sec non-CTP delta total");
}

#[test]
fn csr_sec_nonctp_vega_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.csr_sec_nonctp_vega
        .insert(("RMBS_A".to_string(), 1, "1Y".to_string()), 300_000.0);
    sens.csr_sec_nonctp_vega
        .insert(("RMBS_B".to_string(), 1, "1Y".to_string()), 100_000.0);
    sens.csr_sec_nonctp_vega
        .insert(("CMBS_C".to_string(), 5, "1Y".to_string()), 200_000.0);

    let result = engine
        .calculate(&sens)
        .expect("CSR sec non-CTP vega calculates");

    // The vega risk weight is Basel-correct: MAR21.92 Table 13 gives CSR
    // securitisation (non-CTP) a 120-day liquidity horizon, so
    // RW = min(0.55 * sqrt(12), 100%) = 100%. The correlations are the flat
    // 30%/20% deviation described in
    // `csr_sec_nonctp_delta_pins_current_implementation`.
    //
    //   Intra-bucket rho = 30%:
    //     K_1^2 = 300_000^2 + 100_000^2 + 2 * 0.30 * 300_000 * 100_000
    //           = 1.18e11
    //     K_1   = 343_511.2807463534, S_1 = 400_000
    //     K_5   = 200_000, S_5 = 200_000
    //
    //   Inter-bucket gamma = 20%:
    //     Vega^2 = 1.18e11 + 4.0e10 + 2 * 0.20 * 400_000 * 200_000
    //            = 1.58e11 + 3.2e10 = 1.90e11
    //     Vega   = sqrt(1.90e11) = 435_889.89435406734
    let expected = 435_889.894_354_067_34;
    let charge = vega_charge_of(&result, FrtbRiskClass::CsrSecNonCtp);
    assert_charge(charge, expected, "CSR sec non-CTP vega");
    assert_charge(result.total, expected, "CSR sec non-CTP vega total");

    let raw_sum: f64 = sens.csr_sec_nonctp_vega.values().sum();
    assert!(
        (charge - raw_sum).abs() > 1.0,
        "CSR sec non-CTP vega charge must be an aggregation, not a plain sum"
    );
}

#[test]
fn csr_sec_nonctp_curvature_pins_current_implementation() {
    let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.csr_sec_nonctp_curvature
        .insert(("RMBS_A".to_string(), 1), (400_000.0, -50_000.0));
    sens.csr_sec_nonctp_curvature
        .insert(("RMBS_B".to_string(), 1), (200_000.0, -20_000.0));
    sens.csr_sec_nonctp_curvature
        .insert(("CMBS_C".to_string(), 5), (300_000.0, -30_000.0));

    let result = engine
        .calculate(&sens)
        .expect("CSR sec non-CTP curvature calculates");

    // Pins the flat 30%/20% correlation deviation and the MAR21.100
    // unsquared-intra-rho deviation.
    //
    //   Bucket 1: K_1 = sqrt(2.48e11) = 497_995.98391954927 (the up side
    //   binds; the down side is entirely negative so psi zeroes it), and S_1
    //   is capped at K_1. Bucket 5: K_5 = S_5 = 300_000.
    //
    //     Curv^2 = 2.48e11 + 9.0e10
    //              + 2 * 0.20^2 * 497_995.98391954927 * 300_000
    //            = 3.38e11 + 1.1951903214069183e10
    //            = 3.4995190321406915e11
    //     Curv   = 591_567.3280481852
    let expected = 591_567.328_048_185_2;
    assert_charge(
        curvature_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
        expected,
        "CSR sec non-CTP curvature",
    );
    assert_charge(result.total, expected, "CSR sec non-CTP curvature total");
}

// ---------------------------------------------------------------------------
// Full engine path: all three correlation scenarios, and cross-class summation
// ---------------------------------------------------------------------------

#[test]
fn commodity_delta_scenario_maximum_binds_at_high_correlation() {
    // Default engine: all three correlation scenarios, maximum taken
    // (MAR21.7).
    let engine = FrtbSbaEngine::builder()
        .risk_classes(vec![FrtbRiskClass::Commodity])
        .build()
        .expect("default-scenario engine builds");

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.add_commodity_delta("COAL_A", 1, "1Y", 100_000.0);
    sens.add_commodity_delta("COAL_B", 1, "1Y", 50_000.0);
    sens.add_commodity_delta("POWER", 3, "1Y", 50_000.0);

    let result = engine.calculate(&sens).expect("commodity delta calculates");

    // Same portfolio as `commodity_delta_matches_mar21_derivation`. All
    // sensitivities share a sign, so a higher correlation can only raise the
    // charge and the High scenario must bind.
    //
    //   High (MAR21.6(2)): rho = min(1.25 * 0.55, 1) = 0.6875,
    //                      gamma = min(1.25 * 0.20, 1) = 0.25
    //     K_1^2 = 9.0e12 + 2.25e12 + 2 * 0.6875 * 3.0e6 * 1.5e6 = 1.74375e13
    //     Delta = 5_760_859.3109014565
    //   Medium: 5_531_726.674375732 (derived above)
    //   Low (MAR21.6(3)): rho = max(2*0.55 - 1, 0.75*0.55) = 0.4125,
    //                     gamma = max(2*0.20 - 1, 0.75*0.20) = 0.15
    //     Delta = 5_292_683.629313205
    let high = 5_760_859.310_901_456_5;
    let medium = 5_531_726.674_375_732;
    let low = 5_292_683.629_313_205;

    let charge_for = |scenario: CorrelationScenario| {
        result
            .scenario_charges
            .get(&scenario)
            .copied()
            .unwrap_or(0.0)
    };

    assert_charge(
        charge_for(CorrelationScenario::High),
        high,
        "commodity delta, high scenario",
    );
    assert_charge(
        charge_for(CorrelationScenario::Medium),
        medium,
        "commodity delta, medium scenario",
    );
    assert_charge(
        charge_for(CorrelationScenario::Low),
        low,
        "commodity delta, low scenario",
    );

    assert_eq!(
        result.binding_scenario,
        CorrelationScenario::High,
        "same-sign sensitivities must bind at the high-correlation scenario"
    );
    assert_charge(result.total, high, "commodity delta total (max scenario)");
}

#[test]
fn previously_uncovered_risk_classes_contribute_additively() {
    // MAR21.7: within a scenario the SBA charge is the plain sum of the
    // per-risk-class delta/vega/curvature charges — the SBA has no
    // cross-risk-class correlation matrix. Pin that all three previously
    // uncovered risk classes reach the result breakdown and add up.
    let engine = FrtbSbaEngine::builder()
        .risk_classes(vec![
            FrtbRiskClass::Commodity,
            FrtbRiskClass::CsrSecCtp,
            FrtbRiskClass::CsrSecNonCtp,
        ])
        .scenarios(vec![CorrelationScenario::Medium])
        .build()
        .expect("engine builds");

    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.add_commodity_delta("COAL_A", 1, "1Y", 100_000.0);
    sens.add_commodity_delta("COAL_B", 1, "1Y", 50_000.0);
    sens.add_commodity_delta("POWER", 3, "1Y", 50_000.0);
    sens.csr_sec_ctp_vega
        .insert(("CTP_A".to_string(), 1, "1Y".to_string()), 300_000.0);
    sens.csr_sec_ctp_vega
        .insert(("CTP_B".to_string(), 1, "1Y".to_string()), 100_000.0);
    sens.csr_sec_ctp_vega
        .insert(("CTP_C".to_string(), 5, "1Y".to_string()), 200_000.0);
    sens.csr_sec_nonctp_curvature
        .insert(("RMBS_A".to_string(), 1), (400_000.0, -50_000.0));
    sens.csr_sec_nonctp_curvature
        .insert(("RMBS_B".to_string(), 1), (200_000.0, -20_000.0));
    sens.csr_sec_nonctp_curvature
        .insert(("CMBS_C".to_string(), 5), (300_000.0, -30_000.0));

    let result = engine.calculate(&sens).expect("multi-class calculates");

    // Component values are the ones derived in the single-class tests above.
    let commodity_delta = 5_531_726.674_375_732;
    let ctp_vega = 471_168.759_575_589_85;
    let nonctp_curvature = 591_567.328_048_185_2;

    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::Commodity),
        commodity_delta,
        "commodity delta component",
    );
    assert_charge(
        vega_charge_of(&result, FrtbRiskClass::CsrSecCtp),
        ctp_vega,
        "CSR sec CTP vega component",
    );
    assert_charge(
        curvature_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
        nonctp_curvature,
        "CSR sec non-CTP curvature component",
    );
    assert_charge(
        result.total,
        commodity_delta + ctp_vega + nonctp_curvature,
        "SBA total is the sum of risk-class components",
    );
}

// ---------------------------------------------------------------------------
// Corrected risk-weight tables — engine-level coverage
//
// The tests above deliberately use non-CTP buckets 1 and 5 and non-sec buckets
// that were already correct, so none of them exercised the 22 bucket weights
// corrected on 2026-08-20 against BCBS d457. These do.
//
// Each uses a SINGLE sensitivity in a SINGLE bucket. With one factor,
// K_b = sqrt(WS^2) = |WS| whatever the intra-bucket correlation, and with one
// bucket the inter-bucket aggregation reduces to K_b. The expected value is
// therefore `sensitivity * risk_weight` and is independent of the correlation
// deviations documented in `params/csr.rs` — so these pin the WEIGHTS only.
// ---------------------------------------------------------------------------

/// MAR21.67 sets bucket 25 ("other sector") at 3.5%. It was implemented as
/// 12.5%, a 3.6x overstatement — the single largest weight error in the FRTB
/// parameter set.
#[test]
fn csr_sec_nonctp_bucket_25_uses_the_published_three_and_a_half_percent() {
    let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);
    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.csr_sec_nonctp_delta
        .insert(("OTHER_A".to_string(), 25, "5Y".to_string()), 1_000_000.0);

    let result = engine.calculate(&sens).expect("bucket 25 delta calculates");

    // Weights are in percentage points and multiply directly (see module docs):
    // 1_000_000 * 3.5 = 3_500_000. Under the old 12.5 this was 12_500_000.
    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
        3_500_000.0,
        "CSR sec non-CTP bucket 25 delta (MAR21.67)",
    );
}

/// MAR21.64 Table 8 publishes buckets 7 and 8 at 1.2% and 1.4%; they were
/// implemented as 3.5% and 5.5%.
#[test]
fn csr_sec_nonctp_buckets_7_and_8_use_published_table_8_weights() {
    for (bucket, weight) in [(7u8, 1.2), (8u8, 1.4)] {
        let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);
        let mut sens = FrtbSensitivities::new(Currency::USD);
        sens.csr_sec_nonctp_delta
            .insert(("ABS".to_string(), bucket, "5Y".to_string()), 1_000_000.0);

        let result = engine.calculate(&sens).expect("delta calculates");
        assert_charge(
            delta_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
            1_000_000.0 * weight,
            "CSR sec non-CTP Table 8 weight",
        );
    }
}

/// MAR21.65 derives bucket 13 as 1.25 x bucket 5 (0.8%) = 1.0%; it was 5.0%.
/// MAR21.66 derives bucket 21 as 1.75 x bucket 5 = 1.4%; it was 5.0%.
#[test]
fn csr_sec_nonctp_derived_buckets_scale_from_the_base_row() {
    for (bucket, weight) in [(13u8, 0.8 * 1.25), (21u8, 0.8 * 1.75)] {
        let engine = medium_engine(FrtbRiskClass::CsrSecNonCtp);
        let mut sens = FrtbSensitivities::new(Currency::USD);
        sens.csr_sec_nonctp_delta
            .insert(("ABS".to_string(), bucket, "5Y".to_string()), 1_000_000.0);

        let result = engine.calculate(&sens).expect("delta calculates");
        assert_charge(
            delta_charge_of(&result, FrtbRiskClass::CsrSecNonCtp),
            1_000_000.0 * weight,
            "CSR sec non-CTP derived weight (MAR21.65/21.66)",
        );
    }
}

/// MAR21.53 Table 4 publishes bucket 8 (covered bonds) at 2.5%; it was
/// implemented at 1.0%, understating the charge by 60%.
#[test]
fn csr_nonsec_bucket_8_covered_bonds_uses_the_published_weight() {
    let engine = medium_engine(FrtbRiskClass::CsrNonSec);
    let mut sens = FrtbSensitivities::new(Currency::USD);
    sens.add_csr_nonsec_delta("COVERED_A", 8, "5Y", 1_000_000.0);

    let result = engine.calculate(&sens).expect("bucket 8 delta calculates");

    // 1_000_000 * 2.5 = 2_500_000. Under the old 1.0 this was 1_000_000.
    assert_charge(
        delta_charge_of(&result, FrtbRiskClass::CsrNonSec),
        2_500_000.0,
        "CSR non-sec bucket 8 delta (MAR21.53 Table 4)",
    );
}
