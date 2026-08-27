"""Reusable analytical, Fourier, credit, correlation, and stochastic models.

Bindings for the ``finstack-quant-models`` Rust crate.

Examples:
--------
>>> from finstack_quant.models import bs_price
>>> round(bs_price(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, True), 4)
10.4506

"""

from finstack_quant.finstack_quant import models as _models
from finstack_quant.models import (
    correlation as correlation,
    credit as credit,
    factor as factor,
    monte_carlo as monte_carlo,
    rates as rates,
    volatility as volatility,
)

asian_option_price = _models.asian_option_price
barrier_call = _models.barrier_call
black76_implied_vol = _models.black76_implied_vol
bs_cos_price = _models.bs_cos_price
bs_greeks = _models.bs_greeks
bs_implied_vol = _models.bs_implied_vol
bs_price = _models.bs_price
lookback_option_price = _models.lookback_option_price
merton_jump_cos_price = _models.merton_jump_cos_price
quanto_option_price = _models.quanto_option_price
vanilla_expiry_payoff = _models.vanilla_expiry_payoff
vg_cos_price = _models.vg_cos_price

__all__: list[str] = [
    "asian_option_price",
    "barrier_call",
    "black76_implied_vol",
    "bs_cos_price",
    "bs_greeks",
    "bs_implied_vol",
    "bs_price",
    "correlation",
    "credit",
    "factor",
    "lookback_option_price",
    "merton_jump_cos_price",
    "monte_carlo",
    "quanto_option_price",
    "rates",
    "vanilla_expiry_payoff",
    "vg_cos_price",
    "volatility",
]
