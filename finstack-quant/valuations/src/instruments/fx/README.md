# FX

Foreign-exchange instruments: spot, outright forwards, swaps and NDFs on the
linear side; Garman–Kohlhagen vanillas plus barriers, digitals, touches,
variance swaps and quantos on the option side. Ten leaves, about 16k lines, all
sharing one currency-pair convention and one spot-date rule.

This file is an index — which leaf owns which product and which convention it
follows — plus the conventions shared across the family. No leaf here carries
its own README; per-instrument formulas, references and examples are in the
leaf `mod.rs` rustdoc.

## Leaves

| Directory | Prices | Market convention / model |
|-----------|--------|---------------------------|
| `fx_spot/` | Spot FX positions | Value in quote currency = base amount × spot; settlement rolled with the CLS-consistent joint-calendar spot rule (T+2 for majors, T+1 for USD/CAD) |
| `fx_forward/` | Outright forwards | Covered interest parity, `F = S · DF_foreign(T) / DF_domestic(T)`; PV = notional × (F_market − F_contract) × DF_domestic(T) |
| `fx_swap/` | FX swaps with near and far legs | Near leg at spot, far leg at the CIP forward; the forward points embed the interest differential |
| `ndf/` | Non-deliverable forwards on restricted pairs | `NdfQuoteConvention::BasePerSettlement` (default, standard for Asian NDFs) or `SettlementPerBase`; typed `NdfFixingSource` (`Pboc`, `Cnhfix`, `Rbi`, `Kftc`, `Ptax`, `Taifx`, `PhpBval`, `Jisdor`, `Bnm`, `Other`). Pre-fixing uses CIP; post-fixing requires an observed rate set via `with_fixing_rate()` or pricing errors |
| `fx_option/` | Vanilla European FX calls and puts | Garman–Kohlhagen (1983) — foreign currency behaves as an asset paying a continuous dividend `r_f`. `FxAtmDeltaConvention` = `Spot`, `Forward`, `PremiumAdjustedSpot`, `PremiumAdjustedForward`. **European only**: American and Bermudan exercise return an error |
| `fx_barrier_option/` | Knock-in / knock-out FX barriers | Reiner–Rubinstein (1991) analytic under `ModelKey::FxBarrierBSContinuous`, or GBM Monte Carlo (`MonteCarloGBM`) for discrete monitoring; the `use_gobet_miri` flag selects MC as the default model |
| `fx_digital_option/` | FX binaries | `DigitalPayoutType::CashOrNothing` (fixed cash in the payout currency) or `AssetOrNothing` (one unit of the base currency), Garman–Kohlhagen forms. Cash call + cash put = discounted payout |
| `fx_touch_option/` | One-touch and no-touch (American binaries) | `TouchType` = `OneTouch` / `NoTouch`; `PayoutTiming` = `AtHit` / `AtExpiry`; `BarrierDirection` sets η. Rubinstein–Reiner (1991) closed form for continuous monitoring. One-touch + no-touch = discounted payout |
| `fx_variance_swap/` | FX variance swaps | Carr–Madan replication over OTM FX options, accounting for the domestic/foreign rate differential; observation schedule built on joint base/quote calendars |
| `quanto_option/` | Quanto options on a foreign asset settled in domestic currency at a **fixed** FX rate | Drift adjusted by `r_f − q − ρ·σ_asset·σ_FX`, discounted at `r_d` (`ModelKey::QuantoBS`). Negative ρ raises call value, positive ρ lowers it. **Analytic only** — Monte Carlo is deliberately not registered because the payoff/drift parameterization would differ materially |
| `shared.rs` | Not an instrument. `pub(crate)` collection of FX option pricer inputs (`resolve_fx_spot`, `collect_fx_option_inputs`, `collect_fx_option_inputs_no_vol`), used by `fx_option`, `fx_digital_option` and `fx_barrier_option` | — |

`quanto_option/` lives here because it is an FX-correlation product and carries
the `"fx"` JSON category, but its metrics and pricer register with the exotics
shards. See Registration below.

## Public surface

Import path: `finstack_quant_valuations::instruments::fx::<leaf>`. All ten leaf
directories are `pub mod`; `shared` is `pub(crate) mod`. The headline types are
also re-exported flat at `finstack_quant_valuations::instruments`:

`FxSpot`, `FxForward`, `FxSwap`, `Ndf`, `FxOption`, `FxBarrierOption`,
`FxDigitalOption`, `DigitalPayoutType`, `FxTouchOption`, `TouchType`,
`BarrierDirection`, `PayoutTiming`, `FxVarianceSwap`, `QuantoOption`.

Reachable only under the family path: `fx_option::{FxAtmDeltaConvention,
FxOptionBuilder}`, `fx_forward::FxForwardBuilder`,
`fx_variance_swap::FxVarianceSwapBuilder`, `ndf::{NdfQuoteConvention,
NdfFixingSource}`, and `fx_barrier_option::monte_carlo`.

Three leaves re-export a shared parameter type under their own path:
`fx_option::FxUnderlyingParams` and `fx_swap::FxUnderlyingParams`, and
`fx_variance_swap::PayReceive`. All three name the same types that already exist
at `instruments::*` — aliases, not FX-specific variants.

Inside a leaf, `metrics/`, `pricer.rs` and `types.rs` are `pub(crate)` or
private; supported items surface through each leaf's `pub use`. Two exceptions:
`fx_barrier_option::monte_carlo` is a public submodule, and `fx_spot`
re-exports three metric calculators (`BaseAmountCalculator`,
`InverseRateCalculator`, `SpotRateCalculator`) as `#[doc(hidden)]`.

