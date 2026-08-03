"""Structural credit model bindings.

Mirrors ``finstack_quant_valuations::models::credit``.

Examples:
--------
>>> from finstack_quant.valuations.models.credit import MertonModel
>>> round(MertonModel(100.0, 0.25, 80.0, 0.05).default_probability(1.0), 6)
0.166629

"""

from __future__ import annotations

from finstack_quant.finstack_quant import valuations as _valuations

MertonModel = _valuations.models.credit.MertonModel
DynamicRecoverySpec = _valuations.models.credit.DynamicRecoverySpec
EndogenousHazardSpec = _valuations.models.credit.EndogenousHazardSpec
CreditState = _valuations.models.credit.CreditState
ToggleExerciseModel = _valuations.models.credit.ToggleExerciseModel

__all__ = [
    "CreditState",
    "DynamicRecoverySpec",
    "EndogenousHazardSpec",
    "MertonModel",
    "ToggleExerciseModel",
]
