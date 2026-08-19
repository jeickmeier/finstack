"""Listed-market coverage metadata and exchange routing.

Examples
--------
>>> from finstack_quant.valuations.market import listed_product_catalog
>>> any(row["exchange"] == "eurex" for row in listed_product_catalog())
True
"""

from __future__ import annotations

from typing import Literal

__all__ = ["listed_product_catalog"]

def listed_product_catalog(
    exchange: Literal["cme", "eurex", "montreal", "sgx"] | None = None,
) -> list[dict[str, object]]:
    """Return the maintained liquid listed-derivatives coverage catalog.

    Parameters
    ----------
    exchange : {"cme", "eurex", "montreal", "sgx"} | None, optional
        Exact venue filter. ``None`` returns all four exchanges.

    Returns
    -------
    list[dict[str, object]]
        Product-family rows containing the canonical instrument type, covered
        exchange features, source URL, and any residual modelling gap.

    Raises
    ------
    ValueError
        If ``exchange`` is not one of the accepted canonical venue names, or
        if the embedded listed-product sidecar is invalid.

    Examples
    --------
    >>> from finstack_quant.valuations.market import listed_product_catalog
    >>> rows = listed_product_catalog("cme")
    >>> all(row["exchange"] == "cme" for row in rows)
    True
    """
    ...
