# Serde Stability Policy

This document is the contract for wire-format stability of the `finstack-quant`
workspace. It tells a downstream consumer (data warehouse, risk database,
Python / WASM pipeline) what is safe to persist and under what conditions
those persisted bytes must be upgraded.

## Status

Pre-1.0. Breaking changes across minor versions are possible. Every breaking
change must be documented in `CHANGELOG.md` and, for types tracked below,
gated by a bumped explicit version marker. Maintained contracts use one of
three forms: an envelope `schema` string with a version suffix, a numeric
`version`, or a numeric/string `schema_version`.

The operational contract catalog, migration recipes, schemas, and examples live
in [`CONTRACTS.md`](CONTRACTS.md). Keep both documents in sync whenever a
persisted top-level type changes.

## Scope

This policy applies to every Rust type in the workspace that:

1. Derives or hand-implements `serde::Serialize` and/or `serde::Deserialize`,
   AND
2. Is part of the public API (i.e. reachable from an item exported from the
   crate root), AND
3. Is intended to be persisted — i.e. written to Parquet / JSON / a database —
   rather than strictly for in-process inter-thread communication.

Types that are strictly intermediate (private module items, dev-only helpers,
in-process DTOs between Rust and a binding layer that always re-serializes in
one process) are outside this contract.

## Maintained contract matrix

`Strict missing` describes the bounded database-oriented loader, not raw
`serde_json::from_*`. Strict loaders always require an explicit version marker:
an envelope `schema`, numeric `version`, or numeric/string `schema_version`.

| Persisted contract | Marker | Current | Accepted | Ordinary missing-marker behavior | Strict missing-marker behavior |
|---|---|---:|---|---|---|
| `InstrumentEnvelope` | `schema = "finstack_quant.instrument/1"` | 1 | 1 | compatibility loaders accept the bare `{type,spec}` form | reject with `contract/envelope-required` |
| `CalibrationEnvelope` / `CalibrationResultEnvelope` | `schema = "finstack_quant.calibration/3"` | 3 | 3; exact legacy `finstack_quant.calibration` is read with a warning | serde requires a string but does not validate it by itself | reject missing; legacy marker yields `contract/version-legacy` |
| `MarketContextState` | numeric `version` | 2 | 1–2 | missing maps to legacy version 1 | reject with `contract/version-missing` |
| `FinancialModelSpec` | numeric `schema_version` | 2 | 1–2 | missing maps to legacy version 1 | reject; explicit v1 is migrated to v2 |
| `ScenarioEnvelope` | `schema = "finstack_quant.scenario/1"` | 1 | 1 | bare `ScenarioSpec` remains an in-process compatibility shape | reject with `contract/version-missing` |
| `FactorModelConfigEnvelope` | `schema = "finstack_quant.factor_model_config/1"` | 1 | 1 | bare `FactorModelConfig` remains an in-process compatibility shape | reject with `contract/version-missing` |
| `CreditFactorModel` | `schema_version = "finstack_quant.credit_factor_model/1"` | 1 | 1 | the field is required, but raw serde does not run semantic validation | reject missing or mismatched marker |
| `PortfolioMaterializationEnvelope` | `schema = "finstack_quant.portfolio_materialization/1"` | 1 | 1 | no compatibility form for this normalized contract | reject with `contract/version-missing` |

Scenario-template documents are internal registry inputs. Their missing
`schema` maps to legacy `finstack_quant.scenario_template/1`; this is not a
public persistence promise.

Versioned result outputs remain consumer-checked rather than strict inbound
database contracts:

| Result type | Marker | Current | Missing-marker behavior |
|---|---|---:|---|
| `ValuationResult` | numeric `schema_version` | 1 | ordinary serde defaults to 1 |
| `StatementResult` | numeric `schema_version` | 1 | ordinary serde defaults to 1 |
| `PortfolioResult` | numeric `schema_version` | 1 | ordinary serde defaults to 1 |
| `PortfolioOptimizationResult` | numeric `schema_version` | 1 | write-only canonical result shape; no general deserializer |
| `CreditFactorModel` | string `schema_version` | 1 | required; use its strict loader |

## The contract

For every in-scope type:

- **Additive changes are allowed** (new `Option<T>` field, new enum variant)
  as long as:
  - The new field is annotated `#[serde(default)]` or `#[serde(default =
    "…")]`, AND
  - Deserializing an older payload produces a value that is semantically
    equivalent to the pre-change behavior, AND
  - The change is recorded in `CHANGELOG.md`.

  For a strict persisted contract using `deny_unknown_fields`, an additive
  field is forward-incompatible with an older reader. It therefore requires
  either a version bump or a coordinated reader-first rollout in which every
  reader accepts the field before any writer emits it.

