"""Instrument wrappers and JSON helpers for ``finstack_quant.valuations``.

This mirrors ``finstack_quant_valuations::instruments``: category-specific
wrappers live in submodules, while JSON pricing helpers are exported here.

Examples:
--------
>>> import finstack_quant.valuations.instruments as instruments
>>> instruments.__name__
'finstack_quant.valuations.instruments'
"""

from __future__ import annotations

from finstack_quant.finstack_quant import valuations as _valuations

AssetPool = _valuations.instruments.AssetPool
Bond = _valuations.instruments.Bond
CDSIndex = _valuations.instruments.CDSIndex
CDSIndexBuilder = _valuations.instruments.CDSIndexBuilder
CDSTranche = _valuations.instruments.CDSTranche
CDSTrancheBuilder = _valuations.instruments.CDSTrancheBuilder
CapFloor = _valuations.instruments.CapFloor
CapFloorBuilder = _valuations.instruments.CapFloorBuilder
ConvertibleBond = _valuations.instruments.ConvertibleBond
ConvertibleBondBuilder = _valuations.instruments.ConvertibleBondBuilder
CreditDefaultSwap = _valuations.instruments.CreditDefaultSwap
CreditDefaultSwapBuilder = _valuations.instruments.CreditDefaultSwapBuilder
EquityOption = _valuations.instruments.EquityOption
EquityOptionBuilder = _valuations.instruments.EquityOptionBuilder
FixedLegSpec = _valuations.instruments.FixedLegSpec
FloatLegSpec = _valuations.instruments.FloatLegSpec
FxForward = _valuations.instruments.FxForward
FxForwardBuilder = _valuations.instruments.FxForwardBuilder
FxOption = _valuations.instruments.FxOption
FxOptionBuilder = _valuations.instruments.FxOptionBuilder
InterestRateSwap = _valuations.instruments.InterestRateSwap
InterestRateSwapBuilder = _valuations.instruments.InterestRateSwapBuilder
PremiumLegSpec = _valuations.instruments.PremiumLegSpec
ProtectionLegSpec = _valuations.instruments.ProtectionLegSpec
RepLine = _valuations.instruments.RepLine
StructuredCredit = _valuations.instruments.StructuredCredit
StructuredCreditBuilder = _valuations.instruments.StructuredCreditBuilder
Swaption = _valuations.instruments.Swaption
SwaptionBuilder = _valuations.instruments.SwaptionBuilder
TermLoan = _valuations.instruments.TermLoan
Tranche = _valuations.instruments.Tranche
TrancheBuilder = _valuations.instruments.TrancheBuilder
TrancheStructure = _valuations.instruments.TrancheStructure
validate_instrument_json = _valuations.instruments.validate_instrument_json
bond_from_cashflows_json = _valuations.instruments.bond_from_cashflows_json
price_instrument = _valuations.instruments.price_instrument
price_instrument_with_metrics = _valuations.instruments.price_instrument_with_metrics
instrument_cashflows_json = _valuations.instruments.instrument_cashflows_json
list_models = _valuations.instruments.list_models
list_models_grouped = _valuations.instruments.list_models_grouped
list_standard_metrics = _valuations.instruments.list_standard_metrics
list_standard_metrics_grouped = _valuations.instruments.list_standard_metrics_grouped

# Structured-credit tranche analytics (mirrors WASM; takes a tranche id).
structured_credit_tranche_discount_margin = _valuations.instruments.structured_credit_tranche_discount_margin
structured_credit_tranche_breakeven_cdr = _valuations.instruments.structured_credit_tranche_breakeven_cdr
structured_credit_tranche_oas = _valuations.instruments.structured_credit_tranche_oas
structured_credit_tranche_metrics = _valuations.instruments.structured_credit_tranche_metrics
structured_credit_tranche_scenario_table = _valuations.instruments.structured_credit_tranche_scenario_table

__all__: list[str] = [
    "AssetPool",
    "Bond",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSTranche",
    "CDSTrancheBuilder",
    "CapFloor",
    "CapFloorBuilder",
    "ConvertibleBond",
    "ConvertibleBondBuilder",
    "CreditDefaultSwap",
    "CreditDefaultSwapBuilder",
    "EquityOption",
    "EquityOptionBuilder",
    "FixedLegSpec",
    "FloatLegSpec",
    "FxForward",
    "FxForwardBuilder",
    "FxOption",
    "FxOptionBuilder",
    "InterestRateSwap",
    "InterestRateSwapBuilder",
    "PremiumLegSpec",
    "ProtectionLegSpec",
    "RepLine",
    "StructuredCredit",
    "StructuredCreditBuilder",
    "Swaption",
    "SwaptionBuilder",
    "TermLoan",
    "Tranche",
    "TrancheBuilder",
    "TrancheStructure",
    "bond_from_cashflows_json",
    "instrument_cashflows_json",
    "list_models",
    "list_models_grouped",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "price_instrument",
    "price_instrument_with_metrics",
    "structured_credit_tranche_breakeven_cdr",
    "structured_credit_tranche_discount_margin",
    "structured_credit_tranche_metrics",
    "structured_credit_tranche_oas",
    "structured_credit_tranche_scenario_table",
    "validate_instrument_json",
]
