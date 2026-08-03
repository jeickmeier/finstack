#!/usr/bin/env python3
"""Audit public Rust contract types for effective serde and schema support.

The scanner is a conservative, module-aware Rust lexer rather than a full
compiler frontend. It follows file and inline module visibility, public
re-exports, public type aliases, qualified/imported crate-wide manual impls,
ordinary derives, and ``cfg_attr(..., derive(...))``.

Limitations are deliberate and fail-closed for the maintained contract set:
macro-generated declarations are not expanded, ambiguous unqualified impl
targets are not guessed, and Cargo cfg predicates are evaluated for the
effective default build on the current Python host. ``unix``, ``target_arch``,
and ``target_os`` therefore follow the host running the audit; unknown target
predicates are disabled conservatively. Every
maintained contract has an exact crate/path/type registry entry; if module or
syntax handling fails to discover one, check mode reports the missing registry
entry instead of silently reducing coverage.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, replace
import os
from pathlib import Path
import platform
import re
import tomllib

CAPABILITIES = ("Serialize", "Deserialize", "JsonSchema")
CONTRACT_SUFFIXES = ("Spec", "Envelope", "Result")
NON_PRODUCT_CRATES = frozenset({"test-utils"})
CURRENT_TARGET_IS_UNIX = os.name == "posix"
CURRENT_TARGET_ARCH = {
    "arm64": "aarch64",
    "amd64": "x86_64",
}.get(platform.machine().lower(), platform.machine().lower())
CURRENT_TARGET_OS = {
    "darwin": "macos",
}.get(platform.system().lower(), platform.system().lower())


@dataclass(frozen=True)
class PublicType:
    """A mechanically discovered public Rust struct or enum."""

    crate: str
    path: Path
    crate_path: str
    line: int
    name: str
    kind: str
    capabilities: frozenset[str]
    has_marker: bool
    module_path: str = ""
    target_name: str | None = None
    marker_names: frozenset[str] = frozenset()
    marker_defaults: frozenset[str] = frozenset()
    export_paths: frozenset[str] = frozenset()
    lib_name: str = ""

    @property
    def identity(self) -> tuple[str, str, str, str]:
        """Return the module-qualified declaration identity."""
        return self.crate, self.crate_path, self.module_path, self.name

    @property
    def is_contract_like(self) -> bool:
        """Return whether naming or an explicit marker puts the type in scope."""
        return self.has_marker or self.name.endswith(CONTRACT_SUFFIXES)


@dataclass(frozen=True)
class ExceptionEntry:
    """A narrowly reviewed exception to contract capability requirements."""

    crate: str
    path: str
    type_name: str
    category: str
    rationale: str
    allowed_missing: frozenset[str]
    module_path: str | None = None

    @property
    def resolved_module_path(self) -> str:
        """Return explicit or file-derived module path."""
        if self.module_path is not None:
            return self.module_path
        path = Path(self.path)
        relative = path.relative_to("src")
        if relative.name == "lib.rs":
            return ""
        parts = relative.parent.parts if relative.name == "mod.rs" else (*relative.parent.parts, relative.stem)
        return "::".join(parts)

    @property
    def identity(self) -> tuple[str, str, str, str]:
        """Return the stable declaration identity."""
        return self.crate, self.path, self.resolved_module_path, self.type_name


@dataclass(frozen=True)
class Diagnostic:
    """A contract declaration missing one or more required capabilities."""

    crate: str
    path: Path
    line: int
    type_name: str
    missing: tuple[str, ...]


@dataclass(frozen=True)
class StaleException:
    """A reviewed classification that no longer exactly matches source."""

    entry: ExceptionEntry
    reason: str
    actual_missing: frozenset[str] | None

    @property
    def allowed_missing(self) -> frozenset[str]:
        """Return the capability gap recorded by the classification."""
        return self.entry.allowed_missing


@dataclass(frozen=True)
class CrateSummary:
    """Deterministic audit counts for one product crate."""

    crate: str
    public_types: int
    contract_types: int
    reviewed_exceptions: int
    failures: int


@dataclass(frozen=True)
class AuditReport:
    """Complete result of a workspace audit."""

    declarations: tuple[PublicType, ...]
    diagnostics: tuple[Diagnostic, ...]
    reviewed_exceptions: tuple[ExceptionEntry, ...]
    stale_exceptions: tuple[StaleException, ...]
    summaries: tuple[CrateSummary, ...]
    contract_identities: frozenset[tuple[str, str, str, str]]

    @property
    def failed(self) -> bool:
        """Return whether check mode must fail."""
        return bool(self.diagnostics or self.stale_exceptions)


class AuditConfigurationError(ValueError):
    """Raised when the requested repository root cannot be audited."""


def _exception(
    crate: str,
    path: str,
    type_names: Sequence[str],
    category: str,
    rationale: str,
    allowed_missing: frozenset[str] = frozenset({"Deserialize", "JsonSchema"}),
) -> tuple[ExceptionEntry, ...]:
    return tuple(
        ExceptionEntry(
            crate=crate,
            path=path,
            type_name=type_name,
            category=category,
            rationale=rationale,
            allowed_missing=allowed_missing,
        )
        for type_name in type_names
    )


ONE_WAY_EXCEPTIONS = (
    *_exception(
        "attribution",
        "src/return_contribution.rs",
        (
            "ReturnContributionResult",
            "InstrumentContribution",
            "GroupContribution",
            "FactorContribution",
            "BenchmarkRelativeContribution",
        ),
        "attribution-report",
        "Canonical emitted attribution report rows; no supported inbound wire format.",
    ),
    *_exception(
        "factor-model",
        "src/credit/decomposition.rs",
        ("LevelValuesAtDate", "LevelsAtDate", "LevelValuesDelta", "PeriodDecomposition"),
        "decomposition-output",
        "Computed credit decomposition output; reconstructed by rerunning decomposition.",
    ),
    *_exception(
        "portfolio",
        "src/factor_model/mod.rs",
        (
            "PositionEsContributionView",
            "ParametricEsDecompositionView",
            "PositionVarContributionView",
            "ParametricVarDecompositionView",
            "PositionBudgetEntryView",
            "RiskBudgetResultView",
        ),
        "binding-view",
        "Binding-oriented factor-risk view with no accepted inbound representation.",
    ),
    *_exception(
        "portfolio",
        "src/factor_model/weight_allocation.rs",
        ("WeightAllocationResult", "StrategyAllocation", "AllocationDiagnostics"),
        "allocation-output",
        "Computed allocation output; callers persist inputs and rerun allocation.",
    ),
    *_exception(
        "portfolio",
        "src/scenarios.rs",
        ("ScenarioRevalueView", "ScenarioPnlView"),
        "scenario-view",
        "Scenario output view intentionally replacing persistence-implying Envelope names.",
    ),
    *_exception(
        "portfolio",
        "src/sensitivity/json.rs",
        ("SensitivityMatrixJson", "FactorPnlProfileJson"),
        "binding-view",
        "JSON projection for host bindings, not an inbound Rust contract.",
    ),
    *_exception(
        "statements-analytics",
        "src/analysis/reports.rs",
        ("CreditAssessmentPoint", "CreditAssessment"),
        "analysis-report",
        "Computed statement-analysis report; source statement results are canonical.",
    ),
    *_exception(
        "valuations",
        "src/calibration/api/validate.rs",
        ("ValidationReport", "DependencyGraph", "DependencyNode"),
        "validation-report",
        "Transient calibration validation view, regenerated from the input envelope.",
    ),
)


def _classification(
    crate: str,
    path: str,
    type_names: Sequence[str],
    category: str,
    rationale: str,
    allowed_missing: frozenset[str] = frozenset({"JsonSchema"}),
) -> tuple[ExceptionEntry, ...]:
    return _exception(
        crate,
        path,
        type_names,
        category,
        rationale,
        allowed_missing,
    )


def _computed_output(
    crate: str,
    path: str,
    type_names: Sequence[str],
) -> tuple[ExceptionEntry, ...]:
    return _classification(
        crate,
        path,
        type_names,
        "non-maintained-serde-output",
        "Computed output supports serde transport but is reproduced from canonical inputs; "
        "it is not a maintained/versioned persistence document.",
    )


def _in_process_spec(
    crate: str,
    path: str,
    type_names: Sequence[str],
) -> tuple[ExceptionEntry, ...]:
    return _classification(
        crate,
        path,
        type_names,
        "in-process-serde-spec",
        "Operation input is accepted by serde-powered adapters but has no maintained "
        "versioned-document or standalone-schema promise.",
    )


NON_MAINTAINED_SERDE_EXCEPTIONS = (
    *_computed_output("analytics", "src/benchmark.rs", ("BetaResult", "GreeksResult", "MultiFactorResult")),
    *_in_process_spec(
        "attribution",
        "src/return_contribution.rs",
        ("ReturnContributionSpec",),
    ),
    *_computed_output("core", "src/credit/pd/master_scale.rs", ("MasterScaleResult",)),
    *_classification(
        "core",
        "src/credit/registry.rs",
        ("CreditAssumptionRegistry",),
        "internal-registry-document",
        "Embedded credit-assumption registry is loaded through component-specific validation "
        "and is outside the maintained public persistence catalog.",
    ),
    *_computed_output("core", "src/credit/scoring/types.rs", ("ScoringResult",)),
    *_computed_output("core", "src/expr/ast.rs", ("EvaluationResult",)),
    *_in_process_spec("core", "src/market_data/bumps.rs", ("BumpSpec",)),
    *_computed_output(
        "core",
        "src/market_data/term_structures/base_correlation.rs",
        ("ArbitrageCheckResult",),
    ),
    *_computed_output("core", "src/math/volatility/heston.rs", ("HestonCalibrationResult",)),
    *_computed_output("core", "src/money/fx/types.rs", ("FxRateResult",)),
    *_classification(
        "core",
        "src/rating_scales.rs",
        ("RatingScaleRegistry",),
        "internal-registry-document",
        "Embedded rating-scale registry uses component validation and is explicitly outside "
        "the maintained public persistence catalog.",
    ),
    *_in_process_spec("covenants", "src/engine.rs", ("CovenantSpec", "CovenantTestSpec")),
    *_computed_output("margin", "src/regulatory/frtb/types.rs", ("FrtbSbaResult",)),
    *_computed_output("margin", "src/regulatory/sa_ccr/types.rs", ("EadResult",)),
    *_computed_output("monte_carlo", "src/results.rs", ("MonteCarloResult",)),
    *_computed_output("portfolio", "src/brinson.rs", ("BrinsonPeriodResult",)),
    *_computed_output("portfolio", "src/excess_return.rs", ("ExcessReturnResult",)),
    *_computed_output("portfolio", "src/factor_brinson.rs", ("FactorBrinsonResult",)),
    *_computed_output("portfolio", "src/factor_model/risk_budget.rs", ("RiskBudgetResult",)),
    *_in_process_spec(
        "portfolio",
        "src/factor_model/weight_allocation.rs",
        ("WeightAllocationSpec",),
    ),
    *_computed_output("portfolio", "src/factor_model/whatif.rs", ("WhatIfResult", "StressResult")),
    *_computed_output(
        "portfolio",
        "src/fi_attribution.rs",
        ("FiAttributionResult", "FiCarinoLinkedResult"),
    ),
    *_computed_output(
        "portfolio",
        "src/grid_attribution.rs",
        ("GridAttributionResult", "GridCarinoLinkedResult"),
    ),
    *_computed_output("portfolio", "src/liquidity/lvar.rs", ("LvarResult",)),
    *_classification(
        "portfolio",
        "src/margin/results.rs",
        ("PortfolioMarginResult",),
        "non-maintained-serde-output",
        "Portfolio margin aggregate round-trips through its private wire adapter for host "
        "transport, but is not a versioned maintained result document.",
    ),
    *_in_process_spec("portfolio", "src/optimization/helpers.rs", ("PortfolioOptimizationSpec",)),
    *_in_process_spec("portfolio", "src/portfolio.rs", ("PortfolioSpec",)),
    *_in_process_spec("portfolio", "src/position.rs", ("PositionSpec",)),
    *_computed_output("portfolio", "src/replay.rs", ("ReplayResult",)),
    *_classification(
        "scenarios",
        "src/engine.rs",
        ("ApplicationEnvelope",),
        "in-process-execution-envelope",
        "Scenario application receipt is an immediate execution handoff, not a maintained "
        "persisted envelope from the contract catalog.",
    ),
    *_computed_output("scenarios", "src/horizon.rs", ("HorizonResult",)),
    *_computed_output("statements", "src/adjustments/types.rs", ("NormalizationResult",)),
    *_in_process_spec(
        "statements",
        "src/checks/suite.rs",
        ("CheckSuiteSpec", "BuiltinCheckSpec", "FormulaCheckSpec"),
    ),
    *_classification(
        "statements",
        "src/registry/schema.rs",
        ("MetricRegistry",),
        "internal-registry-document",
        "Embedded statement-metric registry has registry-specific validation and is outside "
        "the maintained public persistence catalog.",
    ),
    *_computed_output(
        "statements-analytics",
        "src/analysis/comps/scoring.rs",
        ("RelativeValueResult",),
    ),
    *_computed_output(
        "statements-analytics",
        "src/analysis/comps/stats.rs",
        ("RegressionResult",),
    ),
    *_in_process_spec(
        "statements-analytics",
        "src/analysis/credit/adjusted_net_debt.rs",
        ("AdjustedNetDebtSpec",),
    ),
    *_computed_output("statements-analytics", "src/analysis/ecl/cecl.rs", ("CeclResult",)),
    *_computed_output(
        "statements-analytics",
        "src/analysis/ecl/engine.rs",
        ("EclResult", "WeightedEclResult", "ExposureEclResult"),
    ),
    *_computed_output(
        "statements-analytics",
        "src/analysis/ecl/portfolio.rs",
        ("PortfolioEclResult",),
    ),
    *_computed_output(
        "statements-analytics",
        "src/analysis/ecl/staging.rs",
        ("StageResult",),
    ),
    *_classification(
        "statements-analytics",
        "src/analysis/scenarios/types.rs",
        ("ParameterSpec",),
        "in-process-serde-spec",
        "Scenario parameter input is accepted by the analysis engine but is not a maintained versioned document.",
    ),
    *_computed_output(
        "statements-analytics",
        "src/analysis/scenarios/types.rs",
        ("SensitivityResult",),
    ),
    *_in_process_spec(
        "statements-analytics",
        "src/templates/real_estate/mod.rs",
        (
            "SimpleLeaseSpec",
            "RentStepSpec",
            "FreeRentWindowSpec",
            "RenewalSpec",
            "LeaseSpec",
            "ManagementFeeSpec",
        ),
    ),
    *_computed_output(
        "valuations",
        "src/instruments/fixed_income/structured_credit/metrics/risk/oas.rs",
        ("OasResult",),
    ),
)

RESULT_ALIAS_EXCEPTIONS = tuple(
    entry
    for crate, path in (
        ("analytics", "src/correlation/error.rs"),
        ("core", "src/error/mod.rs"),
        ("portfolio", "src/error.rs"),
        ("scenarios", "src/error.rs"),
        ("statements", "src/error.rs"),
        ("valuations", "src/error.rs"),
    )
    for entry in _classification(
        crate,
        path,
        ("Result",),
        "generic-error-result-alias",
        "Generic alias to std::result::Result for crate error propagation; it is a type "
        "constructor, not a serializable DTO or persistence contract.",
        frozenset(CAPABILITIES),
    )
)


def _runtime_exception(
    crate: str,
    path: str,
    type_names: Sequence[str],
    category: str = "runtime-result",
) -> tuple[ExceptionEntry, ...]:
    return _exception(
        crate,
        path,
        type_names,
        category,
        "In-memory algorithm state or output; never accepted as a persisted wire contract.",
        frozenset(CAPABILITIES),
    )


# Runtime Result names are individually reviewed because their suffix alone cannot
# distinguish persisted DTOs from in-memory solver or algorithm state.
RUNTIME_RESULT_EXCEPTIONS = (
    *_runtime_exception("core", "src/math/linalg.rs", ("LedoitWolfResult",)),
    *_runtime_exception("margin", "src/calculators/traits.rs", ("ImResult",)),
    *_runtime_exception("margin", "src/calculators/vm.rs", ("VmResult",)),
    *_runtime_exception(
        "statements",
        "src/capital_structure/waterfall/mod.rs",
        ("WaterfallPeriodResult",),
    ),
    *_runtime_exception(
        "statements-analytics",
        "src/analysis/valuation/corporate.rs",
        ("CorporateValuationResult", "DcfSensitivityResult"),
    ),
    *_runtime_exception(
        "statements-analytics",
        "src/analysis/valuation/lbo.rs",
        ("LboResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/credit_derivatives/cds_index/types.rs",
        ("ConstituentResult", "IndexResult", "IndexParSpreadResult"),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/bond/pricing/engine/merton_mc.rs",
        ("MertonMcResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/bond/pricing/ytm_solver.rs",
        ("YtmPricingSpec",),
        "runtime-spec",
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/cmo/waterfall.rs",
        ("WaterfallPeriodResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/dollar_roll/carry.rs",
        ("CarryResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/revolving_credit/pricer/unified.rs",
        ("PathResult", "EnhancedMonteCarloResult"),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/structured_credit/types/pool.rs",
        ("ConcentrationCheckResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/fixed_income/tba/allocation.rs",
        ("AllocationResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/instruments/rates/cms_swap/types.rs",
        ("FundingLegSpec",),
        "runtime-spec",
    ),
    *_runtime_exception("valuations", "src/metrics/risk/var_calculator.rs", ("VarResult",)),
    *_runtime_exception("valuations", "src/models/closed_form/asian.rs", ("AsianPriceResult",)),
    *_runtime_exception(
        "valuations",
        "src/models/trees/short_rate_tree.rs",
        ("CalibrationResult",),
    ),
    *_runtime_exception(
        "valuations",
        "src/models/trees/tree_framework/evolution.rs",
        ("BarrierSpec",),
        "runtime-spec",
    ),
)

REVIEWED_EXCEPTIONS = (
    *ONE_WAY_EXCEPTIONS,
    *NON_MAINTAINED_SERDE_EXCEPTIONS,
    *RESULT_ALIAS_EXCEPTIONS,
    *RUNTIME_RESULT_EXCEPTIONS,
)


@dataclass(frozen=True)
class _ModuleSpan:
    path: str
    parent: str
    public: bool
    start: int
    end: int


@dataclass(frozen=True)
class _SourceUnit:
    path: Path
    crate_path: str
    source: str
    text: str
    base_module: str
    spans: tuple[_ModuleSpan, ...]
    default_features: frozenset[str]


MAINTAINED_CONTRACTS = frozenset({
    ("valuations", "src/instruments/json_loader.rs", "InstrumentEnvelope"),
    ("valuations", "src/calibration/api/schema.rs", "CalibrationEnvelope"),
    ("valuations", "src/calibration/api/schema.rs", "CalibrationResultEnvelope"),
    ("core", "src/market_data/context/state_serde.rs", "MarketContextState"),
    ("statements", "src/types/model.rs", "FinancialModelSpec"),
    ("scenarios", "src/envelope.rs", "ScenarioEnvelope"),
    ("factor-model", "src/envelope.rs", "FactorModelConfigEnvelope"),
    ("factor-model", "src/credit/hierarchy.rs", "CreditFactorModel"),
    ("portfolio", "src/materialization/envelope.rs", "PortfolioMaterializationEnvelope"),
    ("valuations", "src/results/valuation_result.rs", "ValuationResult"),
    ("statements", "src/evaluator/results.rs", "StatementResult"),
    ("portfolio", "src/results.rs", "PortfolioResult"),
    ("portfolio", "src/optimization/result.rs", "PortfolioOptimizationResult"),
})

# Required public binding outputs are not maintained persistence contracts:
# they need no version marker, strict loader, or contract-matrix entry. This
# registry keeps their public reachability and effective output schema
# fail-closed when module/re-export resolution changes.
REQUIRED_PUBLIC_TYPES = {
    (
        "valuations",
        "src/instruments/common_impl/cashflow_export.rs",
        "InstrumentCashflowEnvelope",
    ): CAPABILITIES,
}

MAINTAINED_REQUIRED_CAPABILITIES = {
    identity: (
        frozenset({"Serialize", "JsonSchema"})
        if identity
        == (
            "portfolio",
            "src/optimization/result.rs",
            "PortfolioOptimizationResult",
        )
        else CAPABILITIES
    )
    for identity in MAINTAINED_CONTRACTS
}

MAINTAINED_ONE_WAY_OUTPUTS = frozenset({
    (
        "portfolio",
        "src/optimization/result.rs",
        "PortfolioOptimizationResult",
    ),
})

ONE_WAY_OUTPUT_IDENTITIES = frozenset({
    *((entry.crate, entry.path, entry.type_name) for entry in ONE_WAY_EXCEPTIONS),
    *MAINTAINED_ONE_WAY_OUTPUTS,
})


def _blank(chars: list[str], start: int, end: int) -> None:
    for index in range(start, min(end, len(chars))):
        if chars[index] != "\n":
            chars[index] = " "


def _mask_non_code(source: str) -> str:
    """Mask comments and Rust literals while preserving byte offsets.

    This is intentionally a conservative lexer, not a Rust parser. It handles
    nested block comments, byte/ordinary strings, arbitrary-hash raw strings,
    character literals, and lifetimes. Macro-expanded declarations are outside
    scope; known derives that generate serde/schema impls are listed explicitly.
    """
    chars = list(source)
    index = 0
    block_depth = 0
    state = "code"
    delimiter = ""
    while index < len(chars):
        current = chars[index]
        following = chars[index + 1] if index + 1 < len(chars) else ""
        if state == "code":
            raw_match = re.match(r'(?:br|r)(?P<hashes>#{0,255})"', source[index:])
            if raw_match and (index == 0 or not (source[index - 1].isalnum() or source[index - 1] == "_")):
                hashes = raw_match.group("hashes")
                close = f'"{hashes}'
                end = source.find(close, index + raw_match.end())
                end = len(source) if end < 0 else end + len(close)
                _blank(chars, index, end)
                index = end
                continue
            if current == "/" and following == "/":
                chars[index] = chars[index + 1] = " "
                index += 2
                state = "line-comment"
                continue
            if current == "/" and following == "*":
                chars[index] = chars[index + 1] = " "
                index += 2
                block_depth = 1
                state = "block-comment"
                continue
            if current == '"' or (current == "b" and following == '"'):
                if current == "b":
                    chars[index] = " "
                    index += 1
                delimiter = '"'
                index += 1
                state = "literal"
                continue
            if current == "'" and re.match(r"'(?:\\.|[^\\'\n])'", source[index : index + 4]):
                delimiter = "'"
                index += 1
                state = "literal"
                continue
        elif state == "line-comment":
            if current == "\n":
                state = "code"
            else:
                chars[index] = " "
        elif state == "block-comment":
            if current == "/" and following == "*":
                chars[index] = chars[index + 1] = " "
                index += 2
                block_depth += 1
                continue
            if current == "*" and following == "/":
                chars[index] = chars[index + 1] = " "
                index += 2
                block_depth -= 1
                if block_depth == 0:
                    state = "code"
                continue
            if current != "\n":
                chars[index] = " "
        elif state == "literal":
            if current == "\\" and following:
                _blank(chars, index, index + 2)
                index += 2
                continue
            if current == delimiter:
                state = "code"
            elif current != "\n":
                chars[index] = " "
        index += 1
    return "".join(chars)


def _matching_open(text: str, close_index: int, opener: str, closer: str) -> int | None:
    depth = 0
    for index in range(close_index, -1, -1):
        if text[index] == closer:
            depth += 1
        elif text[index] == opener:
            depth -= 1
            if depth == 0:
                return index
    return None


def _matching_close(text: str, open_index: int, opener: str = "{", closer: str = "}") -> int | None:
    depth = 0
    for index in range(open_index, len(text)):
        if text[index] == opener:
            depth += 1
        elif text[index] == closer:
            depth -= 1
            if depth == 0:
                return index
    return None


def _preceding_attributes(
    text: str,
    declaration_start: int,
    *,
    attribute_source: str | None = None,
) -> tuple[str, ...]:
    ranges: list[tuple[int, int]] = []
    cursor = declaration_start - 1
    while cursor >= 0:
        while cursor >= 0 and text[cursor].isspace():
            cursor -= 1
        if cursor < 0 or text[cursor] != "]":
            break
        open_index = _matching_open(text, cursor, "[", "]")
        if open_index is None:
            break
        hash_index = open_index - 1
        while hash_index >= 0 and text[hash_index].isspace():
            hash_index -= 1
        if hash_index < 0 or text[hash_index] != "#":
            break
        ranges.append((hash_index, cursor + 1))
        cursor = hash_index - 1
    ranges.reverse()
    source = text if attribute_source is None else attribute_source
    return tuple(source[start:end] for start, end in ranges)


def _derived_capabilities(
    attributes: Iterable[str],
    default_features: frozenset[str] = frozenset(),
) -> set[str]:
    capabilities: set[str] = set()
    for attribute in attributes:
        derive_source = attribute
        cfg_attr = re.search(r"\bcfg_attr\s*\((.*)\)\s*\]", attribute, flags=re.DOTALL)
        if cfg_attr is not None:
            parts = _split_top_level(cfg_attr.group(1))
            if len(parts) < 2 or not _cfg_condition_enabled(
                parts[0],
                default_features,
            ):
                continue
            derive_source = ",".join(parts[1:])
        for derive in re.findall(r"\bderive\s*\((.*?)\)", derive_source, flags=re.DOTALL):
            for item in derive.split(","):
                trait = item.strip().split("::")[-1]
                if trait in CAPABILITIES:
                    capabilities.add(trait)
    return capabilities


def _manual_impls(
    text: str,
    *,
    attribute_source: str | None = None,
    default_features: frozenset[str] = frozenset(),
) -> tuple[tuple[str, str, int], ...]:
    pattern = re.compile(
        r"\bimpl(?:\s*<[^{};]*>)?\s+"
        r"(?P<trait>(?:(?:::)?[A-Za-z_]\w*::)*(?:Serialize|Deserialize|JsonSchema))"
        r"(?:\s*<[^{};]*>)?\s+for\s+"
        r"(?P<target>(?:(?:::)?[A-Za-z_]\w*::)*[A-Za-z_]\w*)",
        flags=re.DOTALL,
    )
    implementations = []
    for match in pattern.finditer(text):
        attributes = _preceding_attributes(
            text,
            match.start(),
            attribute_source=attribute_source,
        )
        if _item_enabled(attributes, default_features):
            implementations.append((match.group("trait").split("::")[-1], match.group("target"), match.start()))
    return tuple(implementations)


def _cfg_condition_enabled(
    condition: str,
    default_features: frozenset[str] = frozenset(),
) -> bool:
    condition = re.sub(r"\s+", "", condition)
    host_predicates = {
        "test": False,
        "unix": CURRENT_TARGET_IS_UNIX,
        "windows": not CURRENT_TARGET_IS_UNIX,
    }
    if condition in host_predicates:
        return host_predicates[condition]
    selector = re.fullmatch(r'(feature|target_arch|target_os)="([^"]+)"', condition)
    if selector is not None:
        name, value = selector.groups()
        selectors = {
            "feature": default_features,
            "target_arch": frozenset({CURRENT_TARGET_ARCH}),
            "target_os": frozenset({CURRENT_TARGET_OS}),
        }
        return value in selectors[name]
    for operator, reducer in (
        ("not", lambda values: not values[0]),
        ("all", all),
        ("any", any),
    ):
        prefix = f"{operator}("
        if condition.startswith(prefix) and condition.endswith(")"):
            values = [
                _cfg_condition_enabled(part, default_features) for part in _split_top_level(condition[len(prefix) : -1])
            ]
            return bool(values) and reducer(values)
    return False


def _item_enabled(
    attributes: Iterable[str],
    default_features: frozenset[str] = frozenset(),
) -> bool:
    for attribute in attributes:
        cfg = re.search(r"\bcfg\s*\((.*)\)\s*\]", attribute, flags=re.DOTALL)
        if cfg is not None and not _cfg_condition_enabled(
            cfg.group(1),
            default_features,
        ):
            return False
    return True


def _mask_disabled_inline_modules(
    text: str,
    source: str,
    default_features: frozenset[str],
) -> str:
    chars = list(text)
    pattern = re.compile(r"\b(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{")
    for match in tuple(pattern.finditer(text)):
        attributes = _preceding_attributes(
            text,
            match.start(),
            attribute_source=source,
        )
        if _item_enabled(attributes, default_features):
            continue
        open_index = text.find("{", match.start(), match.end())
        close_index = _matching_close(text, open_index)
        if close_index is None:
            continue
        for index in range(match.start(), close_index + 1):
            if chars[index] != "\n":
                chars[index] = " "
    return "".join(chars)


def _module_for_file(crate_root: Path, path: Path) -> str:
    relative = path.relative_to(crate_root / "src")
    if relative.name == "lib.rs":
        return ""
    parts = relative.parent.parts if relative.name == "mod.rs" else (*relative.parent.parts, relative.stem)
    return "::".join(parts)


def _module_at(unit: _SourceUnit, position: int) -> str:
    containing = [span for span in unit.spans if span.start < position < span.end]
    return max(containing, key=lambda span: span.start).path if containing else unit.base_module


def _inline_module_spans(text: str, base_module: str) -> tuple[_ModuleSpan, ...]:
    spans: list[_ModuleSpan] = []
    pattern = re.compile(r"\b(?P<vis>pub\s+)?mod\s+(?P<name>[A-Za-z_]\w*)\s*\{")
    for match in pattern.finditer(text):
        parent = base_module
        containing = [span for span in spans if span.start < match.start() < span.end]
        if containing:
            parent = max(containing, key=lambda span: span.start).path
        open_index = text.find("{", match.start(), match.end())
        close_index = _matching_close(text, open_index)
        if close_index is None:
            continue
        path = "::".join(filter(None, (parent, match.group("name"))))
        spans.append(
            _ModuleSpan(
                path=path,
                parent=parent,
                public=bool(match.group("vis")),
                start=open_index,
                end=close_index,
            )
        )
    return tuple(spans)


def _crate_relative(path: Path, source_root: Path, crate: str) -> tuple[Path, str]:
    relative_path = path.relative_to(source_root)
    crate_prefix = Path("finstack-quant") / crate
    try:
        crate_path = relative_path.relative_to(crate_prefix).as_posix()
    except ValueError:
        crate_path = path.relative_to(path.parents[1]).as_posix()
    return relative_path, crate_path


def _marker_metadata(body: str) -> tuple[frozenset[str], frozenset[str]]:
    names: set[str] = set()
    defaults: set[str] = set()
    field_pattern = re.compile(
        r"(?:(?:pub(?:\([^)]*\))?)\s+)?(?P<name>schema_version|schema|version)\s*:",
        flags=re.IGNORECASE,
    )
    for field in field_pattern.finditer(body):
        marker_name = field.group("name").lower()
        names.add(marker_name)
        field_attributes = _preceding_attributes(body, field.start())
        has_default = any("serde" in item and re.search(r"\bdefault\b", item) for item in field_attributes)
        if has_default:
            defaults.add(marker_name)
    return frozenset(names), frozenset(defaults)


def _marker_is_contract(
    declaration: PublicType,
    capabilities: frozenset[str] | None = None,
) -> bool:
    if (declaration.crate, declaration.crate_path, declaration.name) in MAINTAINED_CONTRACTS:
        return True
    effective = declaration.capabilities if capabilities is None else capabilities
    recognized = declaration.marker_names & {"schema", "schema_version"}
    return bool(recognized and effective & {"Serialize", "Deserialize"})


def _parse_unit_declarations(
    unit: _SourceUnit,
    *,
    crate: str,
    source_root: Path,
    default_features: frozenset[str] = frozenset(),
    lib_name: str = "",
) -> list[PublicType]:
    declarations: list[PublicType] = []
    pattern = re.compile(r"\bpub\s+(?P<kind>struct|enum|type)\s+(?P<name>[A-Za-z_]\w*)")
    relative_path, crate_path = _crate_relative(unit.path, source_root, crate)
    for match in pattern.finditer(unit.text):
        attributes = _preceding_attributes(
            unit.text,
            match.start(),
            attribute_source=unit.source,
        )
        if not _item_enabled(attributes, default_features):
            continue
        kind = match.group("kind")
        name = match.group("name")
        body = ""
        target_name = None
        if kind == "type":
            semicolon = unit.text.find(";", match.end())
            equals = unit.text.find("=", match.end(), semicolon)
            if equals >= 0 and semicolon >= 0:
                target_name = unit.text[equals + 1 : semicolon].strip()
        else:
            boundary_match = re.search(r"[{;]", unit.text[match.end() :])
            if boundary_match is not None:
                boundary = match.end() + boundary_match.start()
                if unit.text[boundary] == "{":
                    close_index = _matching_close(unit.text, boundary)
                    if close_index is not None:
                        body = unit.text[boundary + 1 : close_index]
        marker_names, marker_defaults = _marker_metadata(body)
        capabilities = frozenset(_derived_capabilities(attributes, default_features))
        declarations.append(
            PublicType(
                crate=crate,
                path=relative_path,
                crate_path=crate_path,
                line=unit.source.count("\n", 0, match.start()) + 1,
                name=name,
                kind="alias" if kind == "type" else kind,
                capabilities=capabilities,
                has_marker=False,
                module_path=_module_at(unit, match.start()),
                target_name=target_name,
                marker_names=marker_names,
                marker_defaults=marker_defaults,
                lib_name=lib_name,
            )
        )
        declarations[-1] = replace(
            declarations[-1],
            has_marker=_marker_is_contract(declarations[-1], capabilities),
        )
    return declarations


def _split_top_level(value: str) -> list[str]:
    parts: list[str] = []
    stack: list[str] = []
    closers = {"(": ")", "[": "]", "{": "}"}
    start = 0
    for index, character in enumerate(value):
        if character in closers:
            stack.append(closers[character])
        elif stack and character == stack[-1]:
            stack.pop()
        elif character == "," and not stack:
            parts.append(value[start:index].strip())
            start = index + 1
    parts.append(value[start:].strip())
    return [part for part in parts if part]


def _expand_use_tree(value: str) -> tuple[str, ...]:
    value = re.sub(r"\s+as\s+", "@AS@", value)
    value = re.sub(r"\s+", "", value)
    open_index = value.find("{")
    if open_index < 0:
        return (value,)
    close_index = _matching_close(value, open_index, "{", "}")
    if close_index is None:
        return (value,)
    prefix = value[:open_index]
    suffix = value[close_index + 1 :]
    expanded = []
    for part in _split_top_level(value[open_index + 1 : close_index]):
        expanded.extend(_expand_use_tree(f"{prefix}{part}{suffix}"))
    return tuple(expanded)


def _reference_candidates(reference: str, module_path: str) -> tuple[str, ...]:
    reference = reference.strip().lstrip(":")
    if reference.startswith("crate::"):
        return (reference.removeprefix("crate::"),)
    if reference.startswith("self::"):
        return ("::".join(filter(None, (module_path, reference.removeprefix("self::")))),)
    if reference.startswith("super::"):
        parent_parts = module_path.split("::") if module_path else []
        while reference.startswith("super::"):
            reference = reference.removeprefix("super::")
            if parent_parts:
                parent_parts.pop()
        return ("::".join((*parent_parts, reference)),)
    current = "::".join(filter(None, (module_path, reference)))
    return tuple(dict.fromkeys((reference, current)))


def _resolve_symbol(
    reference: str,
    *,
    module_path: str,
    symbols: dict[str, PublicType],
    imports: dict[tuple[str, str], str],
) -> str | None:
    reference = re.sub(r"<.*>$", "", reference.strip())
    if "::" not in reference:
        imported = imports.get((module_path, reference))
        if imported is not None:
            reference = imported
    for candidate in _reference_candidates(reference, module_path):
        if candidate in symbols:
            return candidate
    target_name = reference.rsplit("::", 1)[-1]
    matches = [path for path in symbols if path.rsplit("::", 1)[-1] == target_name]
    if len(matches) == 1:
        return matches[0]
    return None


def _unit_uses(unit: _SourceUnit, *, public_only: bool) -> tuple[tuple[str, str, int], ...]:
    prefix = r"\bpub\s+use" if public_only else r"\b(?:pub\s+)?use"
    pattern = re.compile(rf"{prefix}\s+(?P<body>[^;]+);", flags=re.DOTALL)
    uses = []
    for match in pattern.finditer(unit.text):
        attributes = _preceding_attributes(
            unit.text,
            match.start(),
            attribute_source=unit.source,
        )
        if not _item_enabled(attributes, unit.default_features):
            continue
        module_path = _module_at(unit, match.start())
        for expanded in _expand_use_tree(match.group("body")):
            uses.append((module_path, expanded, match.start()))
    return tuple(uses)


def _module_edges(units: Sequence[_SourceUnit]) -> tuple[tuple[str, str, bool], ...]:
    edges: list[tuple[str, str, bool]] = []
    pattern = re.compile(r"\b(?P<vis>pub\s+)?mod\s+(?P<name>[A-Za-z_]\w*)\s*;")
    for unit in units:
        edges.extend((span.parent, span.path, span.public) for span in unit.spans)
        for match in pattern.finditer(unit.text):
            attributes = _preceding_attributes(
                unit.text,
                match.start(),
                attribute_source=unit.source,
            )
            if not _item_enabled(attributes, unit.default_features):
                continue
            parent = _module_at(unit, match.start())
            child = "::".join(filter(None, (parent, match.group("name"))))
            edges.append((parent, child, bool(match.group("vis"))))
    return tuple(edges)


def _reachable_modules(units: Sequence[_SourceUnit]) -> set[str]:
    reachable = {""}
    edges = _module_edges(units)
    changed = True
    while changed:
        changed = False
        for parent, child, public in edges:
            if public and parent in reachable and child not in reachable:
                reachable.add(child)
                changed = True
    return reachable


def _default_enabled_modules(units: Sequence[_SourceUnit]) -> set[str]:
    enabled = {""}
    edges = _module_edges(units)
    changed = True
    while changed:
        changed = False
        for parent, child, _ in edges:
            if parent in enabled and child not in enabled:
                enabled.add(child)
                changed = True
    return enabled


def _imports(units: Sequence[_SourceUnit]) -> dict[tuple[str, str], str]:
    imports: dict[tuple[str, str], str] = {}
    for unit in units:
        for module_path, expanded, _ in _unit_uses(unit, public_only=False):
            reference, _, alias = expanded.partition("@AS@")
            local_name = alias or reference.rsplit("::", 1)[-1]
            candidates = _reference_candidates(reference, module_path)
            imports[(module_path, local_name)] = candidates[0]
    return imports


def _apply_manual_impls(
    declarations: Sequence[PublicType],
    units: Sequence[_SourceUnit],
    enabled_modules: set[str],
    default_features: frozenset[str] = frozenset(),
) -> tuple[PublicType, ...]:
    symbols = {
        "::".join(filter(None, (item.module_path, item.name))): item for item in declarations if item.kind != "alias"
    }
    imports = _imports(units)
    additions: dict[str, set[str]] = defaultdict(set)
    for unit in units:
        if unit.base_module not in enabled_modules:
            continue
        for trait, target, position in _manual_impls(
            unit.text,
            attribute_source=unit.source,
            default_features=default_features,
        ):
            module_path = _module_at(unit, position)
            resolved = _resolve_symbol(target, module_path=module_path, symbols=symbols, imports=imports)
            if resolved is not None:
                additions[resolved].add(trait)
    updated = []
    for item in declarations:
        symbol = "::".join(filter(None, (item.module_path, item.name)))
        capabilities = frozenset(set(item.capabilities) | additions.get(symbol, set()))
        with_capabilities = replace(item, capabilities=capabilities)
        updated.append(
            replace(
                with_capabilities,
                has_marker=_marker_is_contract(with_capabilities, capabilities),
            )
        )
    return tuple(updated)


def _resolve_aliases(
    declarations: Sequence[PublicType],
    units: Sequence[_SourceUnit],
) -> tuple[PublicType, ...]:
    resolved = list(declarations)
    imports = _imports(units)
    for _ in range(len(resolved) + 1):
        changed = False
        symbols = {"::".join(filter(None, (item.module_path, item.name))): item for item in resolved}
        updated = []
        for item in resolved:
            if item.kind != "alias" or not item.target_name or item.capabilities:
                updated.append(item)
                continue
            target = _resolve_symbol(
                item.target_name,
                module_path=item.module_path,
                symbols=symbols,
                imports=imports,
            )
            if target is None:
                imported = imports.get((item.module_path, item.target_name))
                updated.append(replace(item, target_name=imported) if imported is not None else item)
                continue
            target_item = symbols[target]
            updated.append(
                replace(
                    item,
                    capabilities=target_item.capabilities,
                    target_name=target,
                    has_marker=item.has_marker or target_item.has_marker,
                )
            )
            changed = True
        resolved = updated
        if not changed:
            break
    return tuple(resolved)


def _module_exports(
    declarations: Sequence[PublicType],
    units: Sequence[_SourceUnit],
) -> dict[str, dict[str, str]]:
    symbols = {"::".join(filter(None, (item.module_path, item.name))): item for item in declarations}
    imports = _imports(units)
    exports: dict[str, dict[str, str]] = defaultdict(dict)
    for symbol, item in symbols.items():
        exports[item.module_path][item.name] = symbol
    uses = [use for unit in units for use in _unit_uses(unit, public_only=True)]
    edges = _module_edges(units)
    module_paths = {
        "",
        *(item.module_path for item in declarations),
        *(parent for parent, _, _ in edges),
        *(child for _, child, _ in edges),
    }
    for _ in range(len(uses) + len(module_paths) + 1):
        changed = False
        for module_path, expanded, _ in uses:
            reference, _, alias = expanded.partition("@AS@")
            if reference.endswith("::*"):
                target_reference = reference.removesuffix("::*")
                target_module = next(
                    (
                        candidate
                        for candidate in _reference_candidates(
                            target_reference,
                            module_path,
                        )
                        if candidate in module_paths
                    ),
                    None,
                )
                if target_module is None:
                    continue
                for exported_name, target in exports[target_module].items():
                    if exports[module_path].get(exported_name) != target:
                        exports[module_path][exported_name] = target
                        changed = True
                continue
            target = _resolve_symbol(
                reference,
                module_path=module_path,
                symbols=symbols,
                imports=imports,
            )
            if target is None and "::" in reference:
                target_module_ref, target_name = reference.rsplit("::", 1)
                for candidate in _reference_candidates(
                    target_module_ref,
                    module_path,
                ):
                    if target_name in exports[candidate]:
                        target = exports[candidate][target_name]
                        break
            if target is None:
                continue
            exported_name = alias or reference.rsplit("::", 1)[-1]
            if exports[module_path].get(exported_name) != target:
                exports[module_path][exported_name] = target
                changed = True
        if not changed:
            break
    return exports


def _cargo_manifest(crate_root: Path) -> dict[str, object]:
    with (crate_root / "Cargo.toml").open("rb") as handle:
        return tomllib.load(handle)


def _cargo_lib_name(manifest: dict[str, object]) -> str:
    package = manifest.get("package")
    if not isinstance(package, dict) or not isinstance(package.get("name"), str):
        raise AuditConfigurationError("Cargo manifest is missing package.name")
    library = manifest.get("lib")
    if isinstance(library, dict) and isinstance(library.get("name"), str):
        return library["name"]
    return package["name"].replace("-", "_")


def _default_cargo_features(manifest: dict[str, object]) -> frozenset[str]:
    features = manifest.get("features", {})
    if not isinstance(features, dict):
        return frozenset()
    pending = list(features.get("default", ()))
    enabled: set[str] = set()
    while pending:
        feature = pending.pop()
        if not isinstance(feature, str) or feature.startswith("dep:") or "/" in feature or feature in enabled:
            continue
        enabled.add(feature)
        pending.extend(features.get(feature, ()))
    return frozenset(enabled)


def scan_crate(crate_root: Path, *, crate: str, source_root: Path) -> tuple[PublicType, ...]:
    """Scan one crate with conservative module visibility and symbol resolution."""
    manifest = _cargo_manifest(crate_root)
    default_features = _default_cargo_features(manifest)
    lib_name = _cargo_lib_name(manifest)
    units = []
    for path in _rust_source_files(crate_root):
        source = path.read_text(encoding="utf-8")
        text = _mask_disabled_inline_modules(
            _mask_non_code(source),
            source,
            default_features,
        )
        base_module = _module_for_file(crate_root, path)
        _, crate_path = _crate_relative(path, source_root, crate)
        units.append(
            _SourceUnit(
                path=path,
                crate_path=crate_path,
                source=source,
                text=text,
                base_module=base_module,
                spans=_inline_module_spans(text, base_module),
                default_features=default_features,
            )
        )
    enabled_modules = _default_enabled_modules(units)
    declarations = [
        item
        for unit in units
        for item in _parse_unit_declarations(
            unit,
            crate=crate,
            source_root=source_root,
            default_features=default_features,
            lib_name=lib_name,
        )
    ]
    declarations = list(
        _apply_manual_impls(
            declarations,
            units,
            enabled_modules,
            default_features,
        )
    )
    declarations = list(_resolve_aliases(declarations, units))
    symbols = {"::".join(filter(None, (item.module_path, item.name))): item for item in declarations}
    reachable_modules = _reachable_modules(units)
    exports = _module_exports(declarations, units)
    visible_by_symbol: dict[str, PublicType] = {}
    synthetic_aliases: dict[tuple[str, str], PublicType] = {}
    for module_path in reachable_modules:
        for exported_name, target in exports[module_path].items():
            target_item = symbols[target]
            export_path = "::".join(filter(None, (module_path, exported_name)))
            if exported_name == target_item.name:
                previous = visible_by_symbol.get(target)
                export_paths = {export_path}
                if previous is not None:
                    export_paths.update(previous.export_paths)
                visible_by_symbol[target] = replace(
                    target_item,
                    export_paths=frozenset(export_paths),
                )
            else:
                synthetic_aliases[(module_path, exported_name)] = PublicType(
                    crate=crate,
                    path=target_item.path,
                    crate_path=target_item.crate_path,
                    line=target_item.line,
                    name=exported_name,
                    kind="alias",
                    capabilities=target_item.capabilities,
                    has_marker=target_item.has_marker,
                    module_path=module_path,
                    target_name=target,
                    export_paths=frozenset({export_path}),
                    lib_name=target_item.lib_name,
                )
    visible = [visible_by_symbol[symbol] for symbol in sorted(visible_by_symbol)]
    visible.extend(synthetic_aliases[key] for key in sorted(synthetic_aliases))
    return tuple(visible)


def scan_rust_file(path: Path, *, crate: str, source_root: Path) -> tuple[PublicType, ...]:
    """Scan a standalone fixture file without an external module graph."""
    source = path.read_text(encoding="utf-8")
    default_features = frozenset()
    text = _mask_disabled_inline_modules(
        _mask_non_code(source),
        source,
        default_features,
    )
    _, crate_path = _crate_relative(path, source_root, crate)
    unit = _SourceUnit(
        path=path,
        crate_path=crate_path,
        source=source,
        text=text,
        base_module="",
        spans=_inline_module_spans(text, ""),
        default_features=default_features,
    )
    declarations = _parse_unit_declarations(
        unit,
        crate=crate,
        source_root=source_root,
        lib_name=crate.replace("-", "_"),
    )
    declarations = _apply_manual_impls(declarations, (unit,), {""})
    return tuple(item for item in declarations if item.module_path == "")


def _product_crates(root: Path) -> tuple[tuple[str, Path], ...]:
    crates_root = root / "finstack-quant"
    crates = []
    if not crates_root.is_dir():
        raise AuditConfigurationError(
            f"audit root {root} does not contain the expected finstack-quant/ crate directory"
        )
    for candidate in crates_root.iterdir():
        if (
            candidate.is_dir()
            and candidate.name not in NON_PRODUCT_CRATES
            and (candidate / "Cargo.toml").is_file()
            and (candidate / "src").is_dir()
        ):
            crates.append((candidate.name, candidate))
    return tuple(sorted(crates))


def _rust_source_files(crate_root: Path) -> tuple[Path, ...]:
    files = []
    for path in (crate_root / "src").rglob("*.rs"):
        relative_parts = path.relative_to(crate_root / "src").parts
        if any(part in {"tests", "examples", "benches"} for part in relative_parts):
            continue
        if path.name in {"test.rs", "tests.rs"} or path.stem.endswith("_tests"):
            continue
        files.append(path)
    return tuple(sorted(files))


def audit_workspace(
    root: Path,
    *,
    allowlist: Sequence[ExceptionEntry] = REVIEWED_EXCEPTIONS,
) -> AuditReport:
    """Audit all product Rust crate source trees below ``root``."""
    root = root.resolve()
    if not root.is_dir():
        raise AuditConfigurationError(f"audit root does not exist or is not a directory: {root}")
    crates = _product_crates(root)
    if not crates:
        raise AuditConfigurationError(f"no product Rust crates found below {root / 'finstack-quant'}")
    declarations = tuple(
        declaration
        for crate, crate_root in crates
        for declaration in scan_crate(crate_root, crate=crate, source_root=root)
    )
    external_exports = {
        "::".join(
            filter(
                None,
                (
                    declaration.lib_name,
                    export_path,
                ),
            )
        ): declaration
        for declaration in declarations
        for export_path in declaration.export_paths
    }
    globally_resolved = []
    for declaration in declarations:
        if declaration.kind != "alias" or declaration.capabilities or not declaration.target_name:
            globally_resolved.append(declaration)
            continue
        target = external_exports.get(declaration.target_name.lstrip(":"))
        if target is not None:
            globally_resolved.append(
                replace(
                    declaration,
                    capabilities=target.capabilities,
                    has_marker=declaration.has_marker or target.has_marker,
                )
            )
        else:
            globally_resolved.append(declaration)
    declarations = tuple(globally_resolved)
    by_identity = {declaration.identity: declaration for declaration in declarations}
    exception_by_identity = {entry.identity: entry for entry in allowlist}
    if len(exception_by_identity) != len(allowlist):
        raise ValueError("serde audit exception identities must be unique")

    reviewed: list[ExceptionEntry] = []

    def qualified_registry_identity(
        identity: tuple[str, str, str],
    ) -> tuple[str, str, str, str]:
        crate, path, type_name = identity
        module_path = ExceptionEntry(
            crate=crate,
            path=path,
            type_name=type_name,
            category="registry",
            rationale="registry lookup",
            allowed_missing=frozenset(),
        ).resolved_module_path
        return crate, path, module_path, type_name

    one_way_identities = {
        *(entry.identity for entry in ONE_WAY_EXCEPTIONS),
        *(qualified_registry_identity(identity) for identity in MAINTAINED_ONE_WAY_OUTPUTS),
    }
    required_public_identities = {qualified_registry_identity(identity) for identity in REQUIRED_PUBLIC_TYPES}

    def contract_like(declaration: PublicType) -> bool:
        return (
            declaration.is_contract_like
            or (
                declaration.crate,
                declaration.crate_path,
                declaration.module_path,
                declaration.name,
            )
            in one_way_identities | required_public_identities
        )

    stale: list[StaleException] = []
    for entry in sorted(allowlist, key=lambda item: item.identity):
        declaration = by_identity.get(entry.identity)
        if declaration is None:
            stale.append(
                StaleException(
                    entry=entry,
                    reason="declaration-missing",
                    actual_missing=None,
                )
            )
            continue
        actual_missing = frozenset(CAPABILITIES) - declaration.capabilities
        if not contract_like(declaration):
            stale.append(
                StaleException(
                    entry=entry,
                    reason="not-contract-like",
                    actual_missing=actual_missing,
                )
            )
            continue
        if not entry.allowed_missing or entry.allowed_missing != actual_missing:
            stale.append(
                StaleException(
                    entry=entry,
                    reason="capability-set-changed",
                    actual_missing=actual_missing,
                )
            )
            continue
        reviewed.append(entry)

    diagnostics: list[Diagnostic] = []
    audited_crates = {crate for crate, _ in crates}
    for crate, crate_path, type_name in sorted(MAINTAINED_CONTRACTS):
        registry_identity = (crate, crate_path, type_name)
        identity = qualified_registry_identity(registry_identity)
        if crate in audited_crates and identity not in by_identity:
            diagnostics.append(
                Diagnostic(
                    crate=crate,
                    path=Path("finstack-quant") / crate / crate_path,
                    line=0,
                    type_name=type_name,
                    missing=tuple(
                        sorted(
                            MAINTAINED_REQUIRED_CAPABILITIES.get(
                                registry_identity,
                                CAPABILITIES,
                            )
                        )
                    ),
                )
            )
    for registry_identity, required in sorted(REQUIRED_PUBLIC_TYPES.items()):
        crate, crate_path, type_name = registry_identity
        identity = qualified_registry_identity(registry_identity)
        if crate in audited_crates and identity not in by_identity:
            diagnostics.append(
                Diagnostic(
                    crate=crate,
                    path=Path("finstack-quant") / crate / crate_path,
                    line=0,
                    type_name=type_name,
                    missing=tuple(sorted(required)),
                )
            )
    stale_entries = {item.entry for item in stale}
    for declaration in declarations:
        if not contract_like(declaration):
            continue
        maintained_identity = (
            declaration.crate,
            declaration.crate_path,
            declaration.name,
        )
        required = MAINTAINED_REQUIRED_CAPABILITIES.get(
            maintained_identity,
            REQUIRED_PUBLIC_TYPES.get(maintained_identity, CAPABILITIES),
        )
        missing = set(required) - declaration.capabilities
        exception = exception_by_identity.get(declaration.identity)
        if exception is not None and exception not in stale_entries:
            missing -= exception.allowed_missing
        if missing:
            diagnostics.append(
                Diagnostic(
                    crate=declaration.crate,
                    path=declaration.path,
                    line=declaration.line,
                    type_name=declaration.name,
                    missing=tuple(sorted(missing)),
                )
            )
    diagnostics.sort(key=lambda item: (item.crate, item.path.as_posix(), item.line, item.type_name))

    crate_names = sorted({crate for crate, _ in crates})
    summaries = []
    for crate in crate_names:
        crate_declarations = [item for item in declarations if item.crate == crate]
        summaries.append(
            CrateSummary(
                crate=crate,
                public_types=len(crate_declarations),
                contract_types=sum(contract_like(item) for item in crate_declarations),
                reviewed_exceptions=sum(item.crate == crate for item in reviewed),
                failures=(
                    sum(item.crate == crate for item in diagnostics) + sum(item.entry.crate == crate for item in stale)
                ),
            )
        )
    return AuditReport(
        declarations=tuple(
            sorted(declarations, key=lambda item: (item.crate, item.path.as_posix(), item.line, item.name))
        ),
        diagnostics=tuple(diagnostics),
        reviewed_exceptions=tuple(reviewed),
        stale_exceptions=tuple(stale),
        summaries=tuple(summaries),
        contract_identities=frozenset(item.identity for item in declarations if contract_like(item)),
    )


def _format_capabilities(declaration: PublicType) -> str:
    return ",".join(capability for capability in CAPABILITIES if capability in declaration.capabilities) or "none"


def print_report(report: AuditReport, *, verbose: bool) -> None:
    """Print diagnostics and stable per-crate counts."""
    if verbose:
        print("Contract-like public types:")
        for declaration in report.declarations:
            if declaration.identity in report.contract_identities:
                print(
                    f"  {declaration.path}:{declaration.line}: {declaration.name} [{_format_capabilities(declaration)}]"
                )
        print("Reviewed exceptions:")
        for entry in report.reviewed_exceptions:
            print(f"  {entry.crate}/{entry.path}: {entry.type_name} [{entry.category}] {entry.rationale}")
    for diagnostic in report.diagnostics:
        print(f"{diagnostic.path}:{diagnostic.line}: {diagnostic.type_name}: missing {', '.join(diagnostic.missing)}")
    for stale in report.stale_exceptions:
        actual = (
            "declaration-missing" if stale.actual_missing is None else ",".join(sorted(stale.actual_missing)) or "none"
        )
        allowed = ",".join(sorted(stale.allowed_missing)) or "none"
        entry = stale.entry
        print(
            f"STALE classification {entry.crate}/{entry.path}: {entry.type_name} "
            f"[{entry.category}] reason={stale.reason} actual_missing={actual} "
            f"allowed_missing={allowed}"
        )
    print("Per-crate summary:")
    for summary in report.summaries:
        print(
            f"  {summary.crate}: public={summary.public_types} "
            f"contract={summary.contract_types} exceptions={summary.reviewed_exceptions} "
            f"failures={summary.failures}"
        )
    print(
        f"Total: public={len(report.declarations)} "
        f"contract={sum(item.contract_types for item in report.summaries)} "
        f"exceptions={len(report.reviewed_exceptions)} "
        f"failures={len(report.diagnostics)} stale={len(report.stale_exceptions)}"
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repository root")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--report", action="store_true", help="show all gated types and reviewed exceptions")
    mode.add_argument("--check", action="store_true", help="fail on missing capabilities or stale exceptions")
    return parser


def main() -> int:
    """Run report mode by default or enforce the audit with ``--check``."""
    parser = build_parser()
    arguments = parser.parse_args()
    try:
        report = audit_workspace(arguments.root)
    except AuditConfigurationError as error:
        parser.error(str(error))
    print_report(report, verbose=arguments.report or not arguments.check)
    return int(arguments.check and report.failed)


if __name__ == "__main__":
    raise SystemExit(main())
