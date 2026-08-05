"""Shared, example-only notebook paths, markets, and helpers."""

from .instrument_fixtures import (
    acme_bond,
    instrument_envelope,
    instrument_envelope_json,
)
from .market import (
    DEMO_AS_OF,
    build_demo_market,
    usd_ois_2026,
    usd_ois_curve,
    usd_sofr_curve,
    usd_sofr_fixings,
)
from .notebook_helpers import banner, print_metrics, series
from .paths import NOTEBOOKS_ROOT, REPOSITORY_ROOT
from .synthetic import demo_pl_builder, demo_pl_model, random_walk_panel

__all__ = [
    "DEMO_AS_OF",
    "NOTEBOOKS_ROOT",
    "REPOSITORY_ROOT",
    "acme_bond",
    "banner",
    "build_demo_market",
    "demo_pl_builder",
    "demo_pl_model",
    "instrument_envelope",
    "instrument_envelope_json",
    "print_metrics",
    "random_walk_panel",
    "series",
    "usd_ois_2026",
    "usd_ois_curve",
    "usd_sofr_curve",
    "usd_sofr_fixings",
]
