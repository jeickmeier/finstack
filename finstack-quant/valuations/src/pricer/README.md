# pricer

The dispatch core of the valuations crate. It owns the three key types
(`InstrumentType`, `ModelKey`, `PricerKey`), the `Pricer` trait, the
`PricerRegistry` that maps a `(instrument, model)` pair to one pricer
implementation, and the JSON entry points the Python and WASM bindings call.

Every registry-dispatched result runs `price_with_metrics_impl`, reached through
`PricerRegistry::price_with_metrics` (direct callers), `price_with_metrics_shared`
(the instrument-side wrapper behind `Instrument::price_with_metrics`, which only
resolves the model and the registry before calling in here), or `price_batch`.
The ordering rules documented below — validate, resolve `as_of`, price unshocked,
stamp metadata, apply the scenario shock exactly once, enrich with metrics — are
therefore the whole pricing contract for anything reached by a `ModelKey`.

Two public pricing surfaces sit outside that contract and must not be assumed to
follow it: `pricer::cos` (a standalone Fourier numeric surface, never registered)
and `StructuredCredit::price_stochastic`, which runs its own
`ValidatedPricingLifecycle` and returns a `StochasticPricingResult` without a
registry lookup. The `pub(crate)` `PricerRegistry::price_raw` is a third
exception: it shares the dispatch table and the validate / resolve-`as_of` /
apply-shock steps, but returns an unrounded `f64` and never stamps metadata or
enriches with metrics.

## Position in the stack

Consumes `crate::instruments` (the `Instrument` trait, `PricingOptions`,
`InstrumentEnvelope`, the concrete instrument types it registers pricers for),
`crate::metrics` (`MetricId`, `MetricRegistry`), `crate::results`
(`ValuationResult`), and `finstack_quant_core` (`MarketContext`, `Date`,
`Money`, `FinstackConfig`). `crate::instruments` depends back on this module for
the key types and `PricingError`, so the two are mutually recursive by design;
the split is by role, not by dependency layer.

Not to be confused with `finstack_quant_monte_carlo::pricer`
(`EuropeanPricer`, `LsmcPricer`, `basis`, `path_dependent`) — a different module
in a different crate, reached by the MC instrument pricers.

## Layout

| Path | Contents | Visibility |
|------|----------|------------|
| [`mod.rs`](mod.rs) | `register_all_pricers`, `build_standard_registry`, the `standard_registry()` `OnceLock` singleton, the `register_generic!` macro, all re-exports | mixed |
| [`keys.rs`](keys.rs) | `InstrumentType`, `ModelKey`, `PricerKey` — enums, `as_str`, `Display`, `FromStr`, serde | private mod, types re-exported |
| [`registry.rs`](registry.rs) | `Pricer` trait, `PricerRegistry`, the pricing pipeline, `expect_inst`, FX-policy collection | private mod, items re-exported |
| [`errors.rs`](errors.rs) | `PricingError`, `PricingErrorContext`, the two lossy conversions to/from `finstack_quant_core::Error` | private mod, types re-exported |
| [`enrichment.rs`](enrichment.rs) | Metric enrichment applied after the base PV | `pub(super)`, fully internal |
| [`json.rs`](json.rs) | Envelope parse/validate, model + metric discovery, the JSON pricing entry points | `pub mod` |
| [`structured_credit_json.rs`](structured_credit_json.rs) | Five structured-credit tranche analytics that the generic metric registry cannot carry | `pub mod` |
| [`cos.rs`](cos.rs) | Fang-Oosterlee COS Fourier pricer (`CosConfig`, `CosPricer`, `bs_cos_price`, `vg_cos_price`, `merton_jump_cos_price`) | `pub mod` |
| [`rates.rs`](rates.rs) | Registrations: Bond, Irs, Fra, BasisSwap, Deposit, InterestRateFuture, BondFuture, CapFloor, Swaption, Repo, Dcf | private mod |
| [`credit.rs`](credit.rs) | Registrations: Cds, CdsIndex, CdsTranche, CdsOption, StructuredCredit | private mod |
| [`equity.rs`](equity.rs) | Registrations: Equity, EquityFuture, EquityTotalReturnFuture, EquityOption, EquityTotalReturnSwap, VarianceSwap, VolatilityIndexFuture, RealEstateAsset, LeveredRealEstateEquity, PrivateMarketsFund | private mod |
| [`fx.rs`](fx.rs) | Registrations: FxSpot, FxFuture, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, Ndf, FxBarrierOption, FxDigitalOption, FxTouchOption | private mod |
| [`fixed_income.rs`](fixed_income.rs) | Registrations: FiIndexTotalReturnSwap, Convertible, InflationLinkedBond, RevolvingCredit, TermLoan, AgencyMbsPassthrough, AgencyTba, DollarRoll, AgencyCmo | private mod |
| [`inflation.rs`](inflation.rs) | Registrations: InflationSwap, YoYInflationSwap, InflationCapFloor | private mod |
| [`exotics.rs`](exotics.rs) | Registrations: Basket, AsianOption, BarrierOption, LookbackOption, QuantoOption, Autocallable, CmsOption/Swap/SpreadOption, CliquetOption, RangeAccrual, CallableRangeAccrual, Tarn, Snowball, BermudanSwaption | private mod |
| [`commodity.rs`](commodity.rs) | Registrations: CommodityFuture, CommodityForward, CommoditySwap, CommodityOption, CommodityAsianOption, CommoditySwaption, CommoditySpreadOption | private mod |

