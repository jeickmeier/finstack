"""
Scalar market types exposed by ``core.market_data.scalars``.

Examples
--------
>>> import datetime
>>> from finstack_quant.core.market_data import ScalarTimeSeries
>>> series = ScalarTimeSeries("SOFR", [(datetime.date(2025, 1, 1), 0.03), (datetime.date(2025, 1, 3), 0.04)])
>>> ScalarTimeSeries.from_json(series.to_json()).value_on(datetime.date(2025, 1, 1))
0.03

"""

from finstack_quant.core.market_data import InflationIndex as InflationIndex
from finstack_quant.core.market_data import ScalarTimeSeries as ScalarTimeSeries

__all__ = [
    "InflationIndex",
    "ScalarTimeSeries",
]
