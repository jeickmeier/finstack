"""Market conventions and listed-market coverage metadata.

``ConventionRegistry`` mirrors the Rust ``ConventionRegistry::try_global()``:
read-only lookups of the embedded rate-index, CDS, swaption, inflation-swap,
IR-future and cross-currency convention tables.

Examples:
--------
>>> from finstack_quant.valuations.market import ConventionRegistry, listed_product_catalog
>>> any(row["exchange"] == "eurex" for row in listed_product_catalog())
True
>>> ConventionRegistry().require_rate_index("USD-SOFR").currency
'USD'

"""

from finstack_quant.finstack_quant import valuations as _valuations

_market = _valuations.market

CdsConventionSpec = _market.CdsConventionSpec
ConventionRegistry = _market.ConventionRegistry
InflationSwapConventions = _market.InflationSwapConventions
IrFutureConventions = _market.IrFutureConventions
RateIndexConventions = _market.RateIndexConventions
SwaptionConventions = _market.SwaptionConventions
XccyConventions = _market.XccyConventions
listed_product_catalog = _market.listed_product_catalog

__all__ = [
    "CdsConventionSpec",
    "ConventionRegistry",
    "InflationSwapConventions",
    "IrFutureConventions",
    "RateIndexConventions",
    "SwaptionConventions",
    "XccyConventions",
    "listed_product_catalog",
]
