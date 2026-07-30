from __future__ import annotations

import json

from finstack_quant.scenarios import build_scenario_spec


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
