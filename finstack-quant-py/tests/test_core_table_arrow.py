"""Structural tests for `finstack_quant.core.table.ArrowTable`.

`ArrowTable` wraps an Arrow `RecordBatch` and implements the Arrow PyCapsule
C-stream protocol (`__arrow_c_stream__`). This task only wires the class and
the capsule-export machinery; there is no Python-facing constructor yet (that
arrives with the `to_arrow_*` producer methods in a later task), so these
tests are limited to class-level structure. Behavioral round-trip coverage
against pyarrow/polars lands alongside the producer methods.
"""

from __future__ import annotations

import pytest

import finstack_quant.core.table as table_module
from finstack_quant.core.table import ArrowTable


def test_table_module_exports_only_arrow_table() -> None:
    """The `finstack_quant.core.table` module's public surface is `ArrowTable`."""
    assert table_module.__all__ == ["ArrowTable"]
    assert table_module.ArrowTable is ArrowTable


def test_table_module_reports_qualified_package() -> None:
    """The submodule is registered under the `finstack_quant.core` package.

    `__name__` and `__package__` must agree: a compiled submodule that reports
    a bare `__name__` beside a qualified `__package__` satisfies neither
    CPython invariant, and `logging.getLogger(mod.__name__)` then names a
    module that does not exist.
    """
    assert table_module.__package__ == "finstack_quant.core.table"
    assert table_module.__name__ == "finstack_quant.core.table"


def test_arrow_table_has_expected_methods() -> None:
    """`ArrowTable` exposes the documented row/column/stream accessors."""
    for name in ("num_rows", "num_columns", "column_names", "__arrow_c_stream__", "__repr__"):
        assert hasattr(ArrowTable, name), f"ArrowTable missing `{name}`"


def test_arrow_table_module_and_name() -> None:
    """`ArrowTable` reports its canonical dotted module path and class name."""
    assert ArrowTable.__module__ == "finstack_quant.core.table"
    assert ArrowTable.__name__ == "ArrowTable"


def test_arrow_table_class_docstring_cites_pycapsule_spec() -> None:
    """The class docs point readers at the Arrow PyCapsule interface spec."""
    doc = ArrowTable.__doc__ or ""
    assert "PyCapsule" in doc


def test_arrow_c_stream_docstring_mentions_capsule_name_and_schema_arg() -> None:
    """`__arrow_c_stream__` documents the capsule name and `requested_schema`."""
    doc = ArrowTable.__arrow_c_stream__.__doc__ or ""
    assert "arrow_array_stream" in doc
    assert "requested_schema" in doc


def test_arrow_table_has_no_python_constructor() -> None:
    """`ArrowTable` is only producible from Rust (no `#[new]`) in this task."""
    with pytest.raises(TypeError):
        ArrowTable()
