# Bond

Fixed, floating, step-up, amortizing, callable/putable and PIK bonds, with
discount, hazard, tree (OAS) and Merton structural-credit pricing paths.

`Bond` is the reference instrument of `finstack-quant-valuations`: several other
instruments (asset-swap legs, convertibles, bond futures) reuse its cashflow
spec, quote conversions and yield solver.

## Public surface

Import path: `finstack_quant_valuations::instruments::fixed_income::bond`
(the main types are also re-exported at `finstack_quant_valuations::instruments`).

| Item | Purpose |
|------|---------|
| `Bond` | The instrument. `Bond::builder()` plus factories `fixed`, `floating`, `zero_coupon`, `with_convention`, `example`. |
| `CashflowSpec` | `Fixed` / `Floating` / `StepUp` / `Amortizing { base, schedule }`. |
| `AmortizationSpec` | Amortization schedule for `CashflowSpec::Amortizing`. |
| `CallPut`, `CallPutSchedule` | Embedded call/put windows and redemption prices. |
| `MakeWholeSpec` | Make-whole call: `max(price_pct_of_par, PV @ reference curve + spread)`. |
| `ReturnFloorSpec`, `ReturnFloorKind`, `IssuePrice`, `ProtectionWindow` | Guaranteed minimum MOIC/XIRR call protection. |
| `BondSettlementConvention` | Settlement days and ex-coupon days. |
| `AccrualMethod` | `Linear` (default) or `Compounded` (ICMA Rule 251). |
| `bond_from_cashflows_json` | Build a `Bond` from an explicit cashflow list. |
| `BondBuilderParams`, `FloatingConventionParams` | Argument structs for `CashflowSpec::from_bond_builder_params` and `::floating_with_conventions`; they flatten the binding-facing builder fields into one parameter. |
| `pricing::engine::{discount, hazard, tree, merton_mc}` | Pricing engines and their registry adapters. |
| `pricing::quote_conversions`, `pricing::ytm_solver` | Price ↔ yield ↔ spread conversions and the YTM root finder. |

Details are in the rustdoc; this file covers layout, conventions and the parts
that are easy to get wrong.

## Module layout

```
bond/
├── mod.rs                   # re-exports; module-level pricing conventions
├── types/                   # Bond struct and construction
│   ├── definitions.rs       # Bond, CallPut, CallPutSchedule, MakeWholeSpec, BondSettlementConvention
│   ├── construction.rs      # builder + factories (fixed/floating/zero_coupon/with_convention/example)
│   ├── return_floor.rs      # ReturnFloorSpec, ReturnFloorKind, IssuePrice, ProtectionWindow
│   ├── pricing.rs           # Instrument::base_value and quote-override precedence
│   └── traits.rs            # Instrument / CashflowProvider impls
├── cashflow_spec.rs         # CashflowSpec + BondBuilderParams + FloatingConventionParams
├── cashflows.rs             # holder-view (Date, Money) projection
├── json.rs                  # bond_from_cashflows_json
├── pricing/
│   ├── engine/              # pricing math + the thin Simple*Pricer registry adapters
│   │   ├── discount.rs      # BondEngine: PV = Σ CF_i × DF_i
│   │   ├── hazard.rs        # HazardBondEngine + SimpleBondHazardPricer (survival-weighted PV + FRP)
│   │   ├── tree/            # TreePricer + SimpleBondOasPricer (callable/putable, OAS)
│   │   └── merton_mc/       # MertonMcEngine + SimpleBondMertonMcPricer (structural credit, PIK)
│   ├── quote_conversions/   # yield↔price, spread↔price, annuity helpers
│   ├── ytm_solver.rs        # Newton/Brent yield-to-maturity solver
│   ├── return_floor.rs      # lowers ReturnFloorSpec into a CallPutSchedule at pricing time
│   ├── settlement.rs        # quote date, accrued interest, ex-coupon windows
│   └── time_basis.rs        # metric time axis helpers
└── metrics/                 # bond-specific MetricCalculator impls
    ├── price_yield_spread/  # clean/dirty price, YTM, YTW, Z, OAS, I-spread, DM, ASW, vega
    ├── return_metrics/      # MOIC and XIRR (spot and to-worst)
    ├── accrued.rs  convexity.rs  cs01.rs  dv01.rs  yield_dv01.rs
    ├── duration_macaulay.rs  duration_modified.rs  effective.rs  spread_duration.rs  wal.rs
    └── risk_view.rs         # quote-reproducing basis shared by the bump metrics
```

### Engines vs pricers

