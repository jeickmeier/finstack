"""Cross-cutting ergonomics guarantees for the Python bindings.

These lock in the quant-facing contracts that are easy to regress because no
single domain owns them: the package advertises its version, builders chain,
and date-valued parameters accept both ISO strings and `datetime.date`.
"""

from __future__ import annotations

import datetime
import re
from typing import Any

import pytest

import finstack_quant as fq
from finstack_quant.statements import Evaluator, ModelBuilder


class TestVersion:
    """`finstack_quant.__version__` lets a notebook record what it ran against."""

    def test_version_is_exposed(self) -> None:
        assert isinstance(fq.__version__, str)
        assert fq.__version__

    def test_version_is_pep440_release(self) -> None:
        """A parseable version is what makes it useful in a provenance stamp."""
        assert re.match(r"^\d+\.\d+\.\d+", fq.__version__), fq.__version__

    def test_version_matches_extension(self) -> None:
        """The pure-Python re-export must not drift from the compiled module."""
        from finstack_quant.finstack_quant import __version__ as ext_version

        assert fq.__version__ == ext_version

    def test_version_is_not_a_lazy_submodule(self) -> None:
        """`__version__` is in `__all__` but must never route through `__getattr__`."""
        assert "__version__" in fq.__all__
        # A lazily-imported name would be a module, not a string.
        assert not hasattr(fq.__version__, "__name__")


class TestModelBuilderChaining:
    """Configuration calls return the builder, so they compose."""

    def test_chained_construction_builds_a_model(self) -> None:
        model = (
            ModelBuilder("chained")
            .periods("2025Q1..Q3")
            .value("revenue", [("2025Q1", 100.0), ("2025Q2", 110.0), ("2025Q3", 121.0)])
            .compute("cogs", "revenue * 0.4")
            .compute("gross_profit", "revenue - cogs")
            .build()
        )
        assert set(model.node_ids()) == {"revenue", "cogs", "gross_profit"}

    def test_chaining_returns_the_same_object(self) -> None:
        """Chaining must mutate in place rather than fork a copy.

        If it forked, the statement-per-line style would silently diverge from
        the chained one.
        """
        builder = ModelBuilder("identity")
        assert builder.periods("2025Q1..Q1") is builder
        assert builder.value("revenue", [("2025Q1", 1.0)]) is builder

    def test_statement_style_still_works(self) -> None:
        """The pre-existing call style must keep working unchanged."""
        builder = ModelBuilder("statements")
        builder.periods("2025Q1..Q1")
        builder.value("revenue", [("2025Q1", 100.0)])
        builder.compute("half", "revenue * 0.5")
        assert builder.build().node_count == 2

    def test_chained_and_statement_styles_agree(self) -> None:
        chained = ModelBuilder("m").periods("2025Q1..Q2").value("revenue", [("2025Q1", 10.0), ("2025Q2", 20.0)]).build()
        stepwise = ModelBuilder("m")
        stepwise.periods("2025Q1..Q2")
        stepwise.value("revenue", [("2025Q1", 10.0), ("2025Q2", 20.0)])
        assert chained.node_ids() == stepwise.build().node_ids()

    def test_mixed_node_builder_chains(self) -> None:
        builder = ModelBuilder("mixed")
        builder.periods("2025Q1..Q1")
        builder.value("revenue", [("2025Q1", 100.0)])
        model = builder.mixed("margin").formula("revenue * 0.5").name("Margin").build().build()
        assert "margin" in model.node_ids()

    def test_build_is_terminal(self) -> None:
        """`build()` consumes the builder; a later call must say so clearly."""
        builder = ModelBuilder("terminal")
        builder.periods("2025Q1..Q1")
        builder.build()
        with pytest.raises(ValueError, match="no longer usable"):
            builder.value("revenue", [("2025Q1", 1.0)])

    def test_failed_call_does_not_brick_the_builder(self) -> None:
        """A rejected formula must leave the builder usable.

        The pre-validation that guarantees this is easy to lose when the
        method signatures change.
        """
        builder = ModelBuilder("robust")
        builder.periods("2025Q1..Q1")
        builder.value("revenue", [("2025Q1", 100.0)])
        with pytest.raises(ValueError, match="parse error"):
            builder.compute("bad", "revenue -* 2")
        assert builder.compute("good", "revenue * 2") is builder

    def test_chained_model_evaluates(self) -> None:
        """End-to-end: the chained builder feeds the evaluator and reaches pandas."""
        model = (
            ModelBuilder("e2e")
            .periods("2025Q1..Q2")
            .value("revenue", [("2025Q1", 100.0), ("2025Q2", 110.0)])
            .compute("cogs", "revenue * 0.4")
            .build()
        )
        result = Evaluator().evaluate(model)
        frame = result.to_pandas_long()
        assert set(frame["node_id"]) == {"revenue", "cogs"}


class TestDateArguments:
    """Date-valued parameters accept an ISO string or a `datetime.date`."""

    @staticmethod
    def _market() -> str:
        import json

        return json.dumps({
            "schema_version": 1,
            "curves": [
                {
                    "type": "discount",
                    "id": "USD-OIS",
                    "base": "2024-01-01",
                    "day_count": "act_360",
                    "knot_points": [[0.0, 1.0], [5.0, 0.90], [10.0, 0.80]],
                    "interp_style": "monotone_convex",
                    "extrapolation": "flat_forward",
                    "min_forward_rate": None,
                    "allow_non_monotonic": False,
                    "min_forward_tenor": 1e-6,
                }
            ],
            "fx": None,
            "surfaces": [],
            "prices": {},
            "series": [],
            "inflation_indices": [],
            "dividends": [],
            "credit_indices": [],
            "fx_delta_vol_surfaces": [],
            "vol_cubes": [],
            "collateral": {},
            "hierarchy": None,
        })

    @staticmethod
    def _bond() -> Any:
        from finstack_quant.core.currency import Currency
        from finstack_quant.core.money import Money
        from finstack_quant.core.types import Rate
        from finstack_quant.valuations.instruments import Bond

        return Bond.fixed(
            "BOND-1",
            Money(1_000_000.0, Currency("USD")),
            Rate(0.05),
            datetime.date(2024, 1, 1),
            datetime.date(2034, 1, 1),
            "USD-OIS",
        )

    def test_as_of_accepts_string_and_date_identically(self) -> None:
        """The two spellings must price to the same number, not merely both work."""
        from finstack_quant.valuations.instruments import price_instrument

        bond, market = self._bond(), self._market()
        from_str = price_instrument(bond.to_json(), market, "2024-01-01")
        from_date = price_instrument(bond.to_json(), market, datetime.date(2024, 1, 1))
        assert from_str.price == from_date.price

    def test_as_of_rejects_non_date_with_a_useful_message(self) -> None:
        from finstack_quant.valuations.instruments import price_instrument

        bond, market = self._bond(), self._market()
        with pytest.raises((TypeError, ValueError)) as excinfo:
            price_instrument(bond.to_json(), market, 20240101)
        assert "date" in str(excinfo.value).lower()

    def test_as_of_rejects_malformed_iso_string(self) -> None:
        from finstack_quant.valuations.instruments import price_instrument

        bond, market = self._bond(), self._market()
        with pytest.raises(ValueError, match=r"(?i)date|parse|input"):
            price_instrument(bond.to_json(), market, "01/01/2024")
