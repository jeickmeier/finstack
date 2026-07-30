"""Host-level coverage for portfolio materialization bindings."""

from __future__ import annotations

import json
import sys
import threading
import time

import pytest

from finstack_quant.core.market_data import MarketContext
from finstack_quant.portfolio import (
    ContractLimitExceededError,
    ContractValidationError,
    InstrumentArtifactCache,
    MaterializationReport,
    Portfolio,
    value_portfolio,
)


def _empty_bundle() -> dict[str, object]:
    return {
        "schema": "finstack_quant.portfolio_materialization/1",
        "portfolio": {
            "id": "materialized-empty",
            "base_ccy": "USD",
            "as_of": "2025-01-01",
            "entities": {},
        },
        "instruments": [],
        "positions": [],
    }


def _deposit_bundle() -> dict[str, object]:
    return {
        "schema": "finstack_quant.portfolio_materialization/1",
        "portfolio": {
            "id": "materialized-deposit",
            "base_ccy": "USD",
            "as_of": "2025-01-01",
            "entities": {"entity": {"id": "entity", "name": None}},
        },
        "instruments": [
            {
                "artifact_id": "artifact-0",
                "envelope": {
                    "schema": "finstack_quant.instrument/1",
                    "instrument": {
                        "type": "deposit",
                        "spec": {
                            "id": "DEP-0",
                            "notional": {"amount": "1000000", "currency": "USD"},
                            "start_date": "2025-01-01",
                            "maturity": "2025-02-01",
                            "day_count": "Act360",
                            "discount_curve_id": "USD-OIS",
                            "attributes": {},
                        },
                    },
                },
            }
        ],
        "positions": [
            {
                "id": "position-0",
                "entity_id": "entity",
                "instrument_id": "DEP-0",
                "artifact_id": "artifact-0",
                "quantity": 1.0,
                "unit": "units",
            }
        ],
    }


@pytest.mark.parametrize(
    "payload",
    [
        json.dumps(_empty_bundle()),
        json.dumps(_empty_bundle()).encode(),
        bytearray(json.dumps(_empty_bundle()).encode()),
    ],
)
def test_from_materialization_accepts_string_and_bytes(payload: str | bytes | bytearray) -> None:
    portfolio, report = Portfolio.from_materialization(payload)

    assert portfolio.id == "materialized-empty"
    assert len(portfolio) == 0
    assert isinstance(report, MaterializationReport)
    assert report.positions == 0
    assert report.unique_instruments == 0
    assert report.dependencies == 0
    assert report.diagnostics == []
    assert report.truncated is False
    assert report.timing_available is True


def test_materialization_depth_scan_ignores_delimiters_inside_strings() -> None:
    bundle = _empty_bundle()
    portfolio = bundle["portfolio"]
    assert isinstance(portfolio, dict)
    portfolio["meta"] = {"quoted": ("[{" * 200) + '\\"escaped\\"]}' + ("}]" * 200)}

    loaded, report = Portfolio.from_materialization(json.dumps(bundle))

    assert loaded.id == "materialized-empty"
    assert report.diagnostics == []


def test_cache_is_reusable_and_wrappers_are_frozen() -> None:
    payload = json.dumps(_deposit_bundle()).encode()
    cache = InstrumentArtifactCache()

    _, cold = Portfolio.from_materialization(payload, cache)
    _, warm = Portfolio.from_materialization(payload, cache)

    assert cold.cache_hits == 0
    assert warm.cache_hits == 1
    assert cold.dependencies == 1
    assert warm.dependencies == 1
    assert cache.decode_count == 1
    assert len(cache) == 1
    with pytest.raises(AttributeError):
        cache.decode_count = 99  # type: ignore[misc]
    with pytest.raises(AttributeError):
        warm.cache_hits = 99  # type: ignore[misc]


