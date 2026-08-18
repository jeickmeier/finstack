from __future__ import annotations

import pytest

from finstack_quant.scenarios import (
    CurveKind,
    OperationSpec,
    ScenarioSpec,
    TemplateMetadata,
    build_from_template,
    build_scenario_spec,
    compose_scenarios,
    list_builtin_template_metadata,
    parse_scenario_spec,
)


def _time_roll_scenario(
    scenario_id: str,
    period: str,
    priority: int,
) -> ScenarioSpec:
    operation = OperationSpec.time_roll_forward(period)
    return build_scenario_spec(
        scenario_id,
        [operation],
        priority=priority,
        resolution_mode="cumulative",
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
    spec = build_scenario_spec(
        "cumulative-shock",
        [],
        resolution_mode="cumulative",
    )
    assert spec.resolution_mode == "cumulative"


def test_build_scenario_spec_preserves_typed_operations() -> None:
    operation = OperationSpec.curve_parallel_bp(CurveKind.discount(), "USD-OIS", 25.0)
    spec = build_scenario_spec("rates", [operation])

    assert len(spec.operations) == 1
    assert spec.operations[0].kind == "curve_parallel_bp"
    assert '"curve_kind":"discount"' in spec.to_json()


def test_scenario_spec_json_roundtrip_is_typed() -> None:
    built = build_scenario_spec("typed", [], name="Typed")
    parsed = parse_scenario_spec(built.to_json())

    assert isinstance(built, ScenarioSpec)
    assert isinstance(parsed, ScenarioSpec)
    assert parsed.id == "typed"
    assert parsed.name == "Typed"
    assert parsed.operations == []


def test_template_helpers_return_typed_values() -> None:
    metadata = list_builtin_template_metadata()
    built = build_from_template(metadata[0].id)

    assert metadata
    assert isinstance(metadata[0], TemplateMetadata)
    assert isinstance(built, ScenarioSpec)
    assert built.id == metadata[0].id


def test_compose_scenarios_rejects_duplicate_time_rolls() -> None:
    specs = [
        _time_roll_scenario("roll_1m", "1M", 0),
        _time_roll_scenario("roll_3m", "3M", 1),
    ]

    with pytest.raises(ValueError, match="TimeRollForward"):
        compose_scenarios(specs)