The asset-class shards contain no logic beyond registration, apart from the two
hand-written `Pricer` implementations in `credit.rs`. They are private modules
whose only public effect is what `register_all_pricers` assembles.

## Public API vs internal plumbing

Re-exported from `crate::pricer` (and therefore public API):

`InstrumentType`, `ModelKey`, `PricerKey`, `Pricer`, `PricerRegistry`,
`PricingError`, `PricingErrorContext`, `standard_registry()`, `expect_inst`
(marked `#[doc(hidden)]`), every `pub fn` in `json` and
`structured_credit_json`, `STANDARD_OPTION_GREEKS`, and the `cos` module.

Internal, despite living next to the public surface:

| Item | Visibility | Note |
|------|-----------|------|
| `shared_standard_registry()` | `pub(crate)` | `Arc` handle onto the same singleton `standard_registry()` returns |
| `register_generic!` | `pub(crate) use` | Registration boilerplate for `GenericInstrumentPricer` |
| `PricerRegistry::price_with_metrics_shared` | `pub(crate)` | Entry point used by `Instrument::price_with_metrics` |
| `PricerRegistry::price_raw` | `pub(crate)` | Unrounded `f64` path for finite-difference risk |
| `PricerRegistry::metric_registry_override` | `pub(crate)` | |
| `PricerRegistry::duplicate_keys` | `pub(super)` | Read only by `build_standard_registry` and its test |
| `registry::attach_metric_measures` | `pub(super)` | |
| `errors::actionable_unknown_pricer_message` | `pub(crate)` | Rewrites `UnknownPricer` into a Monte-Carlo-aware hint |
| `enrichment::*` | `pub(super)` | |

`PricerRegistry` derives `Clone` and `Default`, so a caller can build a bespoke
registry from `standard_registry().clone()` and mutate it, then pass it via
`PricingOptions::with_registry`.

## Keys

`InstrumentType` and `ModelKey` are
`#[repr(u16)]` with explicit discriminants, `#[non_exhaustive]`, serde
`rename_all = "snake_case"`, and derive `strum::EnumIter`. `PricerKey` is
`#[repr(C)]` (4 bytes) with `deny_unknown_fields`; `keys::tests::abi_is_stable`
pins all three sizes.