Each `engine/*` file holds the pricing math **and** the thin `Simple*Pricer`
registry adapter next to it. The adapter downcasts the instrument, calls the
engine, and wraps the result in a `ValuationResult`. Registration happens once,
in `src/pricer/rates.rs`:

| Engine | `ModelKey` | Registered pricer |
|--------|-----------|-------------------|
| `BondEngine` | `Discounting` | generic pricer (`register_generic!` → `Instrument::base_value`) |
| `HazardBondEngine` | `HazardRate` | `SimpleBondHazardPricer` |
| `TreePricer` | `Tree` | `SimpleBondOasPricer` |
| `MertonMcEngine` | `MertonMc` | `SimpleBondMertonMcPricer` |

Adding a model means one file under `pricing/engine/` and one `registry.register(...)`
line in `src/pricer/rates.rs`.

## Construction

```rust
use finstack_quant_valuations::instruments::fixed_income::bond::{Bond, CashflowSpec};
use finstack_quant_valuations::instruments::{Attributes, InstrumentPricingOverrides};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{DayCount, Tenor};
use finstack_quant_core::money::Money;
use finstack_quant_core::types::Rate;
use time::macros::date;

// Factory: semi-annual, 30/360, T+2 settlement.
let corp = Bond::fixed(
    "CORP-001",
    Money::new(1_000_000.0, Currency::USD),
    Rate::from_percent(5.0),
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    "USD-OIS",
)?;

// Builder: full control over the coupon spec.
let bond = Bond::builder()
    .id("BOND-001".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue_date(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360)?)
    .discount_curve_id("USD-OIS".into())
    .instrument_pricing_overrides(InstrumentPricingOverrides::default())
    .attributes(Attributes::new())
    .build()?;
```

Notes that bite:

- Builder setters use the **field names**: `issue_date` (not `issue`),
  `maturity`, `cashflow_spec`, `discount_curve_id`.
- Every `Option<T>` field gets two setters: `.call_put(schedule)` takes the
  inner value, `.call_put_opt(Some(schedule))` takes the `Option`.
- `CashflowSpec::fixed` / `fixed_rate` return `Result` (the coupon must be a
  finite `Decimal`). `CashflowSpec::floating_bp` takes a typed `Bps` and does
  not.
- `Bond::fixed` / `with_convention` / `zero_coupon` / `floating` all return
  `Result<Bond>` and run `validate()` before returning.

### Floating-rate notes

```rust
use finstack_quant_valuations::instruments::fixed_income::bond::Bond;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{DayCount, Tenor};
use finstack_quant_core::money::Money;
use finstack_quant_core::types::Bps;
use time::macros::date;

let frn = Bond::floating(
    "FRN-001",
    Money::new(1_000_000.0, Currency::USD),
    "USD-SOFR-3M",
    Bps::new(200),
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    Tenor::quarterly(),
    DayCount::Act360,
    "USD-OIS",
)?;
```

Reset lag and fixing calendar default from the index id when it is a known
rate index; construct `FloatingCouponSpec` directly for floors, caps or gearing.

### Callable and putable bonds

```rust
use finstack_quant_valuations::instruments::fixed_income::bond::{
    Bond, CallPut, CallPutSchedule, CashflowSpec,
};
use finstack_quant_valuations::instruments::{Attributes, InstrumentPricingOverrides};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{DayCount, Tenor};
use finstack_quant_core::money::Money;
use time::macros::date;

// A one-day (discrete) call date uses the same value for start_date and end_date.
let schedule = CallPutSchedule {
    calls: vec![
        CallPut {
            start_date: date!(2027 - 01 - 01),
            end_date: date!(2027 - 01 - 01),
            price_pct_of_par: 102.0,
            make_whole: None,
        },
        CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 101.0,
            make_whole: None,
        },
    ],
    puts: vec![],
};

let callable = Bond::builder()
    .id("CALLABLE-001".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue_date(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.06, Tenor::semi_annual(), DayCount::Thirty360)?)
    .discount_curve_id("USD-OIS".into())
    .call_put(schedule)
    .instrument_pricing_overrides(InstrumentPricingOverrides::default())
    .attributes(Attributes::new())
    .build()?;
```

`price_pct_of_par` is applied to the **outstanding** principal at the exercise
date, so amortizing callables are handled correctly. At a node the coupon is
always paid; the exercise decision applies to principal only:
`node_value = coupon + min(max(continuation, put_price), call_price)`.

### PIK bonds

PIK coupons accrete to notional instead of paying cash. Set
`CouponType::Pik` (or `Split { cash_pct, pik_pct }`) on `FixedCouponSpec` /
`FloatingCouponSpec` and price under `ModelKey::MertonMc`.

