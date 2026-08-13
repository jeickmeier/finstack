"""Pricing model wrappers for ``finstack_quant.valuations``.

Examples:
--------
>>> from finstack_quant.valuations.models import credit
>>> round(credit.MertonModel(100.0, 0.25, 80.0, 0.05).default_probability(1.0), 6)
0.166629

"""

from finstack_quant.valuations.models import credit as credit

__all__: list[str] = ["credit"]