**Discriminants are wire state.** They are explicit and non-contiguous because
variants have been added out of order over time (`XccySwap = 52` sits between
`FxSwap = 18` and `InflationLinkedBond = 19`). Never renumber or reuse a
discriminant; append with the next free number.

**`as_str` is a second source of truth.** `Display` and `FromStr` both route
through the hand-written `as_str` match, while serialization goes through the
derived serde impl. Several variants carry a `#[serde(rename = ...)]` whose
value differs from the variant name, and `as_str` must repeat it exactly:

| Variant | Wire name |
|---------|-----------|
| `InstrumentType::Cds` | `credit_default_swap` |
| `InstrumentType::Irs` | `interest_rate_swap` |
| `InstrumentType::Fra` | `forward_rate_agreement` |
| `InstrumentType::Convertible` | `convertible_bond` |
| `InstrumentType::Dcf` | `discounted_cash_flow` |
| `InstrumentType::EquityTotalReturnSwap` | `trs_equity` |
| `InstrumentType::FiIndexTotalReturnSwap` | `trs_fixed_income_index` |
| `InstrumentType::YoYInflationSwap` | `yoy_inflation_swap` |
| `ModelKey::HullWhite1F` | `hull_white_1f` |
| `ModelKey::MonteCarloGBM` | `monte_carlo_gbm` |
| `ModelKey::MonteCarloHullWhite1F` | `monte_carlo_hull_white_1f` |
| `ModelKey::BarrierBSContinuous` | `barrier_bs_continuous` |
| `ModelKey::AsianGeometricBS` | `asian_geometric_bs` |
| `ModelKey::LookbackBSContinuous` | `lookback_bs_continuous` |
| `ModelKey::QuantoBS` | `quanto_bs` |
| `ModelKey::FxBarrierBSContinuous` | `fx_barrier_bs_continuous` |
| `ModelKey::PdeCrankNicolson1D` | `pde_crank_nicolson_1d` |
| `ModelKey::PdeAdi2D` | `pde_adi_2d` |

The guard against drift between the two is
`common::pricer::registry::test_instrument_type_from_str_all_variants` (and its
`ModelKey` twin) in [`../../tests/instruments/common/pricer/registry.rs`](../../tests/instruments/common/pricer/registry.rs),
which asserts `serde_json::to_string(&variant) == format!("\"{}\"", variant.as_str())`
for every variant. The colocated unit tests in `keys.rs` only cover the
`Display`/`FromStr` round trip, which would pass even if serde disagreed.

`ModelKey::is_monte_carlo_model()` is a behavior predicate over eleven variants,
not a cargo-feature gate; it only selects the wording of the `UnknownPricer`
hint.

## The registry

```rust
pub trait Pricer: Send + Sync {
    fn key(&self) -> PricerKey;
    fn price_dyn(&self, instrument: &dyn Instrument, market: &MarketContext, as_of: Date)
        -> Result<ValuationResult, PricingError>;
    fn price_raw_dyn(&self, instrument: &dyn Instrument, market: &MarketContext, as_of: Date)
        -> Result<f64, PricingError>;   // default: price_dyn(..).value.amount()
}
```

`price_dyn` is an **unchecked, unshocked model kernel**. It may assume the
instrument already passed `validate_for_pricing()` and that `as_of` is the
resolved effective date; it must return the base PV without any scenario price
override, because the registry owns applying that exactly once. Override
`price_raw_dyn` whenever the pricer has a genuine `f64` path, so finite-difference
risk does not inherit `Money` rounding noise.

Storage is `BTreeMap<PricerKey, Arc<dyn Pricer>>`. Dispatch is by key alone, so
a hash map would work; the ordered map is there to make any enumeration of the
registry (`all_models`, `all_models_grouped`, `available_models_for_instrument`,
`list_models*`) deterministic without every call site remembering to sort.

