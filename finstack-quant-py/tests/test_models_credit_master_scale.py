"""PD master-scale bindings."""

from __future__ import annotations

import pytest

from finstack_quant.models.credit import pd


def test_library_scales_load_and_are_ordered() -> None:
    scale = pd.MasterScale.sp_assumptions()
    assert scale.n_grades == len(scale) == 8

    uppers = [g.upper_pd for g in scale.grades]
    assert uppers == sorted(uppers), "bands must be ascending in PD"
    assert uppers[-1] == pytest.approx(1.0)

    for grade in scale.grades:
        assert grade.central_pd <= grade.upper_pd

    assert pd.MasterScale.moodys_assumptions().n_grades > 0


def test_map_pd_notches_to_the_owning_band() -> None:
    scale = pd.MasterScale.sp_assumptions()

    result = scale.map_pd(0.003)
    assert result.grade == "BBB"
    assert result.input_pd == pytest.approx(0.003)
    assert result.central_pd == pytest.approx(0.002)
    assert scale.grades[result.grade_index].label == "BBB"

    # A PD exactly on a band's upper bound belongs to that band, not the next.
    boundary = scale.grades[3].upper_pd
    assert scale.map_pd(boundary).grade == scale.grades[3].label


def test_map_pd_rejects_non_finite() -> None:
    scale = pd.MasterScale.sp_assumptions()
    for bad in (float("nan"), float("inf"), float("-inf")):
        with pytest.raises(ValueError, match="non-finite"):
            scale.map_pd(bad)


def test_map_pd_clamps_out_of_range_probabilities() -> None:
    """Out-of-range PDs clamp to the end bands rather than raising.

    This is deliberate — core pins it in `pd_exceeds_all_grades` — but it means
    an invalid probability produces a confident grade: a sign error maps to the
    best grade and a percent/decimal mix-up (5.0 for "5%") to the worst. Callers
    that need input validation must do it before calling.
    """
    scale = pd.MasterScale.sp_assumptions()

    assert scale.map_pd(-0.1).grade == scale.grades[0].label
    assert scale.map_pd(1.5).grade == scale.grades[-1].label
    # The input PD is preserved verbatim, so the clamp is at least visible.
    assert scale.map_pd(1.5).input_pd == pytest.approx(1.5)


def test_custom_scale_round_trips() -> None:
    grades = [
        pd.MasterScaleGrade("GOOD", 0.01, 0.005),
        pd.MasterScaleGrade("BAD", 1.0, 0.30),
    ]
    scale = pd.MasterScale(grades)
    assert scale.n_grades == 2
    assert scale.map_pd(0.002).grade == "GOOD"
    assert scale.map_pd(0.5).grade == "BAD"
