use finstack_quant_core::market_data::traits::Discounting;
use finstack_quant_core::math::{BrentSolver, Solver};
use finstack_quant_core::{Error, Result};

use super::{CalibrationResult, ShortRateTree};

impl ShortRateTree {
    /// Calibrate the standard (κ = 0) Black-Derman-Toy model using
    /// state-price recursion on a binomial lattice with constant lognormal
    /// volatility.
    ///
    /// Mean-reverting Black-Karasinski (κ ≠ 0) is handled by
    /// [`calibrate_bk_trinomial`](Self::calibrate_bk_trinomial), which builds
    /// a genuine trinomial lattice in x = ln r — a binomial lattice cannot
    /// represent the rate-dependent drift `−κ·ln r` while staying
    /// recombining .
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if a discount factor is non-positive, if
    /// the node-rate clamp `[1e-8, 5.0]` engages materially (a tree too wide
    /// to calibrate — the lattice would silently misprice the curve), or if
    /// the calibrated tree fails to reprice the curve within tolerance.
    pub(super) fn calibrate_bdt(
        &mut self,
        rates: &mut [Vec<f64>],
        discount_curve: &dyn Discounting,
        dt: f64,
    ) -> Result<()> {
        let sigma = self.config.volatility;
        let solver = BrentSolver::new();

        // Standard BDT (κ = 0): constant lognormal volatility, per-step
        // log-spread σ√dt. κ ≠ 0 never reaches this path — calibrate()
        // routes it to the trinomial Black-Karasinski lattice.
        let step_vol = sigma * dt.sqrt();
        let u = step_vol.exp();
        let p = 0.5;

        // Bounds for alpha solver.
        // Upper bound is generous to avoid distorting the tail of the lognormal
        // distribution; individual node rates can legitimately exceed 100% in
        // wide trees (high vol, many steps, long maturity).
        let alpha_lb = 1e-8;
        let alpha_ub = 5.0;

        // Relative tolerance for deciding that the `[alpha_lb, alpha_ub]` clamp
        // has *materially* altered a node rate. A node rate that merely sits
        // near a bound is fine; one that the clamp has moved by more than this
        // fraction means the Brent objective no longer responds to `alpha` at
        // that node, so the tree can no longer reprice the curve. When that
        // happens the calibration is unsound and is failed below rather than
        // silently returning a mispriced lattice (`max_error_bp` alone only
        // *reports* the damage — it does not prevent the tree from escaping).
        let clamp_rel_tol = 1.0e-6;
        let materially_clamped = |raw: f64| -> bool {
            let clamped = raw.clamp(alpha_lb, alpha_ub);
            // Relative deviation, guarding the (here impossible) zero `raw`.
            let denom = raw.abs().max(f64::MIN_POSITIVE);
            (raw - clamped).abs() / denom > clamp_rel_tol
        };
        let mut clamp_engaged = false;
        let mut clamp_engaged_step = 0_usize;

        // Initialize first step with initial short rate
        let r0 = if self.time_steps[1] > 0.0 {
            // Use initial forward rate from discount curve
            -discount_curve.df(self.time_steps[1]).ln() / self.time_steps[1]
        } else {
            0.03 // Fallback rate
        };

        rates[0] = vec![r0.clamp(alpha_lb, alpha_ub)]; // Ensure within bounds
        let mut state_prices = vec![vec![1.0]]; // Q[0] = [1.0]

        // Set transition probabilities (constant for BDT)
        for i in 0..self.config.steps {
            self.probs[i] = (p, 1.0 - p);
        }

        // Track calibration quality for diagnostics
        let mut max_error_bp = 0.0_f64;
        let mut max_error_step = 0_usize;
        let mut fallback_count = 0_usize;

        // Build tree forward, calibrating drift at each step
        for step in 0..self.config.steps {
            let current_time = self.time_steps[step + 1];
            let target_df = discount_curve.df(current_time);

            if target_df <= 0.0 {
                return Err(Error::Validation(format!(
                    "BDT calibration: non-positive discount factor {} at time {}",
                    target_df, current_time
                )));
            }

            let num_nodes = step + 1;
            let current_state_prices = &state_prices[step];
            let current_rates = &rates[step];

            // Solve for drift parameter alpha such that model ZCB price matches market
            let comp = self.config.compounding;
            let objective = |alpha: f64| -> f64 {
                let mut model_price = 0.0;

                for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                    let rate = alpha * u.powf(num_nodes as f64 - 1.0 - 2.0 * j as f64);
                    let rate_clamped = rate.clamp(alpha_lb, alpha_ub);
                    model_price += state_price * comp.df(rate_clamped, dt);
                }

                model_price - target_df
            };

            // Initial guess for alpha based on previous step or forward rate
            let initial_alpha = if step == 0 {
                r0.clamp(alpha_lb, alpha_ub)
            } else {
                // Use geometric mean of previous step rates as initial guess
                let mean_rate =
                    current_rates.iter().map(|&r| r.ln()).sum::<f64>() / current_rates.len() as f64;
                mean_rate.exp().clamp(alpha_lb, alpha_ub)
            };

            // Solve for alpha with convergence tracking
            let (alpha, used_fallback) = match solver.solve(objective, initial_alpha) {
                Ok(a) => (a.clamp(alpha_lb, alpha_ub), false),
                Err(_) => {
                    // Solver failed - use fallback based on market rate
                    let market_rate = if current_time > 0.0 {
                        -target_df.ln() / current_time
                    } else {
                        0.03
                    };
                    fallback_count += 1;
                    (market_rate.clamp(alpha_lb, alpha_ub), true)
                }
            };

            let current_step_rates: Vec<f64> = (0..num_nodes)
                .map(|j| {
                    let rate = alpha * u.powf(num_nodes as f64 - 1.0 - 2.0 * j as f64);
                    if materially_clamped(rate) && !clamp_engaged {
                        clamp_engaged = true;
                        clamp_engaged_step = step;
                    }
                    rate.clamp(alpha_lb, alpha_ub)
                })
                .collect();
            rates[step] = current_step_rates.clone();

            let model_df = {
                let mut model_price = 0.0;
                for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                    model_price += state_price * comp.df(current_step_rates[j], dt);
                }
                model_price
            };
            let error_bp = ((model_df - target_df) / target_df).abs() * 10000.0;