The Merton MC engine (`pricing::engine::merton_mc`) prices PIK bonds in a
structural credit framework:

- **Merton model** — asset value follows GBM (or jump diffusion); default is a
  first-passage barrier breach.
- **Endogenous hazard** — the hazard rate rises with leverage.
- **Dynamic recovery** — recovery declines as PIK accrual grows the notional.
- **PIK schedule** — `PikSchedule` / `PikMode` give per-coupon Cash / PIK /
  Split / Toggle modes, including time-stepped windows.
- **Toggle exercise** — threshold, stochastic (sigmoid), or optimal (nested MC)
  PIK-versus-cash decisions.
- **Cash-equivalent metrics** — Z-spread and YTM are computed on an equivalent
  cash-pay structure so PIK and cash-pay bonds are comparable.
- **Barrier calibration** — `merton_mc::calibration` fits the barrier to a
  target historical annual PD.

Public types: `MertonMcConfig`, `MertonMcResult`, `MertonMcCalibrationSpec`,
`CalibrationParameter`, `PikSchedule`, `PikMode`, `PathStatistics`,
`BarrierCrossing`.

## Pricing conventions

- **PV anchor**: `Instrument::value` is the dirty NPV at `as_of`, not the
  quoted dirty price at settlement. Settlement affects how *quotes* are
  interpreted.
- **Quote date**: market-derived metrics (YTM, YTW, Z-spread, DM, OAS,
  duration, convexity) are computed from `quote_date = as_of + settlement_days`,
  with accrued measured at that date, because market quotes are settlement
  quotes.
- **Cashflow sign**: holder view. Coupons, amortization and redemption are
  positive; purchase price and funding legs live outside the schedule. PIK
  coupons carry zero cash and grow the redemption amount.
- **Rate units**: `f64` model inputs are decimals (`0.05` = 5%). Fields and
  arguments suffixed `_bp` are basis points; `Bps` and `Rate` are the typed
  forms.
- **Accrual**: `AccrualMethod::Linear` by default; `Compounded` follows ICMA
  Rule 251. Ex-coupon windows drop accrual to zero.
- **Quote overrides**: `InstrumentPricingOverrides::market_quotes` accepts at
  most one price driver (clean price, dirty price, YTM, YTW, Z-spread, OAS,
  discount margin, I-spread, ASW). `validate()` rejects two. A scenario spread
  shock composes additively with a quoted Z-spread but errors against a
  price-pinning quote rather than silently no-op'ing.

### Regional conventions

`Bond::with_convention(id, notional, coupon, issue, maturity, convention, curve_id)`
applies `BondConvention`:

| Convention | Day count | Frequency | Settlement | Ex-coupon |
|------------|-----------|-----------|------------|-----------|
| `UsTreasury` | ACT/ACT ICMA | Semi-annual | T+1 | — |
| `UsAgency` | 30/360 | Semi-annual | T+1 | — |
| `UsCorporate` | 30/360 | Semi-annual | T+1 | — |
| `EurCorporate` | ACT/ACT ICMA | Annual | T+2 | — |
| `UkGilt` | ACT/ACT ICMA | Semi-annual | T+1 | 7 days |
| `GermanBund` | ACT/ACT ICMA | Annual | T+2 | — |
| `FrenchOat` | ACT/ACT ICMA | Annual | T+2 | — |
| `Jgb` | ACT/365F | Semi-annual | T+2 (cross-border) | — |

## Metrics

Registered for `InstrumentType::Bond` in `metrics/mod.rs`:

| Group | `MetricId` |
|-------|-----------|
| Price | `Accrued`, `CleanPrice`, `DirtyPrice` |
| Yield | `Ytm`, `Ytw`, `YieldDv01` |
| Duration / convexity | `DurationMac`, `DurationMod`, `Convexity`, `SpreadDuration`, `WAL` |
| Spreads | `ZSpread`, `ISpread`, `Oas`, `DiscountMargin`, `ASWPar`, `ASWMarket` |
| Optionality | `EmbeddedOptionValue`, `Vega` |
| Rates risk | `Dv01`, `BucketedDv01` |
| Credit risk | `Cs01`, `BucketedCs01`, `CrossGammaRatesCredit` |
| Return floor | `Moic`, `MoicToWorst`, `Xirr`, `XirrToWorst` |

`Theta` is registered universally by `metrics::standard_registry()`.
Duration and convexity switch to the effective (option-aware) computation in
`metrics/effective.rs` when the bond carries a call/put schedule.

