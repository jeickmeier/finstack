"""Typing aid for the JSON-native calibration envelope.

Rust owns validation and the versioned JSON schema.  Python intentionally
keeps only this broad top-level alias so the binding does not duplicate every
Rust enum variant as a second, hand-maintained schema.

Examples:
--------
>>> from finstack_quant.valuations.envelope import CalibrationEnvelope
>>> envelope: CalibrationEnvelope = {"schema": "finstack_quant.calibration/1"}
>>> envelope["schema"]
'finstack_quant.calibration/1'

"""

from __future__ import annotations

type _JsonValue = bool | int | float | str | list[_JsonValue] | dict[str, _JsonValue] | None

type CalibrationEnvelope = dict[str, _JsonValue]

__all__ = ["CalibrationEnvelope"]
