# Persisted Contracts

This catalog is the operational reference for database-neutral JSON contracts
owned by Finstack Quant. Rust owns bounded parsing, version checks, migration,
semantic validation, canonicalization, hashing, and runtime construction.
Database schemas, SQL migrations, orchestration, and storage adapters belong to
the external Scaffold project.

Compatibility policy is defined in
[`SERDE_STABILITY.md`](SERDE_STABILITY.md). Checked-in generated schemas are the
source-derived structural publication artifacts and are drift-checked against
their Rust types. Strict Rust loaders remain authoritative for trusted ingress
because they additionally enforce versions, resource limits, migrations, and
semantic invariants.

## Checked-in contract matrix

| Contract | Current marker | Supported input | Strict Rust entry point | Schema status / artifact | Canonical fixture |
|---|---|---|---|---|---|
| Instrument | `finstack_quant.instrument/1` | v1 envelope only; compatibility APIs also accept bare tagged instruments | `InstrumentEnvelope::from_slice_strict` | generated + drift-checked: [instrument union and per-type schemas](../finstack-quant/valuations/schemas/instruments/1/) | [v1](../finstack-quant/valuations/tests/data/canonical/instrument.json) |
| Calibration request | `finstack_quant.calibration/3` | v3; exact unversioned legacy marker accepted with warning | `CalibrationEnvelope::from_slice_strict` | generated + drift-checked: [v3 request schema](../finstack-quant/valuations/schemas/calibration/3/calibration.schema.json) | [v3](../finstack-quant/valuations/tests/data/canonical/calibration.json) |
| Calibration result | `finstack_quant.calibration/3` | v3; exact unversioned legacy marker accepted with warning | `CalibrationResultEnvelope::from_slice_strict` | derived but no standalone artifact | none |
| Market context state | numeric `version = 2` | versions 1–2 | `MarketContext::from_state_slice` | derived but no standalone artifact; typed where embedded, including the [attribution schema](../finstack-quant/attribution/schemas/attribution/1/attribution.schema.json) | [v2](../finstack-quant/core/tests/data/canonical/market_context_state.json) |
| Financial model | numeric `schema_version = 2` | versions 1–2; v1 upgrades during strict load | `FinancialModelSpec::from_slice_strict` | generated + drift-checked: [financial model schema](../finstack-quant/statements/schemas/statements/1/financial_model_spec.schema.json) | [v2](../finstack-quant/statements/tests/data/canonical/financial_model.json) |
| Scenario | `finstack_quant.scenario/1` | v1 envelope | `ScenarioEnvelope::from_slice_strict` | generated + drift-checked: [scenario schema](../finstack-quant/scenarios/schemas/scenarios/1/scenario.schema.json) | [v1](../finstack-quant/scenarios/tests/data/canonical/scenario.json) |
| Factor-model configuration | `finstack_quant.factor_model_config/1` | v1 envelope | `FactorModelConfigEnvelope::from_slice_strict` | generated + drift-checked: [factor-model configuration schema](../finstack-quant/factor-model/schemas/factor_model/1/factor_model_config.schema.json) | [v1](../finstack-quant/factor-model/tests/data/canonical/factor_model_config.json) |
| Credit factor model | `finstack_quant.credit_factor_model/1` in `schema_version` | v1 artifact | `CreditFactorModel::from_slice_strict` | generated + drift-checked: [credit factor model schema](../finstack-quant/factor-model/schemas/factor_model/1/credit_factor_model.schema.json) | [v1](../finstack-quant/factor-model/tests/data/canonical/credit_factor_model.json) |
| Portfolio materialization | `finstack_quant.portfolio_materialization/1` | v1 envelope | `Portfolio::from_materialization` | generated + drift-checked: [portfolio materialization schema](../finstack-quant/portfolio/schemas/portfolio/1/portfolio_materialization.schema.json) | [v1](../finstack-quant/portfolio/tests/data/canonical/portfolio_materialization.json) |

### Consumer-checked versioned result outputs

These outputs remain consumer-checked contracts:

| Result output | Current marker | Read contract | Schema status / artifact |
|---|---|---|---|
| `ValuationResult` | numeric `schema_version = 1` | ordinary serde defaults a missing marker to 1 | generated + drift-checked: [valuation result schema](../finstack-quant/valuations/schemas/results/1/valuation_result.schema.json) |
| `StatementResult` | numeric `schema_version = 1` | ordinary serde defaults a missing marker to 1 | generated + drift-checked: [statement result schema](../finstack-quant/statements/schemas/statements/1/statement_result.schema.json) |
| `PortfolioResult` | numeric `schema_version = 1` | ordinary serde defaults a missing marker to 1 | derived but no standalone artifact |
| `PortfolioOptimizationResult` | numeric `schema_version = 1` | serialize-only; no general deserializer | serialize-only; manual `JsonSchema` implementation; no standalone artifact |

