"""Product-independent liquidity risk and market-impact models.

Examples:
--------
>>> from finstack_quant.models.liquidity import days_to_liquidate
>>> days_to_liquidate(1_000_000, 250_000, 0.20)
20.0
"""

from finstack_quant.finstack_quant import models as _models

roll_effective_spread = _models.liquidity.roll_effective_spread
amihud_illiquidity = _models.liquidity.amihud_illiquidity
days_to_liquidate = _models.liquidity.days_to_liquidate
liquidity_tier = _models.liquidity.liquidity_tier
lvar_bangia = _models.liquidity.lvar_bangia
almgren_chriss_impact = _models.liquidity.almgren_chriss_impact
kyle_lambda = _models.liquidity.kyle_lambda

__all__: list[str] = [
    "almgren_chriss_impact",
    "amihud_illiquidity",
    "days_to_liquidate",
    "kyle_lambda",
    "liquidity_tier",
    "lvar_bangia",
    "roll_effective_spread",
]