## Family conventions

- **Pair orientation.** Base/quote throughout: `EUR/USD = 1.10` means one EUR
  buys 1.10 USD. Base is the foreign ("asset") currency, quote is the domestic
  ("numéraire") currency. Option payoffs are quoted in domestic; notionals are
  in base unless a type says otherwise.
- **Spot dates.** The five leaves that derive settlement dates — `fx_spot`,
  `fx_forward`, `fx_swap`, `fx_option` and `ndf` — roll the trade date through
  `instruments::fx_spot_date_for_pair` and adjust subsequent dates with
  `adjust_joint_calendar`. That helper applies the CLS rule where a USD holiday
  on an intermediate day does not delay spot but the final value date must still
  be a good USD business day. Do not hand-roll a T+2 offset.
- **Two clocks.** Option pricers combine a vol-surface clock with a
  discount-curve clock; the model rate must satisfy `exp(-r·t_vol) = df`.
  `common_impl::two_clock` documents the convention and
  `instruments::pricing::time` supplies the curve-consistent lookups. Never
  divide `ln(df)` by a year fraction measured on a different day count.
- **Volatility precedence.** Surface-driven pricers resolve σ through
  `common_impl::vol_resolution::resolve_sigma_at`, so
  `MarketQuoteOverrides::implied_volatility` behaves as a flat σ across tenor
  and strike everywhere in the family.
- **FX policy visibility.** `FxForward`, `FxOption` and `QuantoOption` override
  `Instrument::valuation_details` to surface the FX-matrix triangulation flag on
  the `ValuationResult`, satisfying the workspace FX-policy-visibility
  invariant. A new FX instrument that triangulates should do the same.
- **Delta conventions are explicit.** FX markets quote several deltas; the
  instrument stores which one it means (`FxAtmDeltaConvention`) rather than
  assuming. Metrics report spot, forward and premium-adjusted variants
  separately.

## Registration

The general checklist is in [`../README.md`](../README.md#adding-an-instrument).
Landing sites for this family:

| Step | Where |
|------|-------|
| Pricer | `src/pricer/fx.rs` for nine leaves — note this shard also registers `rates::xccy_swap`. `QuantoOption` registers in `src/pricer/exotics.rs` |
| Instrument key | `InstrumentType` variant in `src/pricer/keys.rs` |
| JSON tag | `with_instrument_json_registry!` in `../json_loader.rs`, category `"fx"`. Current tags: `fx_spot`, `fx_swap`, `fx_forward`, `ndf`, `fx_option`, `fx_digital_option`, `fx_touch_option`, `fx_barrier_option`, `fx_variance_swap`, `quanto_option` |
| Metrics | `register_<name>_metrics` in the leaf's `metrics/`, called from `register_fx_instrument_metrics` in `src/metrics/core/standard_registry.rs` — except `register_quanto_option_metrics`, which is called from `register_exotic_instrument_metrics` |
| Schemas | `mise run rust-gen-schemas`, verified by `mise run rust-check-schemas` |

`FxOption` is registered under `ModelKey::Black76` even though the pricer is
Garman–Kohlhagen in spot form; the two are equivalent through the CIP forward.
Keep the key and the comment together if either moves.

## Tests and benches

Integration tests live in `../../../tests/instruments/<leaf>/`, compiled into
the single `instruments` target. Dedicated directories exist for `fx_spot`,
`fx_forward`, `fx_swap`, `ndf`, `fx_option`, `fx_barrier_option`,
`fx_variance_swap` and `quanto_option`. `fx_digital_option` and
`fx_touch_option` are covered by colocated `#[cfg(test)]` modules plus the
cross-cutting registry, serde and `fx_dependency_completeness` contract tests.

```bash
# whole target
cargo nextest run -p finstack-quant-valuations --test instruments

# one leaf (filter is a substring match on the test name)
cargo nextest run -p finstack-quant-valuations --test instruments fx_option::
cargo nextest run -p finstack-quant-valuations --test instruments fx_dependency_completeness

# colocated unit tests
cargo nextest run -p finstack-quant-valuations --lib fx::fx_touch_option

# whole workspace, what CI runs
mise run rust-test
```

Use `cargo nextest`, not `cargo test` — the latter also runs doc tests, which
this project keeps out of the normal loop. Lint with `mise run rust-lint`.

Criterion benches in `../../../benches/`: `fx_pricing` (spot, forward, swap,
NDF, option and metrics) and `fx_exotics_pricing` (barrier, touch, digital,
variance swap, quanto).

```bash
cargo bench -p finstack-quant-valuations --bench fx_exotics_pricing
mise run rust-bench          # all workspace benches, short sampling
```

## Related

- [`../README.md`](../README.md) — `Instrument` trait, JSON contract, add-an-instrument checklist
- [`../common_impl/README.md`](../common_impl/README.md) — `fx_dates`, two-clock plumbing, vol resolution, variance replication
- [`../rates/README.md`](../rates/README.md) — cross-currency swaps live there but register in this family's pricer shard
- [`../commodity/README.md`](../commodity/README.md) — the other Black-76-centric family
- [`../../metrics/README.md`](../../metrics/README.md) — greeks and metric registration
- [`../../../tests/instruments/README.md`](../../../tests/instruments/README.md) — test layout and generated fixtures
