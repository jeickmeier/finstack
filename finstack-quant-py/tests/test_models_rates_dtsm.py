"""Behavioral coverage for the models-owned DTSM bindings."""

import importlib

import pytest

from finstack_quant.models.rates.dtsm import nelson_siegel_yields


def test_nelson_siegel_yields_preserves_numerical_golden() -> None:
    """The moved host API must preserve its existing Nelson-Siegel values."""
    actual = nelson_siegel_yields(0.7308, (0.03, -0.01, 0.005), [1.0, 5.0, 10.0])
    assert actual == pytest.approx(
        [0.024045061287046227, 0.02853762303631049, 0.02931292600971276],
        abs=1e-15,
    )


def test_nelson_siegel_yields_maps_validation_to_value_error() -> None:
    """Invalid model inputs should retain the documented Python error type."""
    with pytest.raises(ValueError, match="lambda"):
        nelson_siegel_yields(0.0, (0.03, -0.01, 0.005), [1.0])


def test_old_core_dtsm_namespace_is_removed() -> None:
    """The clean break must leave no importable core DTSM module."""
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("finstack_quant.core.market_data.dtsm")
