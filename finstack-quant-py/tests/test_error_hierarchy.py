"""Tests for the shared ``FinstackError`` exception base and its compatibility guarantees.

``FinstackError`` lets a caller write ``except FinstackError`` instead of
enumerating every named exception the library defines. Because it was inserted
*above* pre-existing classes rather than replacing their bases, the tests below
also pin the backward-compatibility invariant that matters most: everything that
``except ValueError`` used to catch, it still catches.

``pyo3::create_exception!`` accepts a single base type, so two named exceptions
stay outside the tree for now (``CalibrationEnvelopeError``, which derives from
``RuntimeError``, and ``CholeskyError``, which is not yet reparented). The tests
here assert only their *stable* ancestry, so they keep passing once those are
folded in.
"""

from __future__ import annotations

import json
from typing import Any

import pytest

from finstack_quant.analytics import AnalyticsError
from finstack_quant.core.currency import Currency
from finstack_quant.core.math import linalg
from finstack_quant.core.money import Money
from finstack_quant.portfolio import (
    ContractLimitExceededError,
    ContractValidationError,
    FinstackError,
    FinstackFxError,
    FinstackOptimizationError,
    FinstackValuationError,
    MalformedContractSchemaError,
    MissingContractVersionError,
    PortfolioError,
    PortfolioResult,
    UnsupportedContractVersionError,
    portfolio_result_get_metric,
    portfolio_result_total_value,
)
from finstack_quant.valuations import CalibrationEnvelopeError

REPARENTED_ERRORS = [
    AnalyticsError,
    PortfolioError,
    FinstackValuationError,
    FinstackFxError,
    FinstackOptimizationError,
    ContractValidationError,
    UnsupportedContractVersionError,
    MissingContractVersionError,
    MalformedContractSchemaError,
    ContractLimitExceededError,
]


def _result_json(metrics: dict[str, Any] | None = None) -> str:
    """Build a minimal canonical ``PortfolioResult`` envelope as JSON."""
    aggregated = metrics if metrics is not None else {}
    return json.dumps({
        "schema_version": 1,
        "valuation": {
            "as_of": "2025-01-01",
            "position_values": {},
            "total_base_currency": {"amount": "0", "currency": "USD"},
            "by_entity": {},
        },
        "metrics": {"aggregated": aggregated, "by_position": {}},
        "meta": {
            "numeric_mode": "f64",
            "rounding": {
                "mode": "bankers",
                "ingest_scale_by_currency": {},
                "output_scale_by_currency": {},
                "tolerances": {"rate_epsilon": 1e-12, "generic_epsilon": 1e-10},
                "version": 1,
            },
        },
    })


class TestFinstackErrorBase:
    """``FinstackError`` exists and sits above the named exception families."""

    def test_is_exception_subclass(self) -> None:
        """The base is a real exception type, usable in an ``except`` clause."""
        assert isinstance(FinstackError, type)
        assert issubclass(FinstackError, Exception)

    def test_is_raisable_and_catchable(self) -> None:
        """A bare ``FinstackError`` can be raised and caught."""
        with pytest.raises(FinstackError, match="boom"):
            raise FinstackError("boom")

    def test_derives_from_value_error(self) -> None:
        """The base derives from ``ValueError`` so reparenting broke no caller.

        ``create_exception!`` allows only one base class, so ``FinstackError``
        could not be spliced in as a second base. Deriving it from ``ValueError``
        instead keeps ``ValueError`` in every subclass's MRO.
        """
        assert issubclass(FinstackError, ValueError)

    @pytest.mark.parametrize("error_type", REPARENTED_ERRORS, ids=lambda t: t.__name__)
    def test_named_errors_are_finstack_errors(self, error_type: type[Exception]) -> None:
        """Every reparented exception is caught by ``except FinstackError``."""
        assert issubclass(error_type, FinstackError)
        with pytest.raises(FinstackError):
            raise error_type("failure")

    def test_single_clause_catches_across_domains(self) -> None:
        """One ``except FinstackError`` spans the analytics and portfolio families."""
        caught: list[str] = []
        for error_type in (AnalyticsError, PortfolioError, ContractValidationError):
            try:
                raise error_type("failure")
            except FinstackError as exc:
                caught.append(type(exc).__name__)
        assert caught == ["AnalyticsError", "PortfolioError", "ContractValidationError"]


