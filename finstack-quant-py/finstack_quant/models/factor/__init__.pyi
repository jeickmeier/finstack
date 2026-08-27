"""
Type stubs for ``finstack_quant.models.factor``.

Examples
--------
>>> from finstack_quant.models.factor import credit
>>> try:
...     credit.CreditFactorModel.from_json("{}")
... except ValueError as exc:
...     "missing field" in str(exc)
True
"""

from __future__ import annotations

from finstack_quant.models.factor import credit as credit
from finstack_quant.models.factor import risk as risk
from finstack_quant.models.factor import schema as schema

__all__ = [
    "credit",
    "risk",
    "schema",
]