- **Non-additive changes** (rename a field, change a field type, reorder or
  remove an enum variant, tighten a validation invariant) require:
  - An explicit version-marker bump on the type or envelope (see below), AND
  - A `CHANGELOG.md` entry explaining the migration path, AND
  - A migration helper (either a `serde(alias = "…")` for the simple rename
    case, or a `From<OldShape> for NewShape` helper for complex cases).

- **Field renames MUST prefer `#[serde(alias = "old_name")]`** over a hard
  rename when the old name has ever shipped. Do not silently rename.

- **Enum variant additions MUST NOT change existing variant discriminants or
  tag values.** Add new variants at the end.

- **`#[non_exhaustive]` on a public error enum or result type** is expected
  unless there is a specific reason not to — this is the workspace default.

### Strict rollout rules

1. Add or upgrade strict readers first.
2. Verify old and new fixtures against the supported-version matrix.
3. Deploy writers only after all persisted-data readers understand the new
   shape.
4. Reject unknown, zero, malformed, and future versions. Never infer current
   version from a missing marker on a strict path.
5. Preserve old tags and aliases during the documented migration window.
6. Bump the contract version for required-field changes, semantic changes, and
   additive fields that cannot use a coordinated reader-first rollout.

## Canonical JSON and content hashes

Canonical algorithm version `c1` is implemented by
`finstack_quant_core::canonical`:

- Serialize the typed value once and reject every non-finite `f32` or `f64`.
- Sort every JSON object recursively by UTF-8 key bytes. Preserve array order.
- Emit compact JSON with no insignificant whitespace.
- Preserve JSON integers. Finite floats use serde_json/Ryu's shortest
  representation.
- Preserve strings, producer date/enum conventions, decimal strings, and
  serde-omitted fields exactly. Canonicalization does not perform domain
  normalization.
- Include extension and `meta` maps in the bytes and digest.

`content_hash(value)` hashes this exact preimage:

```text
b"finstack-canon/" || b"c1" || b"\0" || canonical_json_bytes
```

The result is `sha256:` followed by 64 lowercase hexadecimal digits.
`CANONICAL_VERSION` is a separate manifest/cache-key axis, so a future
canonical algorithm can coexist with SHA-256 identifiers.

Decimal scale is intentionally not normalized globally: producer strings
`"100.0"` and `"100"` hash differently even when their decimal values compare
equal. New envelope producers should normalize decimals before hashing when
cross-producer identity is required. Legacy payloads hash as produced.

## Schema-versioned result and model types

The following result and model types carry an explicit numeric or string
version marker so consumers can detect a mismatch and refuse, upgrade, or fall
back rather than silently misinterpreting bytes. The corresponding constant or
descriptor lives in the same module and is the source of truth.

| Type | Module | Const | Current version |
|---|---|---|---|
| `ValuationResult` | `finstack_quant_valuations::results` | `VALUATION_RESULT_SCHEMA_VERSION` | 1 |
| `StatementResult` | `finstack_quant_statements::evaluator::results` | `STATEMENT_RESULT_SCHEMA_VERSION` | 1 |
| `PortfolioResult` | `finstack_quant_portfolio::results` | `PORTFOLIO_RESULT_SCHEMA_VERSION` | 1 |
| `PortfolioOptimizationResult` | `finstack_quant_portfolio::optimization::result` | `PORTFOLIO_OPTIMIZATION_RESULT_SCHEMA_VERSION` | 1 |
| `CreditFactorModel` | `finstack_quant_factor_model::credit::hierarchy` | `"finstack_quant.credit_factor_model/1"` (string tag, not a `u32` const) | 1 |
| `MarketContextState` | `finstack_quant_core::market_data::context` | `MARKET_CONTEXT_STATE_VERSION` | 2 |
| `FinancialModelSpec` | `finstack_quant_statements::types` | `CURRENT_SCHEMA_VERSION` | 2 |

### When to bump an explicit version marker

Bump (i.e. increment the `const`) in any of these cases:

- A required field is removed or renamed without `#[serde(alias)]`.
- A field's serialized type changes (`f64` → `Money`, `String` → `enum`).
- A field's semantic meaning changes (same name, different interpretation).
- An enum variant is removed or its tag value changes.
- A validation invariant is tightened such that older-serialized values would
  now round-trip-fail (e.g. a field gains a `deny_unknown_fields` sibling).

Do NOT bump for:

- Adding a new field with `#[serde(default)]` when the strict reader-first
  rollout rule above is satisfied.
- Adding a new enum variant at the end.
- Adding a new `impl` block or deriving a new trait.
- Documentation changes.
- Internal refactors that don't touch the serialized shape.

