use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::{Error, HashMap, Result};

use crate::models::trees::hull_white_tree::HullWhiteTree;
use crate::models::trees::tree_framework::{
    price_recombining_tree, state_keys, CachedValues, NodeState, RecombiningInputs, TreeBranching,
    TreeGreeks, TreeModel, TreeValuator,
};

use super::black_karasinski::BkTrinomialLattice;
use super::ShortRateTree;

impl ShortRateTree {
    /// Backward induction over the Black-Karasinski trinomial lattice.
    ///
    /// Honors the per-node transition probabilities, the Hull & White edge
    /// branch switching, and the configured per-node compounding. OAS is an
    /// independently compounded continuous spread, so periodic node-rate
    /// conventions do not distort its discount factor.
    fn price_bk_trinomial<V: TreeValuator>(
        &self,
        lattice: &BkTrinomialLattice,
        initial_vars: &HashMap<&'static str, f64>,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
        oas: f64,
    ) -> Result<f64> {
        let steps = self.config.steps;
        let dt = time_to_maturity / steps as f64;
        let comp = self.config.compounding;
        let oas_shift = oas / 10_000.0;
        let j_max = lattice.j_max;

        let cached_hazard = initial_vars.get(state_keys::HAZARD_RATE).copied();
        let cached_spot = initial_vars.get(state_keys::SPOT).copied();
        let cached_df = initial_vars.get(state_keys::DF).copied();
        let cached_for = |rate: f64| -> CachedValues {
            CachedValues {
                spot: cached_spot,
                interest_rate: Some(rate),
                hazard_rate: cached_hazard,
                df: cached_df,
            }
        };

        // Terminal payoffs.
        let mut values: Vec<f64> = Vec::with_capacity(self.rates[steps].len());
        for &r in self.rates[steps].iter() {
            let state = NodeState::with_cached(
                steps,
                time_to_maturity,
                initial_vars,
                market_context,
                cached_for(r + oas_shift),
            );
            values.push(valuator.value_at_maturity(&state)?);
        }

        // Backward induction with per-node probabilities.
        let mut scratch: Vec<f64> = Vec::new();
        for step in (0..steps).rev() {
            let curr_j_max = step.min(j_max);
            let next_j_max = (step + 1).min(j_max);
            let num_nodes = 2 * curr_j_max + 1;
            let boundary_j_max = if curr_j_max == next_j_max {
                curr_j_max
            } else {
                usize::MAX
            };
            let time_t = step as f64 * dt;

            scratch.clear();
            for j in 0..num_nodes {
                let j_signed = j as i32 - curr_j_max as i32;
                let node_probs = lattice.probs[step][j];

                let mut expected_value = 0.0;
                for (offset, probability) in
                    HullWhiteTree::transition_offsets(j_signed, boundary_j_max, node_probs)
                {
                    if let Some(idx) = HullWhiteTree::transition_index(j_signed, offset, next_j_max)
                    {
                        if idx < values.len() {
                            expected_value += probability * values[idx];
                        }
                    }
                }

                let r = self.rates[step][j];
                let continuation = expected_value * comp.df(r, dt) * (-oas_shift * dt).exp();
                let state = NodeState::with_cached(
                    step,
                    time_t,
                    initial_vars,
                    market_context,
                    cached_for(r + oas_shift),
                );
                scratch.push(valuator.value_at_node(&state, continuation, dt)?);
            }
            std::mem::swap(&mut values, &mut scratch);
        }

        values.first().copied().ok_or_else(|| {
            Error::internal("Black-Karasinski backward induction produced no root value")
        })
    }
}

impl TreeModel for ShortRateTree {
    fn price<V: TreeValuator>(
        &self,
        mut initial_vars: HashMap<&'static str, f64>,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        if self.rates.is_empty() {
            tracing::debug!("ShortRateTree::price called before calibration (rates is empty)");
            return Err(Error::internal(
                "short-rate tree must be calibrated before pricing",
            ));
        }
        self.validate_lattice_geometry()?;

        // Ensure initial rate is present
        if !initial_vars.contains_key(state_keys::INTEREST_RATE) {
            if let Some(row) = self.rates.first() {
                if let Some(&r0) = row.first() {
                    initial_vars.insert(state_keys::INTEREST_RATE, r0);
                }
            }
        }

        // Get OAS from initial variables (default to 0)
        let oas = initial_vars.get("oas").copied().unwrap_or(0.0);

        // Black-Karasinski trinomial lattice: per-node probabilities and
        // capped width with branch switching cannot be expressed through the
        // constant-probability recombining engine, so it has a dedicated
        // backward induction.
        if let Some(lattice) = &self.bk_trinomial {
            return self.price_bk_trinomial(
                lattice,
                &initial_vars,
                time_to_maturity,
                market_context,
                valuator,
                oas,
            );
        }

        // Create custom state generator that uses pre-calibrated rates
        // Clone rates (cheap Arc clone) to avoid lifetime issues with closures
        let rates_clone = std::sync::Arc::clone(&self.rates);
        let state_gen: Box<dyn Fn(usize, usize) -> f64> =
            Box::new(move |step: usize, node: usize| -> f64 {
                if step < rates_clone.len() && node < rates_clone[step].len() {
                    rates_clone[step][node]
                } else {
                    0.0 // Fallback
                }
            });

