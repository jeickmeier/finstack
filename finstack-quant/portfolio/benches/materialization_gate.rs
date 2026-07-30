//! Deterministic percentile and absolute-gate logic for materialization benches.

/// Strict native cold-fixture-A acceptance threshold.
pub const COLD_HARD_LIMIT_MILLIS: f64 = 1_000.0;
/// Native cold-fixture-A engineering target.
pub const COLD_ENGINEERING_TARGET_MILLIS: f64 = 500.0;
/// Native warm-fixture-B engineering target.
pub const WARM_ENGINEERING_TARGET_MILLIS: f64 = 250.0;
/// Minimum number of samples in an acceptance benchmark.
pub const MIN_ACCEPTANCE_SAMPLES: usize = 100;

/// Result of evaluating the absolute latency gates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatencyGateVerdict {
    /// Whether cold fixture A is strictly below the hard one-second gate.
    pub cold_hard_pass: bool,
    /// Whether cold fixture A is strictly below the 500 ms engineering target.
    pub cold_target_pass: bool,
    /// Whether warm fixture B is strictly below the 250 ms engineering target.
    pub warm_target_pass: bool,
}

/// Return the nearest-rank p95 for non-empty millisecond samples.
///
/// # Arguments
///
/// * `samples` - Mutable finite latency samples; sorted in place.
///
/// # Panics
///
/// Panics for an empty sample set or a non-finite sample.
#[must_use]
pub fn p95_millis(samples: &mut [f64]) -> f64 {
    assert!(!samples.is_empty(), "p95 requires at least one sample");
    assert!(
        samples.iter().all(|sample| sample.is_finite()),
        "p95 samples must be finite"
    );
    samples.sort_by(f64::total_cmp);
    let rank = (samples.len() * 95).div_ceil(100);
    samples[rank.saturating_sub(1)]
}

/// Parse a benchmark sample count under the acceptance/smoke policy.
///
/// # Arguments
///
/// * `value` - Optional decimal integer supplied by the benchmark environment.
/// * `allow_smoke` - Whether a deliberately short test-only smoke run is allowed.
///
/// # Errors
///
/// Returns an error when the value is not a positive integer or, for an
/// acceptance run, is below [`MIN_ACCEPTANCE_SAMPLES`].
pub fn parse_sample_count(value: Option<&str>, allow_smoke: bool) -> Result<usize, String> {
    let samples = match value {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|_| format!("sample count must be a finite integer, got '{raw}'"))?,
        None => MIN_ACCEPTANCE_SAMPLES,
    };
    let minimum = if allow_smoke {
        1
    } else {
        MIN_ACCEPTANCE_SAMPLES
    };
    if samples < minimum {
        return Err(format!(
            "sample count must be at least {minimum}, got {samples}"
        ));
    }
    Ok(samples)
}

/// Evaluate cold and warm p95 values against Phase 7 thresholds.
///
/// # Arguments
///
/// * `cold_a_p95_millis` - Native cold fixture A nearest-rank p95 in milliseconds.
/// * `warm_b_p95_millis` - Native warm fixture B nearest-rank p95 in milliseconds.
#[must_use]
pub fn evaluate_latency_gates(
    cold_a_p95_millis: f64,
    warm_b_p95_millis: f64,
) -> LatencyGateVerdict {
    LatencyGateVerdict {
        cold_hard_pass: cold_a_p95_millis < COLD_HARD_LIMIT_MILLIS,
        cold_target_pass: cold_a_p95_millis < COLD_ENGINEERING_TARGET_MILLIS,
        warm_target_pass: warm_b_p95_millis < WARM_ENGINEERING_TARGET_MILLIS,
    }
}
