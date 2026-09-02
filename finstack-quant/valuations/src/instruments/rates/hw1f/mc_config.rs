//! Monte Carlo configuration for rate exotic products.
//!
//! Defines typed runtime settings and derives effective path and RNG-stream counts.

use serde::{Deserialize, Serialize};

/// Runtime Monte Carlo configuration shared across rate exotic pricers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RateExoticMcConfig {
    /// Total number of Monte Carlo paths (before antithetic doubling).
    pub num_paths: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Whether to use antithetic variates (doubles effective paths).
    pub antithetic: bool,
    /// Minimum number of simulation sub-steps between two consecutive
    /// observation/coupon dates. Ensures accurate short-rate dynamics.
    pub min_steps_between_events: usize,
    /// Polynomial basis degree for LSMC regression (only used by
    /// [`crate::instruments::rates::hw1f::hw1f_lsmc`]).
    pub basis_degree: usize,
    /// Split-sample (out-of-sample) LSMC pricing.
    ///
    /// When `true`, paths are partitioned by stream parity: even-indexed
    /// streams are used to fit the continuation-value regression, odd-indexed
    /// streams are used to price under that fitted policy. This removes the
    /// well-known positive in-sample bias of plain Longstaff-Schwartz at the
    /// cost of roughly √2× more standard error (half the paths drive the
    /// estimate). Aggregation reports stats on the pricing half only.
    ///
    /// Default is `false` (in-sample / Longstaff-Schwartz baseline) to keep
    /// existing pricing reproducible; enable for conservative bracketing of
    /// the true callable value.
    pub oos_lsmc: bool,
}

impl Default for RateExoticMcConfig {
    fn default() -> Self {
        let defaults = &finstack_quant_models::monte_carlo::registry::embedded_defaults_or_panic()
            .rust
            .rate_exotics;
        Self {
            num_paths: defaults.num_paths,
            seed: defaults.seed,
            antithetic: defaults.antithetic,
            min_steps_between_events: defaults.min_steps_between_events,
            basis_degree: defaults.basis_degree,
            oos_lsmc: false,
        }
    }
}

impl RateExoticMcConfig {
    /// Defaults for the LMM/BGM Bermudan swaption engine, read from the
    /// `rust.lmm_bermudan` block of the embedded pricer-defaults registry.
    ///
    /// The LMM engine simulates more paths and uses a cubic regression basis
    /// by default; `min_steps_between_events` is the minimum number of
    /// simulation sub-steps between consecutive exercise dates.
    pub fn lmm_bermudan() -> Self {
        let defaults = &finstack_quant_models::monte_carlo::registry::embedded_defaults_or_panic()
            .rust
            .lmm_bermudan;
        Self {
            num_paths: defaults.num_paths,
            seed: defaults.seed,
            antithetic: defaults.antithetic,
            min_steps_between_events: defaults.min_steps_between_exercises,
            basis_degree: defaults.basis_degree,
            oos_lsmc: false,
        }
    }

    /// Apply per-instrument overrides: an optional `mc_paths` from the
    /// instrument's model config (clamped to at least one antithetic pair) and
    /// a seed derived deterministically from the instrument id and the
    /// optional `mc_seed_scenario` label (`"base"` when absent).
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Instrument identifier that seeds the RNG stream.
    /// * `mc_paths` - Optional path-count override from `ModelConfig::mc_paths`.
    /// * `mc_seed_scenario` - Optional scenario label from
    ///   `MetricPricingOverrides::mc_seed_scenario` used to derive the seed.
    #[must_use]
    pub fn with_instrument_overrides(
        mut self,
        instrument_id: &finstack_quant_core::types::InstrumentId,
        mc_paths: Option<usize>,
        mc_seed_scenario: Option<&str>,
    ) -> Self {
        if let Some(paths) = mc_paths {
            self.num_paths = paths.max(self.split().multiplicity);
        }
        self.seed = finstack_quant_models::monte_carlo::seed::derive_seed(
            instrument_id,
            mc_seed_scenario.unwrap_or("base"),
        );
        self
    }