        let rates_clone2 = std::sync::Arc::clone(&self.rates);
        let compounding = self.config.compounding;
        let dt_pricing = time_to_maturity / self.config.steps as f64;
        let rate_gen: Box<dyn Fn(usize, usize) -> f64> =
            Box::new(move |step: usize, node: usize| -> f64 {
                let r = if step < rates_clone2.len() && node < rates_clone2[step].len() {
                    rates_clone2[step][node]
                } else {
                    return 0.0;
                };
                compounding.to_continuous(r, dt_pricing) + oas / 10000.0
            });

        // Set up branching probabilities based on tree type
        let (p_up, p_down, p_middle) = match self.config.branching {
            TreeBranching::Trinomial => {
                // Trinomial: equal probabilities for up/mid/down
                // This provides better numerical stability for mean-reverting models
                (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
            }
            TreeBranching::Binomial => {
                // Binomial: use calibrated probabilities if available, else 50/50
                let (pu, pd) = self.probs.first().copied().unwrap_or((0.5, 0.5));
                (pu, pd, 0.0)
            }
        };

        price_recombining_tree(RecombiningInputs {
            branching: self.config.branching,
            steps: self.config.steps,
            initial_vars,
            time_to_maturity,
            market_context,
            valuator,
            up_factor: 1.0,   // Not used with custom_state_generator
            down_factor: 1.0, // Not used with custom_state_generator
            middle_factor: if self.config.branching == TreeBranching::Trinomial {
                Some(1.0)
            } else {
                None
            },
            prob_up: p_up,
            prob_down: p_down,
            prob_middle: Some(p_middle),
            interest_rate: 0.0, // Not used with custom_rate_generator
            barrier: None,
            custom_state_generator: Some(&*state_gen),
            custom_rate_generator: Some(&*rate_gen),
        })
    }

    fn calculate_greeks<V: TreeValuator>(
        &self,
        initial_vars: HashMap<&'static str, f64>,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
        bump_size: Option<f64>,
    ) -> Result<TreeGreeks> {
        let base_price = self.price(
            initial_vars.clone(),
            time_to_maturity,
            market_context,
            valuator,
        )?;

        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };

        // Default: relative 10% of the calibrated vol, floored at 1 bp. A
        // fixed absolute 0.01 bump was a 100% relative bump for a typical
        // normal σ = 1% short-rate vol, which badly distorts the FD vega.
        // Vega is still reported per 1% (absolute) vol move below.
        let vol_bump = bump_size.unwrap_or((0.1 * self.config.volatility).max(1e-4));
        let curve_id = &self.calibration_curve_id;

        // Vega and theta require recalibrating fresh trees against the discount
        // curve.  The curve is looked up from MarketContext using the CurveId
        // stored during calibrate().
        if let Ok(discount_curve) = market_context.get_discount(curve_id) {
            // --- Vega (central difference with correct denominator) -----------
            let vol_up = self.config.volatility + vol_bump;
            let vol_down = (self.config.volatility - vol_bump).max(1e-6);

            let mut config_up = self.config.clone();
            config_up.volatility = vol_up;
            let mut tree_up = ShortRateTree::new(config_up);
            if tree_up
                .calibrate(curve_id, discount_curve.as_ref(), time_to_maturity)
                .is_ok()
            {
                let price_up = tree_up.price(
                    initial_vars.clone(),
                    time_to_maturity,
                    market_context,
                    valuator,
                )?;

                let mut config_down = self.config.clone();
                config_down.volatility = vol_down;
                let mut tree_down = ShortRateTree::new(config_down);
                if tree_down
                    .calibrate(curve_id, discount_curve.as_ref(), time_to_maturity)
                    .is_ok()
                {
                    let price_down = tree_down.price(
                        initial_vars.clone(),
                        time_to_maturity,
                        market_context,
                        valuator,
                    )?;

                    let actual_span = vol_up - vol_down;
                    greeks.vega = (price_up - price_down) / actual_span * 0.01;
                } else {
                    greeks.vega = (price_up - base_price) / vol_bump * 0.01;
                }
            }

            // --- Theta (recalibrate a fresh tree for bumped maturity) ---------
            let dt_theta = 1.0 / 365.25;
            let ttm_tomorrow = time_to_maturity - dt_theta;
            if ttm_tomorrow > 0.0 {
                let mut tree_tomorrow = ShortRateTree::new(self.config.clone());
                if tree_tomorrow
                    .calibrate(curve_id, discount_curve.as_ref(), ttm_tomorrow)
                    .is_ok()
                {
                    let price_tomorrow = tree_tomorrow.price(
                        initial_vars.clone(),
                        ttm_tomorrow,
                        market_context,
                        valuator,
                    )?;
                    greeks.theta = -(base_price - price_tomorrow) / dt_theta;
                }
            }
        } else {
            tracing::debug!(
                "ShortRateTree::calculate_greeks: discount curve '{}' not found; \
                 vega and theta set to 0",
                curve_id.as_str()
            );
        }

        // Rho: OAS sensitivity (price change per 1 bp parallel spread bump).
        // Note: this measures sensitivity to the option-adjusted spread, not to
        // a parallel shift of the underlying yield curve. For bonds with embedded
        // options the two are not equivalent because an OAS bump does not change
        // the exercise boundary while a curve bump does.
        let mut bumped_vars = initial_vars;
        let base_oas = bumped_vars.get("oas").copied().unwrap_or(0.0);
        bumped_vars.insert("oas", base_oas + 1.0);

        let bumped_price = self.price(bumped_vars, time_to_maturity, market_context, valuator)?;
        greeks.rho = bumped_price - base_price;

        Ok(greeks)
    }
}
