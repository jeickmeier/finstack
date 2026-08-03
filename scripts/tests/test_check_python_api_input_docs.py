"""Tests for Python API documentation policy."""

from __future__ import annotations

import importlib.util
from pathlib import Path

_SCRIPT_PATH = Path(__file__).parents[1] / "check_python_api_input_docs.py"
_SPEC = importlib.util.spec_from_file_location("check_python_api_input_docs", _SCRIPT_PATH)
assert _SPEC is not None
assert _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)


def test_parameterized_nonthrowing_callable_does_not_require_raises(tmp_path: Path) -> None:
    """Parameters alone must not fabricate an exception contract."""
    fixture = tmp_path / "documented.pyi"
    fixture.write_text(
        '''"""Documented fixture module.

Examples
--------
>>> scale(2.0)
4.0
"""

def scale(value: float) -> float:
    """Scale a finite value by two.

    Parameters
    ----------
    value : float
        Finite scalar to multiply by two.

    Returns
    -------
    float
        Twice the supplied scalar.

    Examples
    --------
    >>> scale(2.0)
    4.0
    """
    ...
''',
        encoding="utf-8",
    )

    assert _MODULE.public_callable_errors(fixture) == []


def test_parameterized_none_return_needs_neither_returns_nor_raises(tmp_path: Path) -> None:
    """A documented command may return and raise nothing."""
    fixture = tmp_path / "command.pyi"
    fixture.write_text(
        '''"""Documented command module.

Examples
--------
>>> record("trade")
"""

def record(name: str) -> None:
    """Record one trade identifier.

    Parameters
    ----------
    name : str
        Non-empty trade identifier to record.

    Examples
    --------
    >>> record("trade")
    """
    ...
''',
        encoding="utf-8",
    )

    assert _MODULE.public_callable_errors(fixture) == []


def test_class_example_covers_ordinary_instance_method(tmp_path: Path) -> None:
    """Routine accessors share the class-level usage example."""
    fixture = tmp_path / "class_api.pyi"
    fixture.write_text(
        '''"""Documented class module.

Examples
--------
>>> Doubler().apply(2.0)
4.0
"""

class Doubler:
    """Multiply scalar inputs by two.

    Examples
    --------
    >>> Doubler().apply(2.0)
    4.0
    """

    def apply(self, value: float) -> float:
        """Multiply one finite scalar by two.

        Parameters
        ----------
        value : float
            Finite scalar to multiply by two.

        Returns
        -------
        float
            Twice the supplied scalar.
        """
        ...
''',
        encoding="utf-8",
    )

    assert _MODULE.public_callable_errors(fixture) == []