    /// Path-index partition implied by `antithetic` and `oos_lsmc`.
    ///
    /// Antithetic legs of one raw stream occupy consecutive path slots, so
    /// path `p` belongs to raw stream `p / multiplicity`. In split-sample
    /// mode even-indexed streams train the continuation regression and
    /// odd-indexed streams are priced; otherwise every path is both.
    pub fn split(&self) -> SampleSplit {
        SampleSplit {
            multiplicity: if self.antithetic { 2 } else { 1 },
            oos_lsmc: self.oos_lsmc,
        }
    }

    /// Total effective Monte Carlo paths generated. With `antithetic = true`,
    /// returns `num_paths` rounded **down to the nearest even number** (antithetic
    /// paths come in pairs); with `antithetic = false`, returns `num_paths`
    /// unchanged.
    pub fn effective_path_count(&self) -> usize {
        if self.antithetic {
            self.num_paths / 2 * 2
        } else {
            self.num_paths
        }
    }

    /// Number of distinct RNG shock streams required. With `antithetic = true`
    /// each stream is replayed twice (once with negated shocks), so this is
    /// `num_paths / 2`. With `antithetic = false` it equals `num_paths`.
    pub fn raw_stream_count(&self) -> usize {
        if self.antithetic {
            self.num_paths / 2
        } else {
            self.num_paths
        }
    }
}

/// Train/price partition of simulated paths (see [`RateExoticMcConfig::split`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSplit {
    /// Paths per raw RNG stream: 2 with antithetic sampling, else 1.
    pub multiplicity: usize,
    /// Whether the split-sample (out-of-sample) estimator is active.
    pub oos_lsmc: bool,
}

impl SampleSplit {
    /// Whether path `p` contributes to the continuation-value regression.
    #[inline]
    pub fn is_train(&self, p: usize) -> bool {
        !self.oos_lsmc || (p / self.multiplicity).is_multiple_of(2)
    }

    /// Whether path `p` contributes to the reported price estimate.
    #[inline]
    pub fn is_price(&self, p: usize) -> bool {
        !self.oos_lsmc || !(p / self.multiplicity).is_multiple_of(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_partitions_by_stream_parity() {
        let in_sample = RateExoticMcConfig {
            antithetic: true,
            oos_lsmc: false,
            ..Default::default()
        }
        .split();
        assert!((0..8).all(|p| in_sample.is_train(p) && in_sample.is_price(p)));

        let oos = RateExoticMcConfig {
            antithetic: true,
            oos_lsmc: true,
            ..Default::default()
        }
        .split();
        assert_eq!(oos.multiplicity, 2);
        // Stream 0 = paths {0,1} train; stream 1 = paths {2,3} price.
        assert!(oos.is_train(0) && oos.is_train(1));
        assert!(!oos.is_price(0) && !oos.is_price(1));
        assert!(oos.is_price(2) && oos.is_price(3));
        assert!(!oos.is_train(2) && !oos.is_train(3));
    }

    #[test]
    fn default_values() {
        let cfg = RateExoticMcConfig::default();
        assert_eq!(cfg.num_paths, 20_000);
        assert_eq!(cfg.seed, 42);
        assert!(cfg.antithetic);
        assert_eq!(cfg.min_steps_between_events, 4);
        assert_eq!(cfg.basis_degree, 2);
    }

    #[test]
    fn effective_path_count_antithetic() {
        let cfg = RateExoticMcConfig {
            num_paths: 101,
            antithetic: true,
            ..Default::default()
        };
        // 101/2 = 50 streams * 2 = 100 (odd half path dropped)
        assert_eq!(cfg.effective_path_count(), 100);
    }

    #[test]
    fn raw_stream_count_antithetic_and_non() {
        let a = RateExoticMcConfig {
            num_paths: 100,
            antithetic: true,
            ..Default::default()
        };
        assert_eq!(a.raw_stream_count(), 50);
        let b = RateExoticMcConfig {
            num_paths: 100,
            antithetic: false,
            ..Default::default()
        };
        assert_eq!(b.raw_stream_count(), 100);
    }
}
