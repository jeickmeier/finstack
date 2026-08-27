# models::trees

Lattice pricing for instruments with early exercise or path-dependent state:
equity binomial trees, curve-calibrated short-rate trees, a Hull-White
trinomial tree for Bermudan swaptions, and a two-factor correlated rate/hazard
lattice for credit-risky callables.

A shared backward-induction engine drives every recombining model, and
instrument payoff logic is decoupled from lattice evolution through the
`TreeValuator` / `TreeModel` trait pair.

## Position in the stack

Depends on `finstack_quant_core` for `MarketContext`, the `Discounting` trait,
`HazardCurve`, `PiecewiseConstantCurve`, and `math::time_grid`, and on
[`models::volatility::black`](../volatility/) — `binomial_tree.rs` uses
`d1_d2` to feed the Leisen-Reimer Peizer-Pratt inversion. Consumed by the
instrument pricers listed under
[Usage in the codebase](#usage-in-the-codebase).

Nothing in this module is bound in Python or WASM; the lattices are reached
only through the instrument pricers.

## Layout

| Path | Contents |
|------|----------|
| [`tree_framework/`](tree_framework/) | Traits, `NodeState`, evolution parameters, the generic recombining engine, `state_keys` |
| [`binomial_tree.rs`](binomial_tree.rs) | `BinomialTree` (CRR, Jarrow-Rudd, Leisen-Reimer, Tian) plus American/European/Bermudan/barrier entry points |
| [`short_rate_tree/`](short_rate_tree/) | `ShortRateTree`: Ho-Lee, Black-Derman-Toy, Black-Karasinski |
| [`hull_white_tree.rs`](hull_white_tree.rs) | `HullWhiteTree`: 1-factor trinomial in auxiliary x-space |
| [`two_factor_rates_credit.rs`](two_factor_rates_credit.rs) | `RatesCreditTree`: correlated rate + hazard 2D lattice |

Trinomial branching is a property of the framework
(`TreeBranching::Trinomial`, `EvolutionParams::equity_trinomial`) rather than a
separate model type; it is used by the Black-Karasinski lattice,
`HullWhiteTree`, and the convertible-bond Tsiveriotis-Zhang engine.

## Traits

```text
TreeValuator                       TreeModel
  ├─ value_at_maturity(&NodeState)   ├─ price(initial_vars, ttm, &MarketContext, &valuator)
  └─ value_at_node(&NodeState,       └─ calculate_greeks(..., bump_size)  [default impl]
       continuation_value, dt)
```

`TreeValuator` owns the instrument: terminal payoff, and the per-node decision
(hold vs. exercise, cap/floor, coupon accrual). `TreeModel` owns the lattice:
state evolution and backward-induction orchestration. Both require
`Send + Sync`.

`initial_vars` is a plain `HashMap<&'static str, f64>` keyed by `state_keys`
constants.

Implementors: `BinomialTree`, `ShortRateTree`, `RatesCreditTree`.
`HullWhiteTree` is not a `TreeModel` — it exposes its own
`backward_induction`, `bond_price`, `forward_swap_rate`, and `annuity`
accessors instead, because swaption pricing needs the calibrated tree's
internals directly.

## Model comparison

| Model | Branching | Factors | Calibration target | Primary use |
|-------|-----------|---------|--------------------|-------------|
| `BinomialTree` | Binomial | 1 (equity) | None (parametric) | American / Bermudan equity and commodity options |
| `ShortRateTree` | Binomial (Ho-Lee, BDT) or trinomial (BK, κ > 0) | 1 (short rate) | Discount curve | Callable/putable bonds, term loans, OAS |
| `HullWhiteTree` | Trinomial | 1 (short rate) | Discount curve | Bermudan swaptions, mean reversion beyond the binomial limit |
| `RatesCreditTree` | Binomial × binomial | 2 (rate + hazard) | Discount + hazard curves | Credit-risky bonds and loans with embedded options |

Single-factor trees are O(N²) in time and O(N) in memory; the two-factor
lattice is O(N³) in time and O(N²) in memory.

## Binomial trees

Four variants share the `price_recombining_tree` engine:

| Variant | `TreeType` | Convergence | Notes |
|---------|-----------|-------------|-------|
| Cox-Ross-Rubinstein | `CRR` | O(1/N) | `u = exp(σ√dt)`, `d = 1/u` |
| Jarrow-Rudd | `JR` | O(1/N) | `p = 0.5`; drift pushed into `u`/`d`. Not risk-neutral exact: the one-step expected return differs from `exp((r-q)dt)` by O(dt²), an accepted O(dt) accumulated bias |
| Leisen-Reimer | `LeisenReimer` | O(1/N²) | Peizer-Pratt inversion; use odd step counts |
| Tian | `Tian` | O(1/N) | Third-moment matching |

```rust
use finstack_quant_valuations::instruments::{ExerciseStyle, OptionMarketParams, OptionType};
use finstack_quant_models::trees::{BinomialTree, TreeType};

let params = OptionMarketParams::new(
    100.0, // spot
    100.0, // strike
    0.05,  // rate
    0.20,  // volatility
    1.0,   // time_to_expiry
    0.02,  // dividend_yield
    OptionType::Put,
);

// leisen_reimer_odd rounds an even request up (200 -> 201).
let tree = BinomialTree::leisen_reimer_odd(200);
assert_eq!(tree.steps, 201);

let american = tree.price_american(&params)?;
let european = tree.price_european(&params)?;
assert!(american >= european - 1e-9); // early exercise never destroys value

// Other variants: BinomialTree::crr(200), BinomialTree::new(200, TreeType::Tian).
let greeks =
    BinomialTree::new(200, TreeType::CRR).calculate_greeks(&params, ExerciseStyle::American)?;
assert!(greeks.gamma > 0.0);
# Ok::<(), finstack_quant_core::Error>(())
```

`BinomialTree::leisen_reimer(steps)` logs a warning on an even step count;
`leisen_reimer_odd` rounds up instead. Additional entry points:
`price_bermudan(&params, &exercise_times)`, `price_barrier_out`,
`price_barrier_in`, `price_barrier_in_american`, `price_barrier_in_bermudan`,
and `price_generic::<V: TreeValuator>`.

`calculate_greeks` on `BinomialTree` returns `BinomialGreeks`
(`price`, `delta`, `gamma`, `theta` — no vega or rho) and accepts only
`ExerciseStyle::American` or `::European`. The trait-level
`TreeModel::calculate_greeks` returns the fuller `TreeGreeks`.

Richardson extrapolation for prices and Greeks:

```rust
use finstack_quant_models::trees::TreeGreeks;

let improved_price = TreeGreeks::richardson_price(price_n, price_2n);
let improved_greeks = TreeGreeks::richardson_extrapolate(&coarse, &fine);
```

The `(4·fine − coarse)/3` form is correct only at a refinement ratio of exactly
2 (fine uses 2N steps). For a general ratio `r` the weights become
`(r²·fine − coarse)/(r² − 1)`.

## Short-rate trees

| Model | Dynamics | Vol convention | Negative rates | Mean reversion |
|-------|----------|----------------|----------------|----------------|
| Ho-Lee | `dr = θ(t)dt + σdW` | Normal (rate units) | Yes | Not supported — breaks lattice recombination; use `HullWhiteTree` |
| BDT / Black-Karasinski | `d(ln r) = [θ(t) − κ·ln r]dt + σdW` | Lognormal (proportional) | No | κ = 0 → binomial BDT; κ > 0 → trinomial BK |

Calibration uses Arrow-Debreu forward induction to reproduce the input discount
curve exactly at every step. For κ > 0 the lattice is a genuine trinomial
Black-Karasinski tree in `x = ln r`, reusing the Hull-White trinomial geometry
(spacing `σ√(3dt)`, width cap with edge branch switching, per-node
mean-reverting probabilities) with a Brent solve on the per-step additive shift
in `x`.

```rust
use finstack_quant_models::trees::{ShortRateTree, ShortRateTreeConfig};

// Ho-Lee, 100 steps, 80 bp normal vol. Or: ShortRateTreeConfig::bdt(100, 0.20, 0.0)
let config = ShortRateTreeConfig::ho_lee(100, 0.008);
let mut tree = ShortRateTree::new(config);
tree.calibrate(&curve_id, discount_curve, time_to_maturity)?;

let rate = tree.rate_at_node(10, 3)?;
let (p_up, p_down) = tree.probabilities(10)?;
```

`ShortRateTreeConfig` fields: `steps`, `model` (`ShortRateModel::HoLee` or
`::BlackDermanToy`), `volatility`, `mean_reversion: Option<f64>`, `branching`,
and `compounding` (`TreeCompounding::{Continuous, Simple, SemiAnnual,
Quarterly, Monthly}` — Bloomberg's lognormal OAS model uses `Simple`).
Constructors `ho_lee`, `bdt`, `default_ho_lee`, `default_bdt` set consistent
defaults; `DEFAULT_NORMAL_VOL = 0.01` and `DEFAULT_LOGNORMAL_VOL = 0.20`.

`calibrate` takes `(&CurveId, &dyn Discounting, time_to_maturity)` — the curve
id is recorded so bumped repricing can find the same curve.

**Volatility conventions differ by model** and are not interchangeable:
Ho-Lee σ is absolute (50-150 bp, i.e. 0.005-0.015); BDT σ is proportional
(15-30%, i.e. 0.15-0.30). Convert with
`finstack_quant_models::volatility::convert_atm_volatility`.

**Node ordering differs by model.** Ho-Lee: node 0 is the *lowest* rate.
BDT (κ = 0, binomial): node 0 is the *highest* rate (`α·u^(n-1)`).
BK (κ > 0, trinomial): node 0 is the lowest (`j = −j_max`).

`short_rate_keys` supplies `SHORT_RATE` (`"interest_rate"`), `OAS`, `STEP`,
`NODE`, and `TIME`.

## Hull-White tree

Two-phase construction: build the lattice in auxiliary x-space where
`x(t) = r(t) − α(t)`, then calibrate `α(t)` by forward induction against the
discount curve.

```text
dr(t) = [θ(t) − κr(t)]dt + σdW(t)
dx(t) = −κx(t)dt + σdW(t)
```

Level spacing is `dx_i = σ√(3·dt_{i-1})`, matching the variance of the step
arriving at level `i`. Width is set by the natural branching geometry (each
node's central child plus one node either side), optionally hard-capped by
`config.max_nodes`; if that cap is too tight the transition probabilities go
negative and calibration fails loudly. Boundary handling follows Hull & White
(1994).

```rust
use finstack_quant_models::trees::{HullWhiteTree, HullWhiteTreeConfig};

let config = HullWhiteTreeConfig {
    kappa: 0.03,
    sigma: 0.01,
    steps: 100,
    max_nodes: None,
    ..Default::default()
};

// Exercise dates land exactly on grid points.
let tree = HullWhiteTree::calibrate_with_times(
    config,
    discount_curve,
    time_to_maturity,
    &exercise_times,
)?;

let price = tree.backward_induction(&terminal_values, |step, node, continuation| {
    continuation.max(exercise_value(step, node))
})?;
```

`backward_induction` takes terminal values indexed by node at the final step
(`terminal_values.len()` must equal `tree.num_nodes(tree.num_steps())`) and a
closure `(step, node_idx, continuation_value) -> f64`. `calibrate` is the
uniform-grid shorthand for `calibrate_with_times(.., &[])`;
`calibrate_with_times_and_volatility` additionally accepts a left-continuous
piecewise-constant `σ` schedule whose knots are merged into the grid so no
transition straddles a change.

| Parameter | Typical range |
|-----------|---------------|
| `kappa` | 0.01-0.10 |
| `sigma` | 0.005-0.015 (50-150 bp normal) |
| `steps` | 50-200 (cost is O(n²)) |

## Two-factor rates + credit tree

`RatesCreditTree` models the risk-free short rate and the credit hazard rate
jointly. Each factor is independently calibrated by Arrow-Debreu forward
induction — the rate factor to the discount curve (Ho-Lee style θ adjustment),
the hazard factor to the hazard curve's survival probabilities. Correlated
Bernoulli coupling produces four joint probabilities per node with
`cov = ρ√(var_r · var_h)`.

Node hazards pass through `max(raw, 0.0)` in both the forward recursion and the
backward induction, which is what makes them exact duals and lets the lattice
reproduce the survival curve as the valuator sees it.

```rust
use finstack_quant_models::trees::{RatesCreditConfig, RatesCreditTree};

let config = RatesCreditConfig {
    steps: 100,
    rate_vol: 0.01,
    hazard_vol: 0.02,
    correlation: 0.3,
    rate_mean_reversion: 0.0,
    hazard_mean_reversion: 0.0,
};
let mut tree = RatesCreditTree::new(config);
tree.calibrate(discount_curve, &hazard_curve, time_to_maturity)?;
```

`RatesCreditConfig::default()` is deterministic in **both** factors
(`rate_vol = hazard_vol = 0.0`). The lattice still reprices the discount and
survival curves exactly; it just carries no diffusion. A stochastic default
would let `..Default::default()` silently price optionality the caller never
asked for. `resolve_rates_credit_config` is the single mapping from an
instrument's pricing overrides to this config.

**Mean-reversion limit.** `calibrate` rejects `rate_mean_reversion` or
`hazard_mean_reversion` above `KAPPA_MAX = 0.15`. Discount-curve repricing
stays exact for any κ, but the fixed-geometry binomial lattice collapses the
conditional variance of the factor as κ grows, degrading option values for
callable bonds and term loans. **The binding quantity is `κ·T`, not `κ`:** at
κ = 0.15 the lattice edge retains at most 44% of its intended variance over 5
years and none beyond `T = 1/κ ≈ 6.7` years, where the up-probability clamps to
0 or 1 and those nodes — degenerate Bernoulli marginals carrying no correlation
— drop the configured correlation entirely. `KAPPA_MAX` is therefore a coarse
guard, not a proof of accuracy: read `rate_variance_retention()`,
`hazard_variance_retention()`, and `hazard_floor_saturation()` after
calibration for what the configured `(κ, σ, T, steps)` actually produced. Use
`HullWhiteTree` when κ·T approaches 1.

Other accessors: `max_feasible_correlation(ttm)`, `rate_at_node`,
`hazard_at_node`, `recovery_rate`, `conditional_discount_factors`, and
`price_with_node_coupons::<V: TreeValuator>`.

## Barriers

Discrete (per-step) barrier monitoring via `BarrierSpec`:

```rust
use finstack_quant_models::trees::{BarrierSpec, BarrierStyle};

let barrier = BarrierSpec {
    up_level: Some(120.0),
    down_level: None,
    rebate: 0.0,
    style: BarrierStyle::KnockOut,
};
```

The touch predicate is **non-strict**: `S >= up_level` and `S <= down_level`.
This is more conservative for knock-outs and matches Bloomberg (QuantLib
defaults to strict inequality). The engine enforces `KnockOut` directly; for
`KnockIn` it only tracks hit state, and the valuator decides.

`price_recombining_tree` writes `BARRIER_TOUCHED_UP` and
`BARRIER_TOUCHED_DOWN` at each node; a `TreeValuator` reads them via
`NodeState::barrier_touched_up()` / `barrier_touched_down()`, with
`is_knocked_out()` / `is_knocked_in()` / `is_barrier_hit()` for the aggregate
state.

## Greeks

The default `TreeModel::calculate_greeks` uses finite differences on the
initial state map. `bump_size` defaults to `0.01`.

| Greek | Scheme | Bump | Reported per |
|-------|--------|------|--------------|
| Delta | Central on `SPOT` | `bump_size × spot` | Unit of spot |
| Gamma | Second-order central on `SPOT` | `bump_size × spot` | Unit of spot squared |
| Vega | Central on `VOLATILITY` | 0.01 absolute (down leg floored at 1e-6) | 1% absolute vol move |
| Rho | Central on `INTEREST_RATE` | 0.0001 | 1 bp rate move |
| Theta | Forward on time | `1/365.25` years | Year |

A Greek is computed only if its state key is present in `initial_vars`; theta
is skipped when `time_to_maturity <= 1/365.25`.

Theta is `-(V(T) - V(T - dt)) / dt` with `dt` in **years**, so the returned
number is an annualized `∂V/∂t`, not a per-day decay — divide by 365.25 for a
daily figure. `BinomialTree::calculate_greeks` uses the same convention. The
doc comment on `TreeGreeks` describes theta as per-day and contradicts the
implementation.

## State variables

Nodes carry a `HashMap<&'static str, f64>` keyed by `state_keys`:

| Constant | Key | Meaning |
|----------|-----|---------|
| `SPOT` | `"spot"` | Underlying asset price |
| `INTEREST_RATE` | `"interest_rate"` | Risk-free short rate |
| `CREDIT_SPREAD` | `"credit_spread"` | Credit spread |
| `HAZARD_RATE` | `"hazard_rate"` | Default intensity |
| `DIVIDEND_YIELD` | `"dividend_yield"` | Continuous dividend yield |
| `VOLATILITY` | `"volatility"` | Volatility |
| `RATE_VOLATILITY` | `"rate_volatility"` | Rate vol for two-factor equity+rates models |
| `DF` | `"df"` | Pre-computed per-node discount factor |
| `BARRIER_TOUCHED_UP` | `"barrier_touched_up"` | 1.0 / 0.0 flag |
| `BARRIER_TOUCHED_DOWN` | `"barrier_touched_down"` | 1.0 / 0.0 flag |

`NodeState` pre-extracts `spot`, `interest_rate`, `credit_spread`,
`hazard_rate`, and `discount_factor` into cached fields so the hot path avoids
hash lookups; the accessors return `Option<f64>`. `get_var` / `get_var_or`
reach anything else. Helper builders `single_factor_equity_state` and
`two_factor_equity_rates_state` assemble the common maps.

## Usage in the codebase

| Instrument | Model | Location |
|------------|-------|----------|
| American / Bermudan equity options | `BinomialTree::leisen_reimer` | `instruments/equity/equity_option/pricer.rs` |
| Commodity options | `BinomialTree::leisen_reimer_odd` | `instruments/commodity/commodity_option/types.rs` |
| Callable / putable bonds | `ShortRateTree`, `RatesCreditTree`, `HullWhiteTree` | `instruments/fixed_income/bond/pricing/engine/tree/` |
| Term loans | `ShortRateTree`, `RatesCreditTree` | `instruments/fixed_income/term_loan/pricing/tree_engine.rs` |
| Bermudan swaptions | `HullWhiteTree::calibrate_with_times` | `instruments/rates/swaption/` |
| Convertible bonds | Tsiveriotis-Zhang engine over `EvolutionParams` (binomial or trinomial) | `instruments/fixed_income/convertible/pricer/` |

## Serialization

Tree models and their configuration types are runtime-only and implement
neither `Serialize` nor `Deserialize`. They are constructed on demand during
pricing and are not part of any persistent JSON schema. If persistence is ever
needed (scenario storage, calibration caching), add serde to the configuration
structs (`TreeParameters`, `EvolutionParams`) only and keep the runtime engine
types non-serializable.

## Performance

- Complexity as tabulated above; step-count guidance: 50 for fast estimates,
  100-200 for production, 200+ with Richardson extrapolation for high
  precision.
- `NodeState` caching removes hash lookups from the inner loop.
- Parallel Greeks, node-value caching, and SIMD are deliberately deferred to
  keep the engine simple and deterministic.

## Verification

```bash
# Unit tests for this module (never `cargo test` — it would run doc tests).
cargo nextest run -p finstack-quant-valuations --lib -E 'test(/models::trees/)'

# One model at a time.
cargo nextest run -p finstack-quant-valuations --lib -E 'test(/trees::short_rate_tree/)'
cargo nextest run -p finstack-quant-valuations --lib -E 'test(/hull_white/)'

mise run rust-test
mise run rust-lint

# Criterion suite. No bench targets these lattices directly; they are exercised
# through the instrument benches (option_pricing, bond_pricing,
# convertible_pricing, swaption_pricing).
mise run rust-bench
```

## Adding a tree model

1. Add the file (or directory) and declare it in [`mod.rs`](mod.rs) with the
   public re-exports.
2. Implement `TreeModel`. The shortest path is to build a `RecombiningInputs`
   and call `price_recombining_tree(inputs)` — note it takes the struct **by
   value**. Fields: `branching`, `steps`, `initial_vars`, `time_to_maturity`,
   `market_context`, `valuator`, `up_factor`, `down_factor`, `middle_factor`,
   `prob_up`, `prob_down`, `prob_middle`, `interest_rate`, `barrier`,
   `custom_state_generator`, `custom_rate_generator`.
3. Implement `TreeValuator` for the instrument: terminal payoff in
   `value_at_maturity`, the hold-vs-exercise decision in `value_at_node`.
4. For calibrated trees, add a `calibrate()` that stores per-node state
   privately and inject it through `custom_state_generator` /
   `custom_rate_generator`.
5. Add any new state keys to `state_keys`; add a cached `NodeState` field only
   if the key is read on the hot path.
6. Derive evolution parameters through `EvolutionParams::equity_crr` /
   `equity_trinomial` / `with_drift` where possible — they validate that the
   risk-neutral probabilities lie in `[0, 1]` and sum to one in release builds,
   which is the guard against silent lattice arbitrage.

## References

- Cox, J., Ross, S. & Rubinstein, M. (1979). "Option Pricing: A Simplified
  Approach." *Journal of Financial Economics*, 7(3), 229-263.
- Jarrow, R. & Rudd, A. (1983). *Option Pricing*. Irwin.
- Ho, T. & Lee, S. (1986). "Term Structure Movements and Pricing Interest Rate
  Contingent Claims." *Journal of Finance*, 41(5), 1011-1029.
- Boyle, P. (1986). "Option Valuation Using a Three-Jump Process."
  *International Options Journal*, 3, 7-12. (Trinomial construction used by
  `EvolutionParams::equity_trinomial`.)
- Black, F., Derman, E. & Toy, W. (1990). "A One-Factor Model of Interest Rates
  and Its Application to Treasury Bond Options." *Financial Analysts Journal*,
  46(1), 33-39.
- Black, F. & Karasinski, P. (1991). "Bond and Option Pricing when Short Rates
  are Lognormal." *Financial Analysts Journal*, 47(4), 52-59.
- Tian, Y. (1993). "A Modified Lattice Approach to Option Pricing." *Journal of
  Futures Markets*, 13(5), 563-577.
- Hull, J. & White, A. (1994). "Numerical Procedures for Implementing Term
  Structure Models I: Single-Factor Models." *Journal of Derivatives*, 2(1),
  7-16.
- Tsiveriotis, K. & Fernandes, C. (1998). "Valuing Convertible Bonds with
  Credit Risk." *Journal of Fixed Income*, 8(2), 95-102.
- Broadie, M. & Detemple, J. (1996). "American Option Valuation: New Bounds,
  Approximations, and a Comparison of Existing Methods." *Review of Financial
  Studies*, 9(4), 1211-1250.
- Leisen, D. & Reimer, M. (1996). "Binomial Models for Option Valuation —
  Examining and Improving Convergence." *Applied Mathematical Finance*, 3(4),
  319-346.
- Hull, J. (2018). *Options, Futures, and Other Derivatives* (10th ed.),
  ch. 31.

Full bibliography with stable anchors: [docs/REFERENCES.md](../../../../../docs/REFERENCES.md).
