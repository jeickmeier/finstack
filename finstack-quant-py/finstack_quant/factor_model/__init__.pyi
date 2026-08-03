"""
Type stubs for ``finstack_quant.factor_model``.

Examples
--------
>>> from finstack_quant.factor_model import credit
>>> try:
...     credit.CreditFactorModel.from_json("{}")
... except ValueError as exc:
...     "missing field" in str(exc)
True
"""

from __future__ import annotations

from finstack_quant.factor_model import credit as credit

__all__ = [
    "credit",
]
