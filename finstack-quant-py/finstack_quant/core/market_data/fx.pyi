"""
FX types exposed by ``core.market_data.fx``.

Examples
--------
>>> import datetime
>>> from finstack_quant.core.market_data import FxMatrix
>>> matrix = FxMatrix()
>>> matrix.set_quote("EUR", "USD", 1.1)
>>> result = matrix.rate("EUR", "USD", datetime.date(2025, 1, 1))
>>> (result.rate, result.triangulated)
(1.1, False)

"""

from finstack_quant.core.market_data import FxConversionPolicy as FxConversionPolicy
from finstack_quant.core.market_data import FxMatrix as FxMatrix
from finstack_quant.core.market_data import FxRateResult as FxRateResult

__all__ = ["FxConversionPolicy", "FxRateResult", "FxMatrix"]
