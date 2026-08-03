"""
Python bindings for the corresponding finstack-quant Rust API.

Examples
--------
>>> from finstack_quant.valuations.models import credit
>>> round(credit.MertonModel(100.0, 0.25, 80.0, 0.05).default_probability(1.0), 6)
0.166629

"""

from __future__ import annotations

from finstack_quant.valuations.models import credit as credit

__all__ = ["credit"]