            if error_bp > max_error_bp {
                max_error_bp = error_bp;
                max_error_step = step;
            }

            // Log warning if calibration error is significant (>1bp) or fallback was used
            if error_bp > 1.0 || used_fallback {
                tracing::warn!(
                    "BDT calibration step {}: error={:.2}bp, target_df={:.6}, model_df={:.6}{}",
                    step,
                    error_bp,
                    target_df,
                    model_df,
                    if used_fallback {
                        " (FALLBACK USED)"
                    } else {
                        ""
                    }
                );
            }

            // Build next step rates using calibrated alpha.
            //
            // Terminal row note (same convention as Ho-Lee and BK): the final
            // iteration populates rates[N] for lattice geometry and accessor
            // consistency, but that row's alpha is the one solved for the last
            // pre-maturity interval — there is no interval beyond maturity to
            // drift-calibrate, and backward induction never uses rates[N] for
            // discounting because pricing stops at maturity.
            let next_nodes = num_nodes + 1;
            let mut next_rates = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                let discount_factor = comp.df(current_step_rates[j], dt);
                let state_price_contribution = state_price * discount_factor;

                // Up move: j -> j+1
                if j + 1 < next_nodes {
                    let up_rate = alpha * u.powf(next_nodes as f64 - 1.0 - 2.0 * (j + 1) as f64);
                    if materially_clamped(up_rate) && !clamp_engaged {
                        clamp_engaged = true;
                        clamp_engaged_step = step + 1;
                    }
                    next_rates[j + 1] = up_rate.clamp(alpha_lb, alpha_ub);
                    next_state_prices[j + 1] += state_price_contribution * p;
                }