def test_distinct_artifact_ids_with_identical_content_share_cache_entry() -> None:
    bundle = _deposit_bundle()
    instruments = bundle["instruments"]
    positions = bundle["positions"]
    assert isinstance(instruments, list)
    assert isinstance(positions, list)
    alias = json.loads(json.dumps(instruments[0]))
    alias["artifact_id"] = "artifact-alias"
    instruments.append(alias)
    alias_position = json.loads(json.dumps(positions[0]))
    alias_position["id"] = "position-alias"
    alias_position["artifact_id"] = "artifact-alias"
    positions.append(alias_position)
    cache = InstrumentArtifactCache()

    portfolio, report = Portfolio.from_materialization(json.dumps(bundle), cache)

    assert len(portfolio) == 2
    assert report.cache_hits == 1
    assert cache.decode_count == 1
    assert len(cache) == 1
    warning = next(
        diagnostic for diagnostic in report.diagnostics if diagnostic["code"] == "portfolio/duplicate-artifact-content"
    )
    assert warning["severity"] == "warning"
    assert warning["pointer"] == "/instruments/1/envelope"
    assert warning["revision_id"] == "artifact-alias"
    assert warning["artifact_hash"].startswith("sha256:")


def test_materialized_handle_can_be_valued_twice_without_reparse() -> None:
    portfolio, _ = Portfolio.from_materialization(json.dumps(_empty_bundle()))
    market = MarketContext()

    first = value_portfolio(portfolio, market)
    second = value_portfolio(portfolio, market)

    assert first == second


def test_from_materialization_releases_gil_for_parse_and_build() -> None:
    bundle = _empty_bundle()
    portfolio = bundle["portfolio"]
    assert isinstance(portfolio, dict)
    portfolio["entities"] = {f"entity-{index}": {"id": f"entity-{index}"} for index in range(20_000)}
    payload = json.dumps(bundle)
    started = threading.Event()
    stop = threading.Event()
    progress = [0]

    def heartbeat() -> None:
        started.set()
        while not stop.is_set():
            progress[0] += 1
            time.sleep(0)

    worker = threading.Thread(target=heartbeat, daemon=True)
    worker.start()
    assert started.wait(timeout=1.0)
    old_interval = sys.getswitchinterval()
    sys.setswitchinterval(1.0)
    before = progress[0]
    try:
        built, _ = Portfolio.from_materialization(payload)
        after = progress[0]
    finally:
        sys.setswitchinterval(old_interval)
        stop.set()
        worker.join(timeout=1.0)

    assert built.id == "materialized-empty"
    assert after > before


def test_malformed_bundle_raises_structured_contract_error() -> None:
    with pytest.raises(ContractValidationError) as caught:
        Portfolio.from_materialization(b'{"schema":')

    assert not isinstance(caught.value, ContractLimitExceededError)
    assert issubclass(ContractValidationError, ValueError)
    assert caught.value.report == [
        {
            "code": "contract/parse-error",
            "phase": "parse",
            "severity": "error",
            "pointer": None,
            "message": caught.value.report[0]["message"],
            "contract": None,
            "expected_version": None,
            "actual_version": None,
            "artifact_hash": None,
            "revision_id": None,
            "instrument_id": None,
            "position_id": None,
        }
    ]


def test_materialization_max_input_size_raises_typed_limit_error() -> None:
    payload = b" " * (64 * 1024 * 1024 + 1)

    with pytest.raises(ContractLimitExceededError, match="bytes"):
        Portfolio.from_materialization(payload)


def test_materialization_max_artifact_count_raises_typed_limit_error() -> None:
    bundle = _empty_bundle()
    bundle["instruments"] = [{"artifact_id": f"artifact-{index}", "envelope": {}} for index in range(100_001)]
    payload = json.dumps(bundle, separators=(",", ":"))
    assert len(payload) < 64 * 1024 * 1024

    with pytest.raises(ContractLimitExceededError, match="artifacts"):
        Portfolio.from_materialization(payload)


def test_materialization_rejects_other_python_types() -> None:
    with pytest.raises(TypeError, match="bytes, bytearray, or str"):
        Portfolio.from_materialization(memoryview(b"{}"))