`register` **rejects** a duplicate key: it logs `tracing::warn!`, records the key
in `duplicate_keys`, and leaves the first registration in place. `replace` is the
explicit overwrite, intended for tests and controlled monkey-patching.
`build_standard_registry` runs inside a `OnceLock` initializer and so cannot
return a `Result`; it emits `tracing::error!` per collision instead, and
`pricer::tests::register_all_pricers_has_no_duplicate_keys` is the CI gate that
turns a collision into a failing build.

`standard_registry()` returns a `&'static PricerRegistry` from a process-wide
`OnceLock<Arc<PricerRegistry>>`. When metrics are requested, the pricing path
needs an `Arc<PricerRegistry>` so calculators can reprice through the same
dispatch table; `price_with_metrics` and `price_batch` compare `self` against
the singleton with `std::ptr::eq` and reuse its `Arc` on a hit, deep-cloning the
registry map only for a bespoke registry.

## Pricing pipeline

`PricerRegistry::price_with_metrics` (and the `pub(crate)`
`price_with_metrics_shared` behind `Instrument::price_with_metrics`) runs a fixed
sequence. The ordering is load-bearing; change it and results change.

1. **Validate, then look up.** `ValidatedPricingLifecycle::new(instrument)` calls
   `Instrument::validate_for_pricing()`. A malformed instrument therefore fails
   as invalid input *before* model lookup or any market access. Only then is
   `PricerKey::new(instrument.key(), model)` resolved; a miss produces
   `UnknownPricer { key, available_models }` where `available_models` lists every
   model that *is* registered for that instrument type.
2. **Resolve the valuation date.** `effective_as_of = Instrument::resolve_pricing_as_of(market, as_of)`.
3. **Price unshocked.** `pricer.price_dyn(instrument, market, effective_as_of)`.
4. **Overwrite `as_of`.** The registry sets `result.as_of = effective_as_of`
   unconditionally. A model may enrich the result envelope but must not own the
   canonical valuation date.
5. **Stamp metadata.** Numeric mode and rounding come from the effective
   `FinstackConfig`; the model's own `timestamp` / `version` survive if it set
   them. FX policy precedence: a stamp the pricer already set, else the
   `fx_policy` stamps on the instrument's dependent discount / forward / hazard
   curves de-duplicated in source order and joined with `" | "`, else `None`.
6. **Apply the scenario shock.** `result.value = lifecycle.apply_value(result.value)`
   — once, at the valuation boundary, so every downstream metric context sees the
   same adjusted value.
7. **Return, or enrich.** An empty `metrics` slice returns here. Otherwise
   `enrichment::enrich` runs.

Enrichment has two shapes:

- `model == Discounting`, or the instrument has no custom metrics equivalent:
  one `compute_metrics_dyn` pass over the instrument itself.
- Otherwise: metrics are partitioned. Those in
  `MetricId::SPREAD_EQUIVALENT_METRICS` (z-spread, YTM, ASW, …) are computed on
  `instrument.metrics_equivalent()` — a cash-normalized copy, e.g. PIK coupons
  converted to Cash — so spreads are on a cash-equivalent basis. Everything else
  (duration, DV01, convexity, CS01) is computed on the original instrument's
  actual cashflows.

`attach_metric_measures` inserts calculator results first and the model's own
measures last, so a model-produced measure wins when both emit the same
`MetricId`.

`price_batch` maps `price_with_metrics_impl` over a rayon `par_iter` and
preserves input order; `test_price_batch_matches_serial_results` pins serial ≡
parallel.

## Adding an instrument

The steps that touch this directory, in order:

1. **Add the `InstrumentType` variant** in [`keys.rs`](keys.rs). Give it the next
   free discriminant (do not renumber anything). Add the matching arm to
   `InstrumentType::as_str`, and add a `#[serde(rename = ...)]` only if the wire
   name must differ from the snake_case variant name — in which case `as_str`
   must return that same string.
