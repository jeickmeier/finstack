//! Shared node, evolution, and backward-induction components for pricing trees.
//!
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::HashMap;

use super::state_keys;

/// Complete state information for a node in the pricing tree
#[derive(Clone)]
pub struct NodeState<'a> {
    /// Time step index (0 to N)
    pub step: usize,
    /// Time in years from valuation date
    pub time: f64,
    /// Map of all state variables at this node (reference to avoid cloning)
    pub vars: &'a HashMap<&'static str, f64>,
    /// Access to market context for additional data
    pub market_context: &'a MarketContext,
    /// Cached spot price for performance (avoids hash lookup)
    pub spot: Option<f64>,
    /// Cached interest rate for performance (avoids hash lookup)
    pub interest_rate: Option<f64>,
    /// Cached hazard rate for performance (avoids hash lookup)
    pub hazard_rate: Option<f64>,
    /// Cached discount factor for performance (avoids hash lookup)
    pub df: Option<f64>,
}

/// Pre-extracted state variable cache to avoid redundant HashMap lookups in hot paths.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CachedValues {
    /// Spot price
    pub spot: Option<f64>,
    /// Interest rate
    pub interest_rate: Option<f64>,
    /// Hazard rate (default intensity)
    pub hazard_rate: Option<f64>,
    /// Discount factor
    pub df: Option<f64>,
}

impl<'a> NodeState<'a> {
    /// Create a new node state, extracting the cached fields from `vars`.
    ///
    /// # Arguments
    ///
    /// * `step` - Zero-based time-step index of the node.
    /// * `time` - Node time in years from the valuation date.
    /// * `vars` - State variables at this node keyed by [`state_keys`] constants.
    /// * `market_context` - Market data made available to the valuator.
    pub fn new(
        step: usize,
        time: f64,
        vars: &'a HashMap<&'static str, f64>,
        market_context: &'a MarketContext,
    ) -> Self {
        let cached = CachedValues {
            spot: vars.get(state_keys::SPOT).copied(),
            interest_rate: vars.get(state_keys::INTEREST_RATE).copied(),
            hazard_rate: vars.get(state_keys::HAZARD_RATE).copied(),
            df: vars.get(state_keys::DF).copied(),
        };
        Self::with_cached(step, time, vars, market_context, cached)
    }

    /// Create a new node state with pre-extracted cached values.
    ///
    /// Avoids redundant HashMap lookups when the caller already knows the values.
    /// Used in hot paths (backward induction) where we just inserted the values.
    #[inline]
    pub(crate) fn with_cached(
        step: usize,
        time: f64,
        vars: &'a HashMap<&'static str, f64>,
        market_context: &'a MarketContext,
        cached: CachedValues,
    ) -> Self {
        Self {
            step,
            time,
            vars,
            market_context,
            spot: cached.spot,
            interest_rate: cached.interest_rate,
            hazard_rate: cached.hazard_rate,
            df: cached.df,
        }
    }

    /// Get a state variable by key
    #[inline]
    pub fn get_var(&self, key: &str) -> Option<f64> {
        self.vars.get(key).copied()
    }

    /// Get a state variable by key with a default value
    #[inline]
    pub fn get_var_or(&self, key: &str, default: f64) -> f64 {
        self.vars.get(key).copied().unwrap_or(default)
    }

    /// Get spot price (convenience method, uses cached value)
    #[inline]
    pub fn spot(&self) -> Option<f64> {
        self.spot
    }

    /// Get interest rate (convenience method, uses cached value)
    #[inline]
    pub fn interest_rate(&self) -> Option<f64> {
        self.interest_rate
    }

    /// Get hazard rate (convenience method, uses cached value)
    #[inline]
    pub fn hazard_rate(&self) -> Option<f64> {
        self.hazard_rate
    }

    /// Get discount factor (convenience method, uses cached value)
    #[inline]
    pub fn discount_factor(&self) -> Option<f64> {
        self.df
    }
}
