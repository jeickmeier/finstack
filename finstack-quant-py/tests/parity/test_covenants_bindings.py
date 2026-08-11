"""Focused parity checks for the covenant binding slice."""

from __future__ import annotations

import json
import pickle

import pytest

from finstack_quant import covenants


def _engine_json(spec: dict[str, object]) -> str:
    return json.dumps({"specs": [spec], "breach_history": [], "windows": [], "waivers": []})


def test_covenant_template_roundtrip_and_evaluate() -> None:
    specs_json = covenants.lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0)
    specs = json.loads(specs_json)

    canonical_engine = covenants.validate_covenant_engine_json(_engine_json(specs[0]))
    reports = covenants.evaluate_engine(
        canonical_engine,
        json.dumps({"debt_to_ebitda": 4.0}),
        "2026-03-31",
    )

    report = reports["max_debt_ebitda"]
    assert isinstance(report, covenants.CovenantReport)
    assert report.passed is True
    assert report.actual_value == pytest.approx(4.0)
    assert report.threshold == pytest.approx(5.0)
    assert report.headroom is not None
    assert report.headroom > 0.0
    assert "numeric_mode" in report.meta


def test_evaluate_engine_report_dataframe_and_pickle() -> None:
    spec = json.loads(covenants.lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0))[0]
    report = covenants.evaluate_engine(
        _engine_json(spec),
        json.dumps({"debt_to_ebitda": 5.5}),
        "2026-03-31",
    )["max_debt_ebitda"]

    assert report.passed is False

    frame = report.to_dataframe()
    assert list(frame.columns) == [
        "covenant_type",
        "covenant_id",
        "passed",
        "actual_value",
        "threshold",
        "headroom",
        "details",
    ]
    assert len(frame) == 1
    assert not bool(frame["passed"].iloc[0])

    revived = pickle.loads(pickle.dumps(report))  # noqa: S301
    assert revived.to_json() == report.to_json()


def test_covenant_report_json_roundtrip() -> None:
    report = {
        "covenant_type": "Debt/EBITDA <= 5.00x",
        "covenant_id": "max_debt_to_ebitda",
        "passed": False,
        "actual_value": 5.5,
        "threshold": 5.0,
        "details": "Exceeded",
        "headroom": -0.1,
    }

    canonical = json.loads(covenants.validate_covenant_report_json(json.dumps(report)))

    # Canonicalization stamps the policy envelope onto the report; every field
    # the caller supplied must survive it byte-for-byte.
    meta = canonical.pop("meta")
    assert canonical == report
    assert meta["numeric_mode"] == "f64"
    assert meta["rounding"]["mode"] == "bankers"

    typed = covenants.CovenantReport.from_json(json.dumps(report))
    assert typed.covenant_type == "Debt/EBITDA <= 5.00x"
    assert typed.covenant_id == "max_debt_to_ebitda"
    assert typed.passed is False
    assert typed.actual_value == pytest.approx(5.5)
    assert typed.threshold == pytest.approx(5.0)
    assert typed.details == "Exceeded"
    assert typed.headroom == pytest.approx(-0.1)


def test_covenant_engine_rejects_unknown_fields() -> None:
    engine = {
        "specs": [],
        "breach_history": [],
        "windows": [],
        "waviers": [],
    }

    with pytest.raises(ValueError, match="unknown field"):
        covenants.validate_covenant_engine_json(json.dumps(engine))


def test_threshold_schedule_validation_maps_to_value_error() -> None:
    spec = json.loads(covenants.lbo_standard_json(5.0, 1.5, 1.2, 10_000_000.0))[0]
    spec["threshold_schedule"] = [
        ["2025-01-01", 5.0],
        ["2025-01-01", 4.5],
    ]

    with pytest.raises(ValueError, match="duplicate date"):
        covenants.validate_covenant_spec_json(json.dumps(spec))
