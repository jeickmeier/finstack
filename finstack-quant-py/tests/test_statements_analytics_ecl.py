"""Statements analytics ECL binding tests."""

from __future__ import annotations

import pytest


def test_classify_stage_uses_canonical_defaults_and_backstop_toggles() -> None:
    """Exposure and classification defaults are resolved by canonical Rust."""
    from finstack_quant.statements_analytics import Exposure, classify_stage

    exposure = Exposure("loan", 100.0, 0.4, 0.05, 5.0, 0.02, 0.015)
    assert exposure.dpd == 0
    assert classify_stage(exposure) == ("Stage 1", "no_trigger")

    exposure.dpd = 31
    stage, reason = classify_stage(exposure)
    assert stage == "Stage 2"
    assert reason.startswith("dpd_stage2")
    assert classify_stage(exposure, dpd_30_trigger=False) == ("Stage 1", "no_trigger")

    exposure.dpd = 91
    stage, reason = classify_stage(exposure)
    assert stage == "Stage 3"
    assert reason.startswith("dpd_stage3")
    assert classify_stage(exposure, dpd_30_trigger=False, dpd_90_trigger=False) == (
        "Stage 1",
        "no_trigger",
    )


def test_classify_stage_uses_default_or_explicit_pd_threshold() -> None:
    """The default PD threshold and caller override share the Rust request."""
    from finstack_quant.statements_analytics import Exposure, classify_stage

    exposure = Exposure("loan", 100.0, 0.4, 0.05, 5.0, 0.026, 0.015)
    stage, reason = classify_stage(exposure)
    assert stage == "Stage 2"
    assert reason.startswith("pd_delta_absolute")
    assert classify_stage(exposure, pd_delta_stage2=0.02) == ("Stage 1", "no_trigger")


def test_compute_ecl_weighted_validates_scenario_weights() -> None:
    """Weighted ECL rejects malformed scenario probabilities from Rust."""
    from finstack_quant.statements_analytics import compute_ecl_weighted

    scenarios = [
        (0.75, [(0.0, 0.0), (1.0, 0.02)]),
        (0.20, [(0.0, 0.0), (1.0, 0.05)]),
    ]

    with pytest.raises(ValueError, match=r"scenario weights must sum to 1\.0"):
        compute_ecl_weighted(1_000_000.0, scenarios, 0.45, 0.06, 1.0)


def test_compute_ecl_weighted_preserves_public_error_mapping() -> None:
    """Missing scenarios and PD schedules remain public ValueError failures."""
    from finstack_quant.statements_analytics import compute_ecl_weighted

    with pytest.raises(ValueError, match="At least one scenario is required for weighted ECL"):
        compute_ecl_weighted(1_000_000.0, [], 0.45, 0.06, 1.0)

    with pytest.raises(ValueError, match="At least two data points are required"):
        compute_ecl_weighted(1_000_000.0, [(1.0, [])], 0.45, 0.06, 1.0)


def test_compute_ecl_weighted_returns_probability_weighted_ecl() -> None:
    """Weighted ECL binding delegates the scenario aggregation to Rust."""
    from finstack_quant.statements_analytics import compute_ecl, compute_ecl_weighted

    base_curve = [(0.0, 0.0), (1.0, 0.02)]
    downside_curve = [(0.0, 0.0), (1.0, 0.05)]

    base = compute_ecl(1_000_000.0, base_curve, 0.45, 0.06, 1.0)
    downside = compute_ecl(1_000_000.0, downside_curve, 0.45, 0.06, 1.0)
    weighted = compute_ecl_weighted(
        1_000_000.0,
        [(0.70, base_curve), (0.30, downside_curve)],
        0.45,
        0.06,
        1.0,
    )

    assert weighted == pytest.approx(0.70 * base + 0.30 * downside, rel=1e-12)


def test_compute_ecl_weighted_anchors_unanchored_schedules() -> None:
    """Both ECL entry points accept schedules without an explicit (0, 0) knot."""
    from finstack_quant.statements_analytics import compute_ecl, compute_ecl_weighted

    anchored = [(0.0, 0.0), (1.0, 0.02)]
    unanchored = [(1.0, 0.02)]

    assert compute_ecl(1_000_000.0, unanchored, 0.45, 0.06, 1.0) == pytest.approx(
        compute_ecl(1_000_000.0, anchored, 0.45, 0.06, 1.0)
    )
    assert compute_ecl_weighted(1_000_000.0, [(1.0, unanchored)], 0.45, 0.06, 1.0) == pytest.approx(
        compute_ecl_weighted(1_000_000.0, [(1.0, anchored)], 0.45, 0.06, 1.0)
    )


def test_compute_ecl_ead_schedule_reduces_lifetime_ecl() -> None:
    """An amortizing EAD profile lowers ECL versus a constant balance."""
    from finstack_quant.statements_analytics import compute_ecl

    curve = [(0.0, 0.0), (5.0, 0.10)]
    bullet = compute_ecl(1_000_000.0, curve, 0.45, 0.06, 5.0, stage="stage2")
    amortizing = compute_ecl(
        1_000_000.0,
        curve,
        0.45,
        0.06,
        5.0,
        stage="stage2",
        ead_schedule=[(0.0, 1_000_000.0), (5.0, 0.0)],
    )
    assert amortizing < bullet


def test_compute_ecl_stage3_time_to_recovery() -> None:
    """Stage 3 ECL is discounted LGD x EAD over the recovery horizon."""
    from finstack_quant.statements_analytics import compute_ecl

    curve = [(0.0, 0.0), (1.0, 0.02)]
    fast = compute_ecl(1_000_000.0, curve, 0.45, 0.06, 1.0, stage="stage3", stage3_time_to_recovery_years=0.5)
    slow = compute_ecl(1_000_000.0, curve, 0.45, 0.06, 1.0, stage="stage3", stage3_time_to_recovery_years=3.0)
    # Longer time to recovery discounts the loss more heavily.
    assert slow < fast
    assert fast <= 0.45 * 1_000_000.0
