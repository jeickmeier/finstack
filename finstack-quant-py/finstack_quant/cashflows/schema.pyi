"""
Compiled-in JSON Schemas for the cashflow component wire format.

The seven cashflow component schemas (amortization, coupon, default, fee,
prepayment, recovery, and schedule specifications) are embedded in the
extension module, so they always match the installed version and cannot drift
from the wheel that ships them. These are the documents that resolve the
``https://finstack_quant.dev/schemas/cashflow/1/...`` references appearing
inside instrument schemas.

Examples
--------
>>> import json
>>> from finstack_quant.cashflows import schema
>>> base = "https://finstack_quant.dev/schemas/cashflow/1/"
>>> json.loads(schema.resources()[base + "schedule_params.schema.json"])["title"]
'ScheduleParams'

"""

from __future__ import annotations

__all__ = [
    "resources",
]

def resources() -> dict[str, str]:
    """
    Return every embedded cashflow component schema, keyed by canonical URI.

    The keys are the published ``$id`` URIs used by ``$ref`` targets in
    instrument and cashflow payload schemas, so the mapping can be handed
    directly to a resolver-aware validator.

    Returns
    -------
    dict of str to str
        Mapping from canonical schema URI to pretty-printed JSON Schema text,
        one entry per embedded cashflow component schema.

    Raises
    ------
    ValueError
        If a compiled-in component schema is malformed or cannot be serialized
        back to JSON text.

    Examples
    --------
    >>> from finstack_quant.cashflows import schema
    >>> sorted(uri.rsplit("/", 1)[-1] for uri in schema.resources())[:2]
    ['amortization_spec.schema.json', 'coupon_specs.schema.json']

    """
