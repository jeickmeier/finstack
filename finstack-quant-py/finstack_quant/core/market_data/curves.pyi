"""
Curve and surface types exposed by ``core.market_data.curves``.

Examples
--------
>>> import datetime
>>> from finstack_quant.core.market_data import DiscountCurve
>>> round(DiscountCurve.flat("USD-OIS", datetime.date(2025, 1, 1), 0.05).df(1.0), 6)
0.951229

"""

from finstack_quant.core.market_data import BaseCorrelationCurve as BaseCorrelationCurve
from finstack_quant.core.market_data import CreditIndexData as CreditIndexData
from finstack_quant.core.market_data import DiscountCurve as DiscountCurve
from finstack_quant.core.market_data import ForwardCurve as ForwardCurve
from finstack_quant.core.market_data import FxDeltaVolSurface as FxDeltaVolSurface
from finstack_quant.core.market_data import HazardCurve as HazardCurve
from finstack_quant.core.market_data import InflationCurve as InflationCurve
from finstack_quant.core.market_data import PriceCurve as PriceCurve
from finstack_quant.core.market_data import VolCube as VolCube
from finstack_quant.core.market_data import VolSurface as VolSurface
from finstack_quant.core.market_data import VolatilityIndexCurve as VolatilityIndexCurve

__all__ = [
    "BaseCorrelationCurve",
    "CreditIndexData",
    "DiscountCurve",
    "ForwardCurve",
    "FxDeltaVolSurface",
    "HazardCurve",
    "InflationCurve",
    "PriceCurve",
    "VolCube",
    "VolSurface",
    "VolatilityIndexCurve",
]
