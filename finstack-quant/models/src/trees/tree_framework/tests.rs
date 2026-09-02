//! Shared node, evolution, and backward-induction components for pricing trees.
//!
use super::*;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::HashMap;

fn sample_state_variables() -> HashMap<&'static str, f64> {
    let mut vars = HashMap::default();
    vars.insert(state_keys::SPOT, 100.0);
    vars.insert(state_keys::INTEREST_RATE, 0.03);
    vars.insert(state_keys::HAZARD_RATE, 0.02);
    vars.insert(state_keys::DF, 0.95);
    vars
}

#[test]
fn node_state_caches_common_fields() {
    let market = MarketContext::new();
    let vars = sample_state_variables();

    let state = NodeState::new(2, 0.5, &vars, &market);
    assert_eq!(state.step, 2);
    assert_eq!(state.time, 0.5);
    assert_eq!(state.spot(), Some(100.0));
    assert_eq!(state.interest_rate(), Some(0.03));
    assert_eq!(state.hazard_rate(), Some(0.02));
    assert_eq!(state.discount_factor(), Some(0.95));
    assert_eq!(state.get_var(state_keys::VOLATILITY), None);
    assert_eq!(state.get_var_or(state_keys::VOLATILITY, 0.2), 0.2);
}

#[test]
fn state_helper_builder_populates_expected_keys() {
    let single = single_factor_equity_state(100.0, 0.03, 0.01, 0.20);
    assert_eq!(single.get(state_keys::SPOT), Some(&100.0));
    assert_eq!(single.get(state_keys::INTEREST_RATE), Some(&0.03));
    assert_eq!(single.get(state_keys::DIVIDEND_YIELD), Some(&0.01));
    assert_eq!(single.get(state_keys::VOLATILITY), Some(&0.20));
    assert_eq!(single.len(), 4);
}

#[test]
fn evolution_params_builders_satisfy_basic_probability_invariants() {
    let crr = EvolutionParams::equity_crr(0.2, 0.05, 0.01, 0.25).expect("valid CRR params");
    assert!(crr.up_factor > 1.0);
    assert!(crr.down_factor < 1.0);
    assert!((crr.up_factor * crr.down_factor - 1.0).abs() < 1e-12);
    assert!(crr.prob_up >= 0.0 && crr.prob_up <= 1.0);
    assert!(crr.prob_down >= 0.0 && crr.prob_down <= 1.0);
    assert!((crr.prob_up + crr.prob_down - 1.0).abs() < 1e-12);

    let trinomial =
        EvolutionParams::equity_trinomial(0.2, 0.05, 0.01, 0.25).expect("valid trinomial params");
    assert!(trinomial.up_factor > 1.0);
    assert!(trinomial.down_factor < 1.0);
    assert_eq!(trinomial.middle_factor, Some(1.0));
    assert!(trinomial.prob_up >= 0.0);
    assert!(trinomial.prob_down >= 0.0);
    assert!(
        trinomial.prob_middle.is_some(),
        "middle probability should exist"
    );
    if let Some(p_mid) = trinomial.prob_middle {
        assert!(p_mid >= 0.0);
        assert!((trinomial.prob_up + trinomial.prob_down + p_mid - 1.0).abs() < 1e-10);
    }
}

#[test]
fn evolution_params_crr_rejects_unstable_params() {
    // dt large enough relative to vol that drift kicks p out of [0, 1].
    // Combined with extreme drift, the implied probability falls below zero
    // (or above one) — release builds must surface this rather than silently
    // produce an arbitrage-violating tree.
    let result = EvolutionParams::equity_crr(0.05, 5.0, 0.0, 1.0);
    assert!(
        result.is_err(),
        "CRR with extreme drift/vol/dt must error, not silently corrupt the tree"
    );
}

#[test]
fn evolution_params_trinomial_rejects_negative_probabilities() {
    // Extreme drift relative to vol pushes one trinomial probability negative.
    let result = EvolutionParams::equity_trinomial(0.02, 5.0, 0.0, 1.0);
    assert!(
        result.is_err(),
        "Trinomial with negative implied probability must error"
    );
}