### How to bump

1. Increment the corresponding `*_SCHEMA_VERSION` const in the owning module.
2. If the change is non-trivial, add a `pub fn upgrade_v{N}_to_v{N+1}(old:
   serde_json::Value) -> crate::Result<serde_json::Value>` helper next to the
   type so downstream tools can migrate persisted payloads.
3. Record the bump in `CHANGELOG.md` under `### Changed`, referencing:
   - The old and new version numbers.
   - The semantic change.
   - The migration path (or that old payloads now fail to deserialize, with an
     error type the consumer can match).

### How consumers should read versioned payloads

```rust
use finstack_quant_valuations::results::{
    ValuationResult, VALUATION_RESULT_SCHEMA_VERSION,
};

let payload: ValuationResult = serde_json::from_str(&bytes)?;
if payload.schema_version > VALUATION_RESULT_SCHEMA_VERSION {
    // Refuse: binary is older than data. Upgrade finstack-quant, don't plow through.
    return Err(/* forward-incompatible error */);
}
// payload.schema_version < CURRENT is handled by `#[serde(default)]` and any
// `alias`es / migration helpers the type provides.
```

## Types outside the schema-versioned set

Everything else under `pub` serde types in the workspace follows the
"additive changes only between minor versions" rule, but does not (yet) carry
an explicit version marker. If you persist them, pin a specific workspace version in your
consumer or be prepared to handle deserialization errors on upgrade.

Notable in this category:

- `finstack_quant_valuations::results::ValuationDetails`
  (enum of structured pricing details; variants may be added)
- `finstack_quant_portfolio::valuation::PortfolioValuation`
  (sub-envelope of `PortfolioResult`)
- `finstack_quant_portfolio::factor_model::whatif::{WhatIfResult, StressResult}`
  (no versioning yet — track upstream)
- Bare `PortfolioSpec`, `ScenarioSpec`, and `FactorModelConfig` compatibility
  shapes. Persist their versioned envelope forms for strict storage.
- User-authored nested `*Spec` and `*Config` values that are not one of the
  top-level contracts in the maintained matrix.

### Credit factor hierarchy types (additive, no schema-version constant)

The following types were introduced with the credit factor hierarchy feature.
They follow the additive-only rule; new `Option<T>` fields may be added between
minor versions without a schema-version bump.

- `CreditFactorAttribution` (`finstack_quant_attribution::credit_factor`) —
  additive, opt-in field on `PnlAttribution`; deserializing an older payload
  (missing field) produces `None`.
- `CreditCarryDecomposition` (`finstack_quant_attribution::credit_factor`) —
  additive, opt-in field on `PnlAttribution`; same rule as above.
- `SourceLine` (`finstack_quant_attribution::credit_factor`) — custom
  `Deserialize`: accepts both legacy `Money` shape and new tagged shape for
  backward compatibility.
- `PositionResidualContribution` (`finstack_quant_portfolio::factor_model`) —
  additive, opt-in field on `RiskDecomposition`.
- `CreditCalibrationInputs`, `CreditCalibrationConfig`
  (`finstack_quant_factor_model::credit::calibration`) — round-trippable nested
  inputs without an independent top-level marker; persist them inside a
  versioned owning artifact or pin the workspace version.

Persist these nested types only inside a versioned owning artifact, or pin the
workspace version in the consumer. If one becomes an independently persisted
top-level contract, introduce an explicit version marker, strict loader,
canonical fixture, and migration policy before shipping that persistence
surface.

## MSRV and toolchain

- Workspace `rust-version = "1.90"` (see root `Cargo.toml`).
- MSRV bumps are allowed in a minor release and must be recorded in
  `CHANGELOG.md`. A consumer pinning an older toolchain should pin the
  workspace version accordingly.

## What is NOT covered

- **Python wheel ABI** across Python minor versions. PyO3 and the Python
  ABI handle that.
- **WASM binary layout** across `wasm-bindgen` versions. Regenerate the `pkg/`
  output whenever the Rust side changes.
- **In-memory layout of Rust structs.** `#[repr(Rust)]` is the default and
  layout is not a stability contract. Do not `mem::transmute` finstack types.
- **Generic Criterion scratch output.** The named portfolio-materialization
  baseline manifest and release result record are maintained acceptance
  artifacts documented in
  [`MATERIALIZATION_BENCHMARKS.md`](MATERIALIZATION_BENCHMARKS.md).

## Getting clarity

If a change you're about to make seems ambiguous under this policy, the
default is: treat it as breaking, bump the schema version, document it in
`CHANGELOG.md`, and add a migration note. Consumers can't un-persist bad
assumptions.