## Return floors (guaranteed minimum MOIC / XIRR)

A return floor is an **issuer-side, call-protection-only** term common in
private credit and leveraged loans: on any early issuer-called or prepaid
redemption inside the protection window, the redemption price is floored so the
investor's realized return from issue against invested capital `V0` meets a
stated minimum. It does **not** guarantee the held-to-maturity return — the
maturity path is always unfloored.

```rust
use finstack_quant_valuations::instruments::fixed_income::bond::{
    Bond, ProtectionWindow, ReturnFloorSpec,
};
use finstack_quant_core::types::Rate;
use time::macros::date;

// 1.25x MOIC floor, prepayable across the bond's full life.
let moic_floor = Bond::example()?.min_moic(1.25);

// 12% minimum XIRR floor.
let xirr_floor = Bond::example()?.min_xirr(Rate::from_percent(12.0));

// NC-2: the floor only binds on calls from 2027-01-01 onward.
let nc2 = Bond::example()?.with_return_floor(
    ReturnFloorSpec::moic(1.25).window(ProtectionWindow::From(date!(2027 - 01 - 01))),
);
```

`ReturnFloorSpec` builders: `moic(f64)` / `xirr(impl Into<Rate>)`, then
`.issue_price(IssuePrice::{Par, PctOfPar(p), Amount(m)})`,
`.window(ProtectionWindow::{Full, From(d), Between { start, end }})`,
`.day_count(DayCount)` (defaults to Act/365F, matching `core::cashflow::xirr`).

The spec is lowered into a `CallPutSchedule` at pricing time
(`pricing/return_floor.rs`), making every in-window coupon date a
floor-protected call date.

**`*ToWorst` honesty caveat**: `MoicToWorst` / `XirrToWorst` take the minimum
over *all* exit paths, including the unfloored maturity path, so they are not
bounded below by the floor target. When the natural maturity return is below
the target, the maturity path is the worst case and the metric reports it. The
floor's actual guarantee (every early-call path meets the target) is verified by
the unit tests in `pricing/return_floor.rs` and by
[`tests/return_floor_example.rs`](../../../../tests/return_floor_example.rs).

### Known limitations

- **Floating coupons** are forward-projected from the curve at pricing time;
  path-accurate LSMC (rate paths driving coupon and call trigger together) is
  not implemented.
- **Make-whole calls** cannot compose with a return floor; the combination is
  a validation error because make-whole effective prices are path dependent.
- **Amortizing to-worst**: `MoicToWorst` / `XirrToWorst` use the initial
  notional as the redemption basis — exact for bullets, overstated for
  amortizers.
- `min_moic` / `min_xirr` imply `ProtectionWindow::Full`; use
  `.with_return_floor(...)` with an explicit window for a no-call period.

## Bindings

- **Python**: `finstack_quant.valuations.instruments.Bond` — a typed wrapper
  with `Bond.fixed(...)`, `Bond.floating(...)`, `to_json`/`from_json` and
  `price_merton_mc(...)`. Generic pricing goes through
  `finstack_quant.valuations.instruments.price_instrument(instrument_json, market, as_of, ...)`.
- **WASM**: `valuations.instruments.Bond`, plus the JSON-envelope entry points
  `valuations.instruments.priceInstrument`,
  `valuations.instruments.instrumentCashflowsJson` and
  `valuations.instruments.bondFromCashflowsJson`.

Both paths use the canonical `finstack_quant.instrument/1` envelope with
`InstrumentJson::Bond`; unknown fields are rejected on deserialize.

## Other limitations

- Deterministic curve inputs outside the Merton MC engine; no stochastic
  rate/credit paths.
- No tax/withholding, fail penalties, or settlement-date PV.
- Merton MC DV01/CS01 require re-running the simulation with bumped curves;
  only cash-equivalent Z-spread and YTM are computed inline.
- Inflation linkage and convertibility live in
  [`../inflation_linked_bond/`](../inflation_linked_bond/) and
  [`../convertible/`](../convertible/).

## Verification

```bash
cargo nextest run -p finstack-quant-valuations --test instruments bond::

mise run rust-test

mise run rust-lint
```

## See also

- [`../../README.md`](../../README.md) — instrument module map and how to add one
- [`../../../metrics/README.md`](../../../metrics/README.md) — metric ids and calculators
- [`INVARIANTS.md`](../../../../../../INVARIANTS.md) — Decimal/f64,
  determinism and serde invariants
- [`docs/REFERENCES.md`](../../../../../../docs/REFERENCES.md) —
  bibliography for day counts, conventions and models
