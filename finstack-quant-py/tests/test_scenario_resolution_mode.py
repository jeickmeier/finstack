from __future__ import annotations

import json

import pytest

from finstack_quant.scenarios import build_scenario_spec, compose_scenarios


def _time_roll_scenario(
    scenario_id: str,
    period: str,
    priority: int,
) -> dict[str, object]:
    operation = {
        "kind": "time_roll_forward",
        "period": period,
        "apply_shocks": True,
        "roll_mode": "business_days",
    }
    return json.loads(
        build_scenario_spec(
            scenario_id,
            json.dumps([operation]),
            priority=priority,
            resolution_mode="cumulative",
        )
    )


def test_build_scenario_spec_runtime_doc_describes_resolution_contract() -> None:
    doc = build_scenario_spec.__doc__
    assert doc
    assert "resolution_mode" in doc
    assert "most_specific_wins" in doc
    assert "cumulative" in doc
    assert "ValueError" in doc
    assert "Returns" in doc


def test_build_scenario_spec_exposes_resolution_mode() -> None:
    spec = json.loads(
        build_scenario_spec(
            "cumulative-shock",
            "[]",
            resolution_mode="cumulative",
        )
    )
    assert spec["resolution_mode"] == "cumulative"


def test_compose_scenarios_rejects_duplicate_time_rolls() -> None:
    specs = [
        _time_roll_scenario("roll_1m", "1M", 0),
        _time_roll_scenario("roll_3m", "3M", 1),
    ]

    with pytest.raises(ValueError, match="TimeRollForward"):
        compose_scenarios(json.dumps(specs))
