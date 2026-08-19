"""Listed-market coverage metadata.

Examples:
--------
>>> from finstack_quant.valuations.market import listed_product_catalog
>>> any(row["exchange"] == "eurex" for row in listed_product_catalog())
True

"""

from finstack_quant.finstack_quant import valuations as _valuations

listed_product_catalog = _valuations.market.listed_product_catalog

__all__ = ["listed_product_catalog"]
