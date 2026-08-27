"""
Type stubs for the compiled ``finstack_quant.finstack_quant`` extension module.

These stubs allow static type checkers to resolve the extension namespace in
environments where the PyO3 module has not been built yet, such as the CI lint
job.

Examples
--------
>>> from finstack_quant import core
>>> core.dates.Tenor.parse("3M").months
3

"""

from __future__ import annotations

from typing import Any

analytics: Any
attribution: Any
cashflows: Any
core: Any
covenants: Any
features: Any
margin: Any
models: Any
portfolio: Any
scenarios: Any
schema: Any
statements: Any
statements_analytics: Any
valuations: Any

__version__: str
__all__: list[str]