2. **Register the pricer** in the matching asset-class shard, inside that shard's
   `register_*_pricers` function. If the instrument prices by discounting its own
   cashflows, one line suffices:

   ```rust
   register_generic!(registry, InstrumentType::Foo, crate::instruments::Foo);
   ```

   That expands to a `GenericInstrumentPricer::<Foo>::discounting(...)`
   registration under `ModelKey::Discounting`, which forwards to the
   instrument's `base_value` / `base_value_raw`. For a non-discounting model, use
   the four-argument form (`register_generic!(registry, inst, ty, ModelKey::Bar)`)
   or write a `Pricer` impl and call `registry.register(...)` directly.
3. **Add a new `ModelKey`** only if no existing variant describes the model;
   same discriminant and `as_str` rules apply.
4. If the instrument's native model is not `Discounting`, override
   `Instrument::default_model` on the type so `price_with_metrics` and the
   JSON `"default"` selector reach the right pricer.
5. In a hand-written `Pricer`, downcast with
   `expect_inst::<Foo>(instrument, InstrumentType::Foo)?` — it checks the
   `InstrumentType` tag *and* the concrete type, and returns `TypeMismatch`.
   Return the unshocked base PV; do not apply scenario overrides or re-resolve
   `as_of`.

The remaining steps live outside this directory:
`with_instrument_json_registry!` in `../instruments/json_loader.rs`, the
`register_<name>_metrics` hook in `../metrics/core/standard_registry.rs`, tests
under `../../tests/instruments/<name>/`, and
`mise run rust-gen-schemas` when the public JSON shape changes. See
[`../instruments/README.md`](../instruments/README.md).

## Errors

`PricingError` has five `#[non_exhaustive]` variants: `UnknownPricer`,
`TypeMismatch`, `ModelFailure`, `InvalidInput`, `MissingMarketData`. Every
variant except the first two carries a `PricingErrorContext`
(instrument id, instrument type, model, curve ids), which is what makes a batch
failure diagnosable.

Both conversions are **lossy in one direction each** and are documented with a
mapping table in [`errors.rs`](errors.rs):

- `PricingError -> finstack_quant_core::Error` flattens the typed context into a
  message string; `ModelFailure` always lands as `Calibration { category: "pricing_model" }`.
- `PricingError::from_core(err, context)` is the *only* way in. There is
  deliberately no blanket `From<core::Error>` impl, because it would silently
  attach an empty context; the signature forces the caller to supply one.

`PricingError` derives serde because it flows through the crate-level
`crate::Error` wire envelope.

`Instrument::price_with_metrics` post-processes `UnknownPricer` through
`actionable_unknown_pricer_message`, which — for a Monte Carlo `ModelKey` —
replaces the message with a hint naming the likely cause (a barrier/lookback
switched off continuous monitoring, a Bermudan swaption asking for LSMC) plus
the available-model list.

## JSON entry points

`json.rs` is the shared pipeline behind both host bindings: parse a canonical
`InstrumentEnvelope`, optionally merge metric pricing overrides, parse the
as-of date and model key, dispatch through the standard registry.

| Function | Returns |
|----------|---------|
| `parse_instrument_json` | `InstrumentJson` |
| `parse_boxed_instrument_json` | `Box<dyn Instrument>` |
| `instrument_envelope_from_spec` | `String` (canonical envelope JSON) |
| `validate_instrument_json`, `validate_typed_instrument_json` | `String` (re-serialized envelope) |
| `pretty_instrument_json` | `String` |
| `parse_model_key` | `ModelKey` |
| `list_models`, `list_models_grouped` | `Vec<String>` / `BTreeMap<String, Vec<String>>` |
| `list_standard_metrics`, `list_standard_metrics_grouped` | same shapes |
| `price_instrument_json` | `ValuationResult` |
| `metric_value_from_instrument_json` | `f64` |
| `present_metric_values_from_instrument_json` | `Vec<(&str, f64)>` |
| `present_standard_option_greeks_from_instrument_json` | `Vec<(&'static str, f64)>` |

