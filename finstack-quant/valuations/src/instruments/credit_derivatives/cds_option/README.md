# CDS Option

CDS options are priced with the Bloomberg CDSO numerical-quadrature model, not
the legacy closed-form Black-on-forward-spread approximation. The instrument
supports European payer/receiver options on single-name CDS and CDS indices,
struck either on a forward spread or on a clean index price (the CDX HY
convention), with explicit controls for settlement, protection-start
convention, knockout behavior, index factors, realized index loss, and
underlying CDS coupon.

## Strike conventions

The strike is a typed enum, `CDSOptionStrike`:

- `Spread` — decimal annual rate; `{"spread": "0.0325"}` means 325 bp.
  Single-name, CDX IG, and iTraxx options are quoted this way.
- `CleanPricePct` — percentage-price points; `{"clean_price_pct": "107.0"}`
  means a clean-price fraction `K = 1.07`. CDX HY index options are quoted
  this way. Price strikes require an index underlying, no-knockout terms, an
  explicit contractual coupon, the current index factor `f`, and the original
  strike factor `f0` (`strike_index_factor`); `f0` is never inferred from the
  current factor after a default.

The pre-enum bare decimal strike wire shape is rejected with no compatibility
fallback.

## Methodology

The pricer implements Bloomberg DOCS 2055833 Eq. 2.5:

```text
O = P(t_e) * E_0[(xi * V_te + H(K) + D)+]
```

- `V_te` is the random forward CDS value at option expiry; the state variable
  is the lognormal forward CDS spread for both strike conventions.
- `H(K)` is the deterministic strike term, branched by strike kind:
  - spread strike: `H_spread = xi * (c - K) * A(K)` (Eq. 2.4);
  - clean-price strike: `H_price = xi * (K - 1) * f0 / f`, evaluated inside
    the outer current-factor scale `f`.
- `D` is the deterministic settlement of realized index losses and expected
  front-end protection: `D = xi * (L / f + FEP)`. Realized loss lives here
  exactly once — it is never also folded into an adjusted strike.
- `m` is calibrated so the lognormal spread process reproduces the bootstrapped
  no-knockout forward value `F_0`.

The native ATM-forward clean-price coordinate follows from payer/receiver
parity under the same payoff: `K_ATM = 1 - (f*F0 + L + f*FEP) / f0`, exposed
in percentage points for moneyness and surface selection.

Underlying CDS mechanics use Bloomberg CDSW-style conventions from DOCS 2057273
where relevant, including spot default-leg valuation and the CDSO-scoped
inclusive protection-end adjustment.

## Settlement

Cash- and physical-settled European options carry the same cash-equivalent
model NPV before expiry and route through the same quadrature. The clean
payoff excludes accrued because the same underlying accrued appears on both
sides before exercise and cancels; a physical exercise cashflow at settlement
is dirty and includes accrued at exercise settlement. This pricer values the
pre-expiry option only — it does not create or deliver a live underlying CDS
position, and valuation at or after a physical exercise lifecycle boundary
fails explicitly rather than returning a misleading cash number. Manual
exercise state, partial exercise, and post-expiry settlement lifecycle are out
of scope.

## Volatility

The quadrature consumes a lognormal forward-spread model volatility for both
strike conventions — a clean-price strike axis does not change the state
variable. Resolution is strict:

1. An instrument implied-vol override has highest precedence and needs no
   surface.
2. Otherwise the `VolSurface` under `vol_surface_id` is queried with
   `value_checked(t_expiry, native_strike_coordinate)` — the decimal spread
   (`0.0325`) for spread strikes and the percentage clean price (`107.0`) for
   price strikes. Expiry or strike extrapolation is an error; there is no
   clamped fallback.
3. The surface must carry `VolSurfaceAxis::Strike` and
   `VolQuoteType::BlackLognormal`. Stored values are model spread vols even on
   a price-quoted strike axis; provider premiums or provider-specific "price
   vols" must be inverted to model vol before a surface is materialized.

## Usage Example

```rust
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_quant_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 5)?;
let opt = CDSOption::example().unwrap();
let pv = opt.value(&market_context, as_of)?;
```

## Metrics

- PV, delta, gamma, vega, theta, CS01, DV01, Recovery01.
- `dv01` is the canonical CDSO interest-rate sensitivity: it bumps the
  calibrated swap-curve quotes and rebuilds the discount curve.
- `par_spread` reports the Bloomberg CDSO displayed ATM forward spread.
- `implied_vol` solves the Bloomberg quadrature price in log-vol space.
- Spread-struck delta/gamma are the Bloomberg closed-form Black-76 `N(d1)`
  screen values on the displayed ATM forward spread. That formula is not
  valid for a clean-price strike; price-struck delta/gamma use curve-reprice
  hedge-ratio semantics instead.

## Limitations

- European exercise only; pre-expiry valuation only.
- Lognormal spread volatility; stochastic recovery and volatility are out of scope.
- Distressed forward spreads beyond the Bloomberg CDSO calibration guard are
  rejected.
- Some Bloomberg CDSO internals remain proprietary; source-backed residuals are
  documented in the `cdx_ig_46` golden fixture rather than widened away.

## References

- Bloomberg L.P. Quantitative Analytics, *Pricing Credit Index Options*, DOCS
  2055833.
- Bloomberg L.P. Quantitative Analytics, *The Bloomberg CDS Model*, DOCS
  2057273.
- S&P Dow Jones Indices, *CDS Indices Primer* — clean-price strike
  factor/loss adjustment (the `107.0 -> 107.9874` fixture).
