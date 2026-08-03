"""
Market context type exposed by ``core.market_data.context``.

Examples
--------
>>> import datetime
>>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
>>> context = MarketContext().insert(DiscountCurve.flat("USD-OIS", datetime.date(2025, 1, 1), 0.05))
>>> MarketContext.from_json(context.to_json()).get_discount("USD-OIS").id
'USD-OIS'

"""

from finstack_quant.core.market_data import MarketContext as MarketContext

__all__ = ["MarketContext"]
