"""
Type stubs for ``finstack_quant.core.table``.

Arrow interchange surface for finstack-quant tabular results, from the
``finstack-quant-core``/``finstack-quant-arrow`` Rust crates. Exposes
:class:`ArrowTable`, a zero-copy Arrow ``RecordBatch`` wrapper implementing
the Arrow PyCapsule C-stream protocol so ``pyarrow``, ``polars``, ``duckdb``,
and ``pandas`` can consume finstack tabular results directly.

Examples
--------
>>> import json
>>> from finstack_quant.core.market_data import MarketContext
>>> from finstack_quant.portfolio import Portfolio, value_portfolio
>>> bundle = {
...     "schema": "finstack_quant.portfolio_materialization/1",
...     "portfolio": {"id": "empty", "base_currency": "USD", "as_of": "2025-01-01", "entities": {}},
...     "instruments": [],
...     "positions": [],
... }
>>> portfolio, _ = Portfolio.from_materialization(json.dumps(bundle))
>>> table = value_portfolio(portfolio, MarketContext()).to_arrow_positions()
>>> (table.num_rows, table.num_columns, table.column_names())
(0, 6, ['position_id', 'entity_id', 'value_native', 'value_base', 'currency_native', 'currency_base'])

"""

from __future__ import annotations

from typing import Any

__all__ = ["ArrowTable"]

class ArrowTable:
    """
    Arrow ``RecordBatch`` wrapper exposing the Arrow PyCapsule C-stream protocol.

    Produced internally from a core ``TableEnvelope`` by finstack-quant
    ``to_arrow_*`` producer methods. Has no Python-facing constructor;
    consume it via the standard Arrow PyCapsule interface, for example
    ``pyarrow.table(arrow_table)`` or ``polars.DataFrame(arrow_table)``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.core.market_data import MarketContext
    >>> from finstack_quant.portfolio import Portfolio, value_portfolio
    >>> bundle = {
    ...     "schema": "finstack_quant.portfolio_materialization/1",
    ...     "portfolio": {"id": "empty", "base_currency": "USD", "as_of": "2025-01-01", "entities": {}},
    ...     "instruments": [],
    ...     "positions": [],
    ... }
    >>> portfolio, _ = Portfolio.from_materialization(json.dumps(bundle))
    >>> table = value_portfolio(portfolio, MarketContext()).to_arrow_positions()
    >>> (table.num_rows, table.num_columns, table.column_names())
    (0, 6, ['position_id', 'entity_id', 'value_native', 'value_base', 'currency_native', 'currency_base'])

    """

    @property
    def num_rows(self) -> int:
        """
        Number of rows in the table.

        Returns
        -------
        int
            The row count of the underlying Arrow ``RecordBatch``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def num_columns(self) -> int:
        """
        Number of columns in the table.

        Returns
        -------
        int
            The column count of the underlying Arrow ``RecordBatch``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def column_names(self) -> list[str]:
        """
        Column names in declaration order.

        Returns
        -------
        list[str]
            Field names of the underlying Arrow schema, in column order.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def __arrow_c_stream__(self, requested_schema: Any | None = None) -> Any:
        """
        Export the table via the Arrow PyCapsule C-stream protocol.

        Implements the Arrow PyCapsule Interface
        (https://arrow.apache.org/docs/format/CDataInterface/PyCapsuleInterface.html):
        returns a ``PyCapsule`` named ``"arrow_array_stream"`` wrapping an
        ``ArrowArrayStream`` with a single record batch. Consumers such as
        ``pyarrow.table(obj)``, ``polars.DataFrame(obj)``, and DuckDB call
        this automatically; it is rarely invoked directly.

        Parameters
        ----------
        requested_schema : object or None
            Optional PyCapsule-protocol schema-negotiation argument. Accepted
            for interface compliance and ignored: the native schema is
            always exported.

        Returns
        -------
        object
            A ``PyCapsule`` named ``"arrow_array_stream"``.

        Raises
        ------
        RuntimeError
            If the underlying Arrow stream cannot be constructed.
        """
        ...

    def __repr__(self) -> str: ...
