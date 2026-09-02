//! Shared node, evolution, and backward-induction components for pricing trees.
//!
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::HashMap;
use finstack_quant_core::Result;

use super::node_state::{CachedValues, NodeState};
use super::state_keys;
use super::traits::TreeValuator;

/// Inputs for the shared binomial recombining engine: constant per-step
/// evolution parameters or caller-supplied per-node generators.
#[derive(Clone)]
pub struct RecombiningInputs<'a, V: TreeValuator> {
    /// Number of time steps in the tree
    pub steps: usize,
    /// Initial state variable values at root node
    pub initial_vars: HashMap<&'static str, f64>,
    /// Time to maturity in years
    pub time_to_maturity: f64,
    /// Market data context for curve lookups
    pub market_context: &'a MarketContext,
    /// Payoff valuator implementing TreeValuator trait
    pub valuator: &'a V,
    /// Multiplicative factor for up move (e.g., exp(σ√dt))
    pub up_factor: f64,
    /// Multiplicative factor for down move (e.g., exp(-σ√dt))
    pub down_factor: f64,
    /// Risk-neutral probability of up move
    pub prob_up: f64,
    /// Risk-neutral probability of down move
    pub prob_down: f64,
    /// Risk-free interest rate per annum (used for discounting if custom_rate_generator is None)
    pub interest_rate: f64,
    /// Optional custom state generator for primary state variable (overrides up/down factors)
    pub custom_state_generator: Option<&'a dyn Fn(usize, usize) -> f64>,
    /// Optional custom rate generator for discounting (overrides interest_rate)
    pub custom_rate_generator: Option<&'a dyn Fn(usize, usize) -> f64>,
}

/// Price an instrument on a binomial recombining tree with backward induction.
///
/// Node `i` at step `n` has `i` up moves and `n - i` down moves. Payoffs are
/// evaluated at maturity and expected values are discounted backward to the
/// root. The evolving primary state variable is `SPOT` (equity trees) or
/// `INTEREST_RATE` (short-rate trees); it is threaded into the matching cached
/// field of [`NodeState`] so the induction loop performs no hashing.
///
/// # Arguments
///
/// * `inputs` - Complete tree configuration including evolution parameters,
///   valuator, and optional per-node generators
///
/// # Returns
///
/// Present value of the instrument at time 0
pub fn price_recombining_tree<V: TreeValuator>(inputs: RecombiningInputs<'_, V>) -> Result<f64> {
    let dt = inputs.time_to_maturity / inputs.steps as f64;

    // Constant discount factor when no custom rate generator is supplied.
    let flat_df = (-inputs.interest_rate * dt).exp();
    let get_df = |step: usize, node: usize| -> f64 {
        inputs
            .custom_rate_generator
            .map_or(flat_df, |rate_gen| (-rate_gen(step, node) * dt).exp())
    };

    let spot0 = *inputs
        .initial_vars
        .get(state_keys::SPOT)
        .or_else(|| inputs.initial_vars.get(state_keys::INTEREST_RATE))
        .ok_or_else(|| {
            finstack_quant_core::Error::internal(
                "tree pricing requires initial SPOT or INTEREST_RATE state",
            )
        })?;

    // Determine once which state key drives evolution and hoist every scalar
    // that stays constant across the whole tree.
    let uses_spot_key = inputs.initial_vars.contains_key(state_keys::SPOT);
    let cached_hazard = inputs.initial_vars.get(state_keys::HAZARD_RATE).copied();
    let const_spot = if uses_spot_key {
        None
    } else {
        inputs.initial_vars.get(state_keys::SPOT).copied()
    };
    let const_rate = if uses_spot_key {
        inputs.initial_vars.get(state_keys::INTEREST_RATE).copied()
    } else {
        None
    };
    let const_df = inputs.initial_vars.get(state_keys::DF).copied();
    let cached_for = |node_value: f64| -> CachedValues {
        if uses_spot_key {
            CachedValues {
                spot: Some(node_value),
                interest_rate: const_rate,
                hazard_rate: cached_hazard,
                df: const_df,
            }
        } else {
            CachedValues {
                spot: const_spot,
                interest_rate: Some(node_value),
                hazard_rate: cached_hazard,
                df: const_df,
            }
        }
    };

    // `initial_vars` is never mutated during induction: its constant keys
    // (volatility, dividend_yield, ...) remain available to valuators via
    // `NodeState::get_var`, while the evolving value rides in `CachedValues`.
    let node_vars = &inputs.initial_vars;

    // Node values for one level. Without a custom generator the level is
    // walked incrementally from `spot0 * d^step`, multiplying by `u/d` per node.
    let ud_ratio = inputs.up_factor / inputs.down_factor;
    let level_values = |step: usize, out: &mut Vec<f64>| {
        out.clear();
        match inputs.custom_state_generator {
            Some(state_gen) => out.extend((0..=step).map(|i| state_gen(step, i))),
            None => {
                let mut value = spot0 * inputs.down_factor.powi(step as i32);
                for i in 0..=step {
                    out.push(value);
                    if i < step {
                        value *= ud_ratio;
                    }
                }
            }
        }
    };

    let mut level = Vec::with_capacity(inputs.steps + 1);
    level_values(inputs.steps, &mut level);
    let mut values = Vec::with_capacity(inputs.steps + 1);
    for &node_value in &level {
        let terminal_state = NodeState::with_cached(
            inputs.steps,
            inputs.time_to_maturity,
            node_vars,
            inputs.market_context,
            cached_for(node_value),
        );
        values.push(inputs.valuator.value_at_maturity(&terminal_state)?);
    }

    for step in (0..inputs.steps).rev() {
        let time_t = step as f64 * dt;
        level_values(step, &mut level);
        for (i, &node_value) in level.iter().enumerate() {
            let continuation =
                get_df(step, i) * (inputs.prob_up * values[i + 1] + inputs.prob_down * values[i]);
            let node_state = NodeState::with_cached(
                step,
                time_t,
                node_vars,
                inputs.market_context,
                cached_for(node_value),
            );
            values[i] = inputs
                .valuator
                .value_at_node(&node_state, continuation, dt)?;
        }
        values.pop();
    }

    Ok(values[0])
}

/// Helper function to create initial state variables for single-factor equity model
///
/// # Arguments
///
/// * `spot` - Initial equity spot price in the option's quote currency.
/// * `risk_free_rate` - Continuously compounded domestic risk-free rate as a
///   decimal annual rate.
/// * `dividend_yield` - Continuously compounded equity dividend yield as a
///   decimal annual rate.
/// * `volatility` - Annualized equity diffusion volatility as a decimal.
pub fn single_factor_equity_state(
    spot: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> HashMap<&'static str, f64> {
    let mut vars = HashMap::default();
    vars.insert(state_keys::SPOT, spot);
    vars.insert(state_keys::INTEREST_RATE, risk_free_rate);
    vars.insert(state_keys::DIVIDEND_YIELD, dividend_yield);
    vars.insert(state_keys::VOLATILITY, volatility);
    vars
}