See the compatibility details in the
[consumer-checked versioned result matrix](SERDE_STABILITY.md#consumer-checked-versioned-result-outputs).

Strict loaders reject missing, malformed, zero, unrelated, and future markers.
They enforce [`LoadLimits`](../finstack-quant/core/src/contract/limits.rs) and
return bounded
[`ValidationReport`](../finstack-quant/core/src/contract/diagnostics.rs)
findings. Raw serde is a compatibility mechanism, not a database trust
boundary.

### Generated serde contracts without bounded loaders

`AttributionEnvelope` and `AttributionResultEnvelope` both require the exact
`finstack_quant.attribution/1` marker during ordinary serde deserialization;
missing and mismatched markers fail. Their generated request and result schemas
use the same marker constant. They do not currently provide a separate bounded
strict loader, so callers handling untrusted bytes must impose their own input
limits before deserialization.

The [margin schema](../finstack-quant/margin/schemas/margin/1/margin.schema.json)
is a synthetic `oneOf` bundle for `OtcMarginSpec`, `CsaSpec`, and `MarginCall`.
There is no corresponding margin envelope type or strict root loader. The
schema's MPOR, maturity, haircut, concentration, default-haircut, and
notification-hour constraints are enforced by the nested Rust types during
ordinary serde serialization and deserialization. This prevents public struct
literals and permissive constructors from emitting JSON that those same types
cannot read back.

## Migration recipes

Apply migrations before canonical hashing. Retain the original bytes and
revision ID until the upgraded artifact has passed its strict loader.

### FinancialModelSpec v1 to v2

If `schema_version` is absent, stamp `1`; never stamp the current version onto
an unknown historical payload. The strict loader performs the supported
v1-to-v2 migration and semantic validation:

```rust
use finstack_quant_core::LoadLimits;
use finstack_quant_statements::FinancialModelSpec;

let mut document: serde_json::Value = serde_json::from_slice(source)?;
document
    .as_object_mut()
    .ok_or("financial model root must be an object")?
    .entry("schema_version")
    .or_insert(serde_json::json!(1));
let bytes = serde_json::to_vec(&document)?;
let (model, report) =
    FinancialModelSpec::from_slice_strict(&bytes, &LoadLimits::default())?;
```

For an explicit migration tool, call
`finstack_quant_statements::types::upgrade_v1_to_v2` after stamping version 1.
The helper converts historical tagged debt variants to the registry
`{type,spec}` shape and sets `schema_version` to 2. Untagged generic debt must
match one of the supported historical instrument types; otherwise add its
canonical registry tag before retrying.

### Unversioned calibration marker to v3

For an envelope already using the v3 flat `market_data` / `prior_market`
shape, replace exactly:

```json
{"schema":"finstack_quant.calibration"}
```

with:

```json
{"schema":"finstack_quant.calibration/3"}
```

The strict request and result loaders temporarily accept the unversioned marker
and emit `contract/version-legacy`. A historical v2 request containing
`initial_market` is not a one-line migration: convert that field to the v3 flat
inputs first, remove `initial_market`, then stamp `/3`. Validate the result with
`CalibrationEnvelope::from_slice_strict`.

### Bare instrument to envelope

Wrap the complete bare tagged value without modifying its `type` or `spec`:

```json
{
  "schema": "finstack_quant.instrument/1",
  "instrument": {
    "type": "fx_spot",
    "spec": {}
  }
}
```

`spec` above denotes the original complete spec object. Validate the finished
document against the per-type schema and
`InstrumentEnvelope::from_slice_strict`; do not persist the abbreviated
illustration.

### PortfolioSpec to materialization envelope

Load the portable spec once, build the runtime portfolio, then let Rust produce
the normalized envelope:

```rust
use finstack_quant_core::canonical::to_canonical_bytes;
use finstack_quant_portfolio::{Portfolio, PortfolioSpec};

let spec: PortfolioSpec = serde_json::from_slice(source)?;
let portfolio = Portfolio::from_spec(spec)?;
let envelope = portfolio.to_materialization()?;
let canonical_bytes = to_canonical_bytes(&envelope)?;
```

`to_materialization` deduplicates instruments by canonical content hash and
emits content-addressed artifact IDs. It fails if a runtime instrument cannot
produce registry JSON; it never writes a lossy `null` instrument. Validate a
persisted result with `Portfolio::from_materialization`.

### MarketContextState without version

Stamp numeric version 1, then use the strict restore path:

```rust
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::LoadLimits;

let mut document: serde_json::Value = serde_json::from_slice(source)?;
document
    .as_object_mut()
    .ok_or("market context root must be an object")?
    .entry("version")
    .or_insert(serde_json::json!(1));
let bytes = serde_json::to_vec(&document)?;
let (market, report) = MarketContext::from_state_slice(&bytes, &LoadLimits::default())?;
```

Version 1 represents the pre-hierarchy snapshot. Do not stamp 2 merely because
the current writer emits 2.

## Canonical JSON examples

Every top-level contract family has a compact canonical JSON fixture, a
checked-in domain-separated `content_hash` identity, and an owning test that
compares exact bytes and identities. The `.sha256` files contain the
`sha256:<hex>` identity over the `finstack-canon/c1\0` preimage; they are not
ordinary SHA-256 digests of the fixture bytes:

- [Instrument v1 fixture](../finstack-quant/valuations/tests/data/canonical/instrument.json),
  [identity](../finstack-quant/valuations/tests/data/canonical/instrument.sha256),
  and [test](../finstack-quant/valuations/tests/canonical_contracts.rs).
- [Calibration v3 fixture](../finstack-quant/valuations/tests/data/canonical/calibration.json),
  [identity](../finstack-quant/valuations/tests/data/canonical/calibration.sha256),
  and [test](../finstack-quant/valuations/tests/canonical_contracts.rs).
- [MarketContextState v2 fixture](../finstack-quant/core/tests/data/canonical/market_context_state.json),
  [identity](../finstack-quant/core/tests/data/canonical/market_context_state.sha256),
  and [test](../finstack-quant/core/tests/contract/canonical.rs).
- [FinancialModelSpec v2 fixture](../finstack-quant/statements/tests/data/canonical/financial_model.json),
  [identity](../finstack-quant/statements/tests/data/canonical/financial_model.sha256),
  and [test](../finstack-quant/statements/tests/canonical_contract.rs).
- [ScenarioEnvelope v1 fixture](../finstack-quant/scenarios/tests/data/canonical/scenario.json),
  [identity](../finstack-quant/scenarios/tests/data/canonical/scenario.sha256),
  and [test](../finstack-quant/scenarios/tests/canonical_contract.rs).
- [FactorModelConfigEnvelope v1 fixture](../finstack-quant/factor-model/tests/data/canonical/factor_model_config.json),
  [identity](../finstack-quant/factor-model/tests/data/canonical/factor_model_config.sha256),
  and [test](../finstack-quant/factor-model/tests/canonical_contract.rs).
- [CreditFactorModel v1 fixture](../finstack-quant/factor-model/tests/data/canonical/credit_factor_model.json),
  [identity](../finstack-quant/factor-model/tests/data/canonical/credit_factor_model.sha256),
  and [test](../finstack-quant/factor-model/tests/canonical_contract.rs).
- [PortfolioMaterializationEnvelope v1 fixture](../finstack-quant/portfolio/tests/data/canonical/portfolio_materialization.json),
  [identity](../finstack-quant/portfolio/tests/data/canonical/portfolio_materialization.sha256),
  and [test](../finstack-quant/portfolio/tests/materialization.rs).

The owning tests derive these bytes through `to_canonical_bytes` and their
identities through `content_hash`. Pretty-printed examples are authoring views,
not canonical byte representations.

## Structured diagnostics

`Diagnostic` fields are stable persisted names:

| Field | Meaning |
|---|---|
| `code` | stable machine classification such as `contract/version-unsupported` |
| `phase` | `parse`, `version`, `migrate`, `structure`, `semantic`, `canonicalize`, `hash`, or `build` |
| `severity` | `error` or `warning` |
| `pointer` | RFC 6901 JSON Pointer when available |
| `message` | human-readable detail; do not parse it for classification |
| `contract` | stable contract ID without version suffix |
| `expected_version`, `actual_version` | numeric version context |
| `artifact_hash`, `revision_id` | content/storage identity context |
| `instrument_id`, `position_id` | domain identity context |

`ValidationReport` is:

```json
{
  "diagnostics": [
    {
      "actual_version": 2,
      "artifact_hash": null,
      "code": "contract/version-unsupported",
      "contract": "finstack_quant.instrument",
      "expected_version": 1,
      "instrument_id": null,
      "message": "unsupported contract version",
      "phase": "version",
      "pointer": "/schema",
      "position_id": null,
      "revision_id": null,
      "severity": "error"
    }
  ],
  "truncated": false
}
```

The source-generated Draft 2020-12 schemas are
[`diagnostic.schema.json`](../finstack-quant/valuations/schemas/common/1/diagnostic.schema.json)
and
[`validation_report.schema.json`](../finstack-quant/valuations/schemas/common/1/validation_report.schema.json).
Python raises `ContractValidationError` and exposes diagnostic dictionaries on
`exc.report`. JavaScript throws `ContractValidationError` with `kind` and, for
report failures, `error.report`.

## Materialization APIs and performance

Rust APIs are defined in
[`portfolio/src/materialization`](../finstack-quant/portfolio/src/materialization).
Python declarations are in
[`portfolio/__init__.pyi`](../finstack-quant-py/finstack_quant/portfolio/__init__.pyi);
WASM declarations are in
[`index.d.ts`](../finstack-quant-wasm/index.d.ts), with the facade in
[`exports/portfolio.js`](../finstack-quant-wasm/exports/portfolio.js).

The reproducible fixture protocol, timing boundaries, hardware/toolchain,
absolute gates, baseline provenance, and reference results are in
[`MATERIALIZATION_BENCHMARKS.md`](MATERIALIZATION_BENCHMARKS.md). The full
source-backed machine record is
[`materialization-benchmark-results.json`](materialization-benchmark-results.json),
and baseline provenance is
[`materialization-benchmark-baseline.json`](materialization-benchmark-baseline.json).

The maintained protocol uses 5,000-position fixtures: A has 5,000 unique
artifacts and B has 50. Cold samples start with a fresh sized cache; warm
samples use a pre-populated cache. Release measurements use at least 100 calls
and nearest-rank median/p95; fixture generation, compilation, startup, file
reads, cache construction, network, database I/O, market construction, and
pricing are outside the timer. Regression comparisons gate at 10%, and native
cold-A p95 must remain below 1 second.

The 2026-07-29 reference record was captured on macOS 26.5.2 arm64, Apple M5
Max (18 physical/logical cores), rustc 1.91.1, CPython 3.12.13, and Node
24.16.0. Exact tool versions, raw samples, phase timings, confidence intervals,
fixture digests, and commands are source-backed by the machine record linked
above.

## MarketContextState schema publication

`MarketContextState` schema derivation is complete (`JsonSchema`), but no
standalone file is maintained. Generated contracts that embed the type
directly, including the
[attribution request schema](../finstack-quant/attribution/schemas/attribution/1/attribution.schema.json),
publish its typed curve, FX, scalar, time-series, surface, dividend,
credit-index, volatility-cube, and hierarchy fields. The strict bounded loader
remains authoritative for semantic checks, cross-references, resource limits,
and supported-version handling that JSON Schema cannot express.

## Scaffold and PRD supersession

The external Scaffold project owns database and service concerns. In
[`FINSTACK_DATA_PLATFORM_PRD.md`](FINSTACK_DATA_PLATFORM_PRD.md), the following
requirements are superseded for this repository:

- §6.1 crate decomposition for PostgreSQL, DuckDB, Turso, service, Python, and
  WASM data packages;
- §13 database adapter requirements;
- §14.2 native PostgreSQL/DuckDB access in the Python facade;
- §§15.2–15.4 pandas conversion ownership;
- §22 milestones B–D database-adapter work.

The PRD remains applicable here for §11.7 determinism, §12 artifact coverage,
§16 materialization performance, and the quant-library portions of §20
integration prerequisites. The implementation boundary is recorded in the
[JSON/serde readiness plan](superpowers/plans/2026-07-26-json-serde-db-readiness.md);
that plan and the PRD are historical/design inputs, while this catalog and the
checked-in source are the maintained contract.

## Generation and drift checks

Run:

```bash
mise run rust-gen-schemas
mise run rust-check-schemas
mise run wasm-gen-bindings
mise run gen-check
```

`rust-check-schemas` runs the public serde/`JsonSchema` audit and focused schema
parity suites for cashflows, portfolio materialization, margin, attribution,
statements, scenarios, factor-model, and valuations. `rust-gen-schemas`
registers the owning generators, including the factor-model, scenarios, and
statements trees; factor-model schemas are owned by
[`factor-model/schemas`](../finstack-quant/factor-model/schemas/), not
`valuations`.

`gen-check` regenerates schemas and TypeScript artifacts and compares the
manifested content/path digest before and after generation. It detects
second-run drift without mistaking intended uncommitted generated outputs for
drift. CI runs the same task.
