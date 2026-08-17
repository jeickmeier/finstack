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
from finstack_quant.core.market_data import FxPairConvention as FxPairConvention
from finstack_quant.core.market_data import FxQuoteConvention as FxQuoteConvention
from finstack_quant.core.market_data import FxRateResult as FxRateResult
from finstack_quant.core.market_data import fx_market_pair as fx_market_pair
from finstack_quant.core.market_data import fx_pair_convention as fx_pair_convention
from finstack_quant.core.market_data import fx_pip_size as fx_pip_size
from finstack_quant.core.market_data import invert_fx_rate as invert_fx_rate

__all__ = [
    "FxConversionPolicy",
    "FxMatrix",
    "FxPairConvention",
    "FxQuoteConvention",
    "FxRateResult",
    "fx_market_pair",
    "fx_pair_convention",
    "fx_pip_size",
    "invert_fx_rate",
]
