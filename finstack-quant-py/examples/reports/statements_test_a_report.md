# Test A — Financial Statements

Decision: **CONDITIONAL**

The model and normalization contracts are mechanically ready. Empirical
`AdjustmentValue` coverage remains pending Test B.

## A1 — Round-trip validity

- Status: **PASS**
- Model: `40` nodes across `3` periods
- Adjustments: `2`
- `FinancialModelSpec` and `NormalizationConfig` both round-trip structurally
- Both documents validate against their checked-in Draft 2020-12 schemas
- Rust evaluation and `NormalizationEngine::normalize` complete cleanly

| Period | Reported EBITDA | Fixed applied | Percentage applied | Final EBITDA |
|---|---:|---:|---:|---:|
| 2025Q1 | 170.0 | 17.0 | 10.0 | 197.0 |
| 2025Q2 | 200.0 | 20.0 | 11.0 | 231.0 |
| 2025Q3 | 200.0 | 20.0 | 11.0 | 231.0 |

## A2 — Adjustment schema

- Status: **PASS**
- Five input types derive `JsonSchema`
- Artifact: `finstack-quant/statements/schemas/statements/1/normalization_config.schema.json`
- Supported variants: `fixed`, `percentage_of_node`

## A3 — Variant measurement

- Status: **CONDITIONAL**
- Evidence source: **synthetic dry run**
- Unsupported synthetic share: **4 / 10 (40.0%)**
- Threshold result: `provisional_trigger` because `40.0% > 30%`
- Production decision: `pending_test_b`

The synthetic percentage is not an observed corpus result. If Test B measures
the real unsupported share above 30%, add canonical Rust run-rate or
annualization semantics before M4 freezes the contract.

## Root-cause actions

- Published the missing Rust-owned normalization schema.
- Preserved adjustments as a sidecar rather than changing `FinancialModelSpec`.
- Kept financial logic in Rust.
- Corrected stale notebook claims about variants and cap defaults.

## A4 — Quant-contract readiness attestation

Decision: **CONDITIONAL**

Signer: **Jon**

Signed on: **2026-08-02**