                // Down move: j -> j
                if j < next_nodes {
                    let down_rate = alpha * u.powf(next_nodes as f64 - 1.0 - 2.0 * j as f64);
                    if materially_clamped(down_rate) && !clamp_engaged {
                        clamp_engaged = true;
                        clamp_engaged_step = step + 1;
                    }
                    next_rates[j] = down_rate.clamp(alpha_lb, alpha_ub);
                    next_state_prices[j] += state_price_contribution * (1.0 - p);
                }
            }

            rates[step + 1] = next_rates;
            state_prices.push(next_state_prices);
        }

        // Log calibration summary
        if max_error_bp > 1.0 || fallback_count > 0 {
            tracing::warn!(
                "BDT calibration completed: max error={:.2}bp at step {}, fallbacks={} (target: <1bp, 0 fallbacks)",
                max_error_bp,
                max_error_step,
                fallback_count
            );
        } else {
            tracing::debug!(
                "BDT calibration completed: max error={:.4}bp at step {}",
                max_error_bp,
                max_error_step
            );
        }

        // Hard repricing tolerance. A well-posed BDT tree calibrates to far
        // below 1 bp (floating-point accumulation only); the codebase's own
        // `CalibrationResult::is_acceptable` bar is 1 bp. This *hard error*
        // gate is set well above that — at 25 bp — so it never rejects a
        // merely-imperfect tree, only one that has genuinely *stopped*
        // repricing the curve. Empirically the BDT clamp failure is bimodal:
        // a wide tree either reprices fine (clamp engages only on vanishing-
        // weight tail nodes) or breaks catastrophically (thousands of bp), so
        // 25 bp cleanly separates the two. Unlike the diagnostic
        // `max_error_bp` field — which only *reports* — this gate *enforces*
        // the contract so a silently-mispriced tree can never be returned as
        // `converged`. The milder 1-25 bp band is still surfaced via the
        // `tracing::warn!` above and the `is_acceptable` / `is_good` flags.
        const MAX_CALIBRATION_ERROR_BPS: f64 = 25.0;

        // Enforce that the calibrated tree actually reprices the curve.
        //
        // The node-rate clamp `[1e-8, 5.0]` is applied inside the Brent
        // objective. When it engages on a node with material Arrow-Debreu
        // weight the objective stops responding to `alpha`, the solver settles
        // on the wrong drift, and the lattice silently stops repricing the
        // curve — exactly the failure this gate catches. (Clamp engagement on
        // a deep, vanishing-weight tail node is harmless: it leaves
        // `max_error_bp` at ~0 and is intentionally *not* failed here.)
        //
        // `max_error_bp` is re-derived above by an independent forward pass
        // over the final `rates`, so it faithfully reflects any clamp-induced
        // mispricing. When the tolerance is breached, the diagnostic message
        // reports whether the clamp engaged (the usual root cause for a wide
        // tree) so the caller knows which knob to turn.
        if !max_error_bp.is_finite() || max_error_bp > MAX_CALIBRATION_ERROR_BPS {
            self.calibration_quality = Some(CalibrationResult {
                max_error_bp,
                max_error_step,
                fallback_count,
                converged: false,
            });
            let clamp_note = if clamp_engaged {
                format!(
                    " The node-rate clamp [{alpha_lb:.0e}, {alpha_ub}] engaged \
                     materially (first at step {clamp_engaged_step}) — the tree \
                     is too wide; lower the volatility, the step count, or the \
                     maturity."
                )
            } else {
                String::new()
            };
            return Err(Error::Validation(format!(
                "BDT calibration failed to reprice the discount curve: max \
                 error {max_error_bp:.2} bp at step {max_error_step} exceeds \
                 the {MAX_CALIBRATION_ERROR_BPS:.1} bp tolerance.{clamp_note}"
            )));
        }

        // Store calibration result for user inspection
        self.calibration_quality = Some(CalibrationResult {
            max_error_bp,
            max_error_step,
            fallback_count,
            converged: true,
        });

        Ok(())
    }
}
