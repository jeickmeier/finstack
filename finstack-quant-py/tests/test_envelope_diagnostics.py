"""Tests for Phase 4 envelope diagnostics surface."""

from __future__ import annotations

from collections.abc import Callable
import json

import pytest

from finstack_quant.valuations import (
    CalibrationEnvelopeError,
    calibrate,
    dependency_graph_json,
    dry_run,
    validate_calibration_json,
)


def _empty_envelope() -> dict:
    return {
        "schema": "finstack_quant.calibration/1",
        "plan": {
            "id": "smoke",
            "description": None,
            "quote_sets": {},
            "steps": [],
            "settings": {},
        },
    }


def test_dry_run_returns_json_report() -> None:
    report = json.loads(dry_run(json.dumps(_empty_envelope())))
    assert report["errors"] == []
    assert "dependency_graph" in report


def test_dependency_graph_json_well_formed() -> None:
    graph = json.loads(dependency_graph_json(json.dumps(_empty_envelope())))
    assert "initial_ids" in graph
    assert graph["nodes"] == []


def test_dry_run_surfaces_undefined_quote_set_with_suggestion() -> None:
    envelope = _empty_envelope()
    envelope["plan"]["quote_sets"] = {"usd_quotes": []}
    envelope["plan"]["steps"] = [
        {
            "id": "discount_step",
            "quote_set": "usd_quotess",
            "kind": "discount",
            "curve_id": "USD-OIS",
            "currency": "USD",
            "base_date": "2026-05-08",
        }
    ]
    report = json.loads(dry_run(json.dumps(envelope)))
    undef = next(
        (e for e in report["errors"] if e["kind"] == "undefined_quote_set"),
        None,
    )
    assert undef is not None, report["errors"]
    assert undef["ref_name"] == "usd_quotess"
    assert undef["suggestion"] == "usd_quotes"


@pytest.mark.parametrize("operation", [validate_calibration_json, calibrate])
def test_calibration_entry_points_enforce_semantic_validation(
    operation: Callable[[str], str],
) -> None:
    envelope = _empty_envelope()
    envelope["plan"]["steps"] = [
        {
            "id": "discount_step",
            "quote_set": "missing_quotes",
            "kind": "discount",
            "curve_id": "USD-OIS",
            "currency": "USD",
            "base_date": "2026-05-08",
        }
    ]

    with pytest.raises(CalibrationEnvelopeError) as excinfo:
        operation(json.dumps(envelope))

    assert excinfo.value.kind == "undefined_quote_set"
    details = json.loads(excinfo.value.details)
    assert details["ref_name"] == "missing_quotes"


def test_calibration_envelope_error_inherits_runtime_error() -> None:
    """Backwards-compat: existing `except RuntimeError` callers still catch it."""
    assert issubclass(CalibrationEnvelopeError, RuntimeError)


def test_dry_run_raises_typed_exception_on_bad_json() -> None:
    with pytest.raises(CalibrationEnvelopeError) as excinfo:
        dry_run("not json at all")
    exc = excinfo.value
    assert exc.kind == "json_parse"
    assert exc.step_id is None
    # `details` is a JSON string carrying the structured payload.
    payload = json.loads(exc.details)
    assert payload["kind"] == "json_parse"


def test_calibrate_raises_typed_exception_on_bad_json() -> None:
    with pytest.raises(CalibrationEnvelopeError) as excinfo:
        calibrate("{ malformed")
    assert excinfo.value.kind == "json_parse"


def test_runtime_error_handler_catches_calibration_envelope_error() -> None:
    """Existing pre-Phase-4 `except RuntimeError` callers continue to work.

    Catching as the broader ``RuntimeError`` parent must still produce the
    typed subclass so legacy code paths that introspect via ``isinstance``
    keep functioning.
    """
    with pytest.raises(RuntimeError) as excinfo:
        dry_run("garbage")
    assert isinstance(excinfo.value, CalibrationEnvelopeError)
