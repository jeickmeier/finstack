"""
Compiled-in JSON Schemas for the statements wire format.

The model specification, the evaluated statement result, and the EBITDA
normalization configuration schemas are embedded in the extension module, so
they always match the installed version and cannot drift from the wheel that
ships them.

Each accessor returns JSON *text*; parse it with ``json.loads`` or hand it
straight to a validator such as ``jsonschema.Draft202012Validator``.

Examples
--------
>>> import json
>>> from finstack_quant.statements import schema
>>> json.loads(schema.financial_model_spec_schema())["title"]
'FinancialModelSpec'

"""

from __future__ import annotations

__all__ = [
    "financial_model_spec_schema",
    "normalization_config_schema",
    "statement_result_schema",
]

def financial_model_spec_schema() -> str:
    """
    Return the JSON Schema for a serialized ``FinancialModelSpec``.

    This is the contract for a statements model definition: periods, nodes,
    forecast specifications, and formulas.

    Returns
    -------
    str
        Pretty-printed JSON Schema text for the financial model specification.

    Raises
    ------
    ValueError
        If the compiled-in schema is malformed or cannot be serialized back to
        JSON text.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.statements import schema
    >>> json.loads(schema.financial_model_spec_schema())["title"]
    'FinancialModelSpec'

    """

def normalization_config_schema() -> str:
    """
    Return the JSON Schema for a serialized ``NormalizationConfig``.

    This is the contract for EBITDA normalization: the adjustment items, their
    sign conventions, and the add-back policy applied during normalization.

    Returns
    -------
    str
        Pretty-printed JSON Schema text for the normalization configuration.

    Raises
    ------
    ValueError
        If the compiled-in schema is malformed or cannot be serialized back to
        JSON text.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.statements import schema
    >>> json.loads(schema.normalization_config_schema())["title"]
    'NormalizationConfig'

    """

def statement_result_schema() -> str:
    """
    Return the JSON Schema for a serialized ``StatementResult``.

    This is the shape of an evaluated model: per-period node values plus the
    numeric mode and rounding context stamped into the result envelope.

    Returns
    -------
    str
        Pretty-printed JSON Schema text for the statement result envelope.

    Raises
    ------
    ValueError
        If the compiled-in schema is malformed or cannot be serialized back to
        JSON text.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.statements import schema
    >>> json.loads(schema.statement_result_schema())["title"]
    'StatementResult'

    """