class TestBackwardCompatibility:
    """Pre-existing ``except`` clauses must keep behaving identically."""

    def test_analytics_error_still_caught_by_value_error(self) -> None:
        """``AnalyticsError`` documented itself as a ``ValueError``; it still is."""
        assert issubclass(AnalyticsError, ValueError)
        with pytest.raises(ValueError, match="invalid analytics input"):
            raise AnalyticsError("invalid analytics input")

    @pytest.mark.parametrize("error_type", REPARENTED_ERRORS, ids=lambda t: t.__name__)
    def test_reparented_errors_still_caught_by_value_error(self, error_type: type[Exception]) -> None:
        """Inserting ``FinstackError`` kept ``ValueError`` in every MRO."""
        assert issubclass(error_type, ValueError)
        with pytest.raises(ValueError, match="failure"):
            raise error_type("failure")

    def test_contract_subclasses_keep_their_intermediate_base(self) -> None:
        """The contract family kept its own two-level shape."""
        for error_type in (
            UnsupportedContractVersionError,
            MissingContractVersionError,
            MalformedContractSchemaError,
            ContractLimitExceededError,
        ):
            assert issubclass(error_type, ContractValidationError)

    def test_portfolio_subclasses_keep_their_intermediate_base(self) -> None:
        """The portfolio family kept its own two-level shape."""
        for error_type in (
            FinstackValuationError,
            FinstackFxError,
            FinstackOptimizationError,
        ):
            assert issubclass(error_type, PortfolioError)

    def test_currency_mismatch_still_raises_value_error(self) -> None:
        """Bare ``ValueError`` from core mapping helpers was not reclassified."""
        with pytest.raises(ValueError, match=r"(?i)currency|mismatch"):
            Money(100.0, Currency("USD")) + Money(50.0, Currency("EUR"))

    def test_unclassified_core_errors_stay_outside_the_tree(self) -> None:
        """Core still raises bare ``ValueError``, not a ``FinstackError`` subclass.

        Reclassifying these would silently change the exception type at every
        existing call site, so they were deliberately left alone.
        """
        with pytest.raises(ValueError, match=r"(?i)currency|mismatch") as exc_info:
            Money(100.0, Currency("USD")) + Money(50.0, Currency("EUR"))
        assert not isinstance(exc_info.value, FinstackError)


class TestErrorsOutsideTheTree:
    """Exceptions that could not join the tree keep their documented ancestry."""

    def test_calibration_envelope_error_is_runtime_error(self) -> None:
        """``CalibrationEnvelopeError`` derives from ``RuntimeError``.

        It cannot also derive from ``FinstackError`` (a ``ValueError``) without
        multiple inheritance, which ``create_exception!`` cannot express. This
        assertion holds either way, so it survives a later fix.
        """
        assert issubclass(CalibrationEnvelopeError, RuntimeError)

    def test_cholesky_error_is_value_error(self) -> None:
        """``CholeskyError`` derives from ``ValueError`` and can be reparented."""
        assert issubclass(linalg.CholeskyError, ValueError)


class TestResultAccessorEquivalence:
    """The free result accessors and the ``PortfolioResult`` members agree.

    These pin the equivalence a future deprecation of the free functions would
    rely on, and the one capability the members do not offer (JSON input), which
    is why the free functions were not deprecated in this change.
    """

    def test_total_value_matches_property(self) -> None:
        """``portfolio_result_total_value`` equals the ``total_value`` property."""
        result = PortfolioResult.from_json(_result_json())
        assert portfolio_result_total_value(result) == result.total_value

    def test_get_metric_matches_method(self) -> None:
        """``portfolio_result_get_metric`` equals ``PortfolioResult.get_metric``."""
        metrics = {"dv01": {"metric_id": "dv01", "total": 12.5, "by_entity": {}}}
        result = PortfolioResult.from_json(_result_json(metrics))
        assert portfolio_result_get_metric(result, "dv01") == result.get_metric("dv01")

    def test_missing_metric_returns_none_on_both_paths(self) -> None:
        """Both spellings return ``None`` for an absent metric; neither raises."""
        result = PortfolioResult.from_json(_result_json())
        assert portfolio_result_get_metric(result, "absent") is None
        assert result.get_metric("absent") is None

    def test_require_metric_is_not_equivalent_to_get_metric(self) -> None:
        """``require_metric`` raises where ``get_metric`` returns ``None``."""
        result = PortfolioResult.from_json(_result_json())
        with pytest.raises(KeyError):
            result.require_metric("absent")

    def test_free_functions_additionally_accept_json(self) -> None:
        """Only the free functions take a JSON string; the members need an object.

        This is the behavioural gap that blocks a like-for-like deprecation: the
        member replacement for the JSON path is two calls, not one.
        """
        payload = _result_json({"dv01": {"metric_id": "dv01", "total": 12.5, "by_entity": {}}})
        assert portfolio_result_total_value(payload) == 0.0
        assert portfolio_result_get_metric(payload, "dv01") == 12.5