**The `_json` suffix here means JSON *in*, not JSON *out*.** `price_instrument_json`
returns a typed `ValuationResult`; the five `structured_credit_tranche_*_json`
functions return `f64`, `OasResult`, `TrancheMetrics`, and `ScenarioTable`. Only
`validate_*`, `pretty_*`, and `instrument_envelope_from_spec` return a JSON
string, and each of those is a validation/formatting surface rather than a
computation. This is consistent with the result-return contract in
[`.agents/rules/project-rules.md`](../../../../.agents/rules/project-rules.md)
even though the names read the other way.

Two further conventions:

- **`list_models*` is registry-derived, not enum-derived.** A `ModelKey` variant
  with no registered pricer is omitted, so the output describes real dispatch
  coverage rather than the model vocabulary `ModelKey::iter()` would advertise.
  `list_standard_metrics*`, by contrast, comes from the metric registry.
- **Non-finite results are rejected at the boundary.** `structured_credit_json.rs`
  runs `ensure_finite` over every scalar field before returning, because
  `serde_json` renders `NaN`/`inf` as JSON `null`, which downstream consumers
  coerce to `0` — a failed computation must not read as a valid one.

`price_instrument_json` accepts `"default"` for `model`, which resolves to
`Instrument::default_model()`. The comparison in `resolve_model_key` is exact
(`model == "default"`), so the "case-insensitive" wording in the rustdoc on
`price_instrument_json` and `parse_model_key` is wrong — `"Default"` is rejected.

## COS Fourier pricer

`cos.rs` implements the Fang-Oosterlee (2008) cosine-series method against the
characteristic functions in `finstack_quant_core::math::characteristic_function`:
Black-Scholes, Variance Gamma, and Merton jump-diffusion. `CosPricer` amortizes
the strike-independent coefficients across a strike strip.

It lives directly under `pricer::cos` rather than in a `pricer::fourier`
namespace because it is the only Fourier method the crate exposes — an earlier
Lewis (2001) pricer was removed after it was found to diverge off-ATM and to
clamp non-finite integrand panels to zero. It is not reachable through the
registry; it is a standalone numeric surface bound directly in Python
(`finstack_quant.valuations`) and WASM, and benched by
[`../../benches/fourier_var_pricing.rs`](../../benches/fourier_var_pricing.rs).

## Verification

```bash
# Colocated unit tests (never `cargo test` — it would also run doc tests).
cargo nextest run -p finstack-quant-valuations --lib -E 'test(/pricer::/)'

# Registry construction, key round trips, batch parity, error display.
cargo nextest run -p finstack-quant-valuations --test instruments -E 'test(/common::pricer/)'

mise run rust-test
mise run rust-lint

# Regenerate after changing any public serde type reachable from a key.
mise run rust-gen-schemas
mise run rust-check-schemas
```

The CI-critical unit tests in this directory are
`pricer::tests::register_all_pricers_has_no_duplicate_keys` (no shard silently
overwrites another's pricer) and `pricer::keys::tests::abi_is_stable` (the
`repr` sizes that the wire format assumes).

No bench targets this module directly; registry dispatch is exercised through
the per-instrument benches (`bond_pricing`, `option_pricing`, `swap_pricing`, …)
listed in [`../../benches/README.md`](../../benches/README.md).

## Related

- [`../../README.md`](../../README.md) — crate overview and conventions
- [`../instruments/README.md`](../instruments/README.md) — the `Instrument` trait and the full add-an-instrument checklist
- [`../metrics/README.md`](../metrics/README.md) — `MetricId`, calculators, `MetricRegistry`
- [`../results/README.md`](../results/README.md) — `ValuationResult`, `ResultsMeta`, FX-policy stamping
- [`../models/`](../models/) — the numerical methods the registered pricers call:
  [`closed_form/`](../models/closed_form/README.md), [`pde/`](../models/pde/README.md),
  [`trees/`](../models/trees/README.md), [`credit/`](../models/credit/README.md),
  [`volatility/`](../models/volatility/README.md)
