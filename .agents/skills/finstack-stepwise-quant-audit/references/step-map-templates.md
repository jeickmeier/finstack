# Step Map Templates by Process Type

Canonical step decompositions for common quant processes. Use these as the
starting skeleton when building the step map, then adapt to the actual code:
merge steps the implementation combines, split steps it separates, and drop
steps that are out of the walkthrough boundary. Every template step lists the
standards that most often decide its verdict.

Do not force the code into a template. If the implementation orders steps
differently, audit the order itself — a nonstandard order is sometimes the bug
(e.g. rounding coupon amounts before aggregation, bumping after caching).

## Table of contents

1. Bond pricing (dirty/clean price, yield)
2. Vanilla swap pricing (OIS/RFR)
3. Curve bootstrap
4. Option pricing (European, Black/Black-Scholes)
5. Risk / Greeks path (bump-and-reprice)
6. Cashflow schedule generation
7. Model calibration
8. Monte Carlo valuation
9. CDS / credit pricing

---

## 1. Bond pricing

1. **Trade and reference data validation** — coupon, frequency, day count,
   maturity, first/last coupon dates, ex-dividend rule if applicable
2. **Settlement date determination** — trade date + settlement lag on the right
   calendar (T+1 US Treasuries, T+2 most corporates/govvies)
3. **Coupon schedule construction** — unadjusted dates from maturity backward
   (or dated date forward), stub identification, business-day adjustment for
   payment vs accrual
4. **Accrued interest** — day count applied from last coupon to settlement;
   check ex-dividend negative accrual where relevant (e.g. UK gilts)
5. **Cashflow projection** — coupon amounts (rate x day-count fraction x
   notional), principal at maturity, odd stubs prorated correctly
6. **Discounting** — curve role (govvy/z-spread/repo), discount to settlement
   date not trade date
7. **Clean/dirty assembly** — dirty = PV of remaining cashflows; clean = dirty −
   accrued; verify which one the function returns and which the market quotes
8. **Yield/price round-trip** — solver choice, compounding convention of the
   quoted yield (street vs true), tolerance

Common blockers: discounting to the wrong date; clean/dirty confusion; wrong
first-coupon stub accrual; yield compounding convention mismatch.

## 2. Vanilla swap pricing (OIS/RFR)

1. **Trade validation** — notional, direction (pay/receive fixed), effective and
   termination dates, index (SOFR, ESTR, SONIA), payment lag
2. **Schedule construction per leg** — frequency, roll convention, stub policy,
   payment lag (typically 2 business days for SOFR OIS)
3. **Fixed leg accruals** — day count (ACT/360 USD, ACT/360 EUR OIS, ACT/365F
   GBP), fixed coupon amounts
4. **Floating leg projection** — daily compounded in-arrears RFR from the
   projection curve; past fixings from history, future from curve; lockout or
   lookback if the trade specifies one
5. **Curve role assignment** — projection vs discount curve; for cleared or
   CSA trades the discount curve reflects collateral (OIS in collateral
   currency)
6. **Leg PV and netting** — sign convention: payer of fixed sees fixed leg
   negative; NPV = receive leg − pay leg from the valuation party's view
7. **Par rate / NPV consistency** — at the par rate, NPV ≈ 0; useful internal
   cross-check

Common blockers: using projection curve for discounting; missing payment lag;
compounding simple-averaged instead of geometrically compounded RFR; sign flip
between legs.

## 3. Curve bootstrap

1. **Instrument set validation** — sorted by maturity, no duplicates, quotes in
   the expected unit (rate vs price vs spread)
2. **Node placement** — one node per instrument maturity (or documented
   otherwise); anchor DF(0) = 1
3. **Short-end handling** — deposits/futures/FRAs: convexity adjustment for
   futures if used
4. **Sequential or global solve** — sequential Newton per node is standard;
   global solve needs a documented reason; tolerance ~1e-12 on reprice
5. **Interpolation choice** — log-linear on DF or monotone convex on forwards;
   interpolation applied on the documented quantity (see
   `../finstack-quant-finance-review/market-standards/algorithm-standards.md`)
6. **Reprice check** — every input instrument reprices to its quote within
   tolerance; this is the certification step
7. **Extrapolation policy** — explicit flat/linear beyond last node; no silent
   panics or NaN
8. **Forward smoothness inspection** — no sawtooth forwards unless the
   instrument set genuinely implies them

Common blockers: interpolating on zero rates while claiming DF interpolation;
reprice check missing or loose; negative forwards from oscillating splines.

## 4. Option pricing (European, Black/Black-Scholes)

1. **Input validation** — spot/forward, strike, expiry, vol, rates/carry; reject
   or document negative time, zero vol handling
2. **Time to expiry** — day count for vol time (typically ACT/365F calendar
   time) vs discounting time; be explicit if they differ
3. **Forward construction** — from spot with carry (dividends, foreign rate,
   repo) or taken directly; consistent with the curve setup
4. **d1/d2 and price formula** — correct signs, vol*sqrt(T) in the right
   places; Black (forward) vs Black-Scholes (spot) distinguished
5. **Discounting** — discount factor on the numeraire/domestic curve to
   settlement of the premium, not necessarily expiry
6. **Edge regimes** — zero vol → intrinsic on forward; T→0 → intrinsic; deep
   ITM/OTM → no NaN from erf/exp overflow
7. **Units of output** — premium currency and per-unit vs total notional
   (especially FX: pips vs percent, domestic vs foreign)

Common blockers: vol time vs discount time mixed; FX premium currency wrong;
zero-vol branch missing.

## 5. Risk / Greeks path (bump-and-reprice)

1. **Base valuation snapshot** — reproducible base PV with fixed market data
2. **Bump specification** — unit (1bp rate, 1% vol point, absolute vs relative),
   direction, and target (node, curve-parallel, spot)
3. **Market data rebuild** — bumped object rebuilt consistently: rebumping raw
   quotes and re-bootstrapping vs bumping the zero curve produces different
   risk; the method must be documented and consistent
4. **Reprice** — same pricing path as base; no cached values from the base run
   leaking in
5. **Difference and scaling** — (PV_up − PV_base)/bump or central difference;
   scaling to per-1bp or per-1% stated
6. **Aggregation and keys** — metric keys and bucket labels consistent
   (`bucketed_dv01::USD-OIS::10y` style); sum of buckets ≈ parallel within
   convexity tolerance
7. **Sign convention** — DV01 sign for long/short; report convention (positive
   = gain when rates fall?) stated

Common blockers: cache leakage across bump; bumping zeros vs quotes silently;
one-sided difference where central is claimed; bucket sum ≠ parallel.

## 6. Cashflow schedule generation

1. **Anchor and direction** — backward from maturity (market default) or
   forward from effective; IMM/EOM roll flags
2. **Unadjusted date grid** — frequency stepping without business-day
   adjustment first
3. **Stub resolution** — short/long, front/back per trade terms; no silent
   dropping of the odd period
4. **Business-day adjustment** — convention (MF, F, P) and calendar(s) applied
   to payment dates; accrual dates adjusted or not per trade convention
   (adjusted vs unadjusted accrual)
5. **Accrual fractions** — day count per leg over adjusted or unadjusted
   periods as specified
6. **Fixing dates** — offset from accrual start (or end for in-arrears), fixing
   calendar distinct from payment calendar where relevant
7. **Invariants** — periods contiguous, no overlaps/gaps, first accrual start =
   effective date, last payment = adjusted maturity

Common blockers: adjusting accrual dates when the convention says unadjusted;
stub at the wrong end; fixing calendar = payment calendar assumption.

## 7. Model calibration

1. **Target set construction** — instruments, weights, and quote types; exclude
   stale/crossed quotes explicitly
2. **Parameter bounds and initial guess** — documented, financially meaningful
   bounds (e.g. SABR beta fixed or bounded, vol-of-vol > 0)
3. **Objective function** — price vs vol space error, relative vs absolute,
   vega-weighted or not — each changes the fit and must be stated
4. **Optimizer** — algorithm, convergence tolerance, max iterations, behavior
   on non-convergence (fail loudly, never return the last iterate silently)
5. **Fit quality report** — per-instrument residuals, not just aggregate RMSE
6. **Stability/reproducibility** — same inputs → same parameters; sensitivity
   to initial guess documented
7. **Arbitrage sanity** — calibrated surface/curve free of butterfly/calendar
   arbitrage where the model claims it

Common blockers: silent non-convergence; objective space undocumented;
parameters at bounds unreported.

## 8. Monte Carlo valuation

1. **Model and discretization** — SDE, scheme (Euler, Milstein, exact),
   step size justification
2. **Random number generation** — generator, seeding policy (reproducible),
   dimension ordering for Sobol/quasi
3. **Path construction** — martingale check under the pricing measure (E[S_T]
   ≈ forward); correlation via Cholesky validated PSD
4. **Payoff evaluation** — path payoff matches term sheet including barriers,
   averaging windows, and observation calendars
5. **Discounting** — pathwise numeraire or deterministic DF consistent with the
   measure
6. **Estimator and error** — mean plus standard error reported; convergence
   criterion, variance reduction (antithetic, control variate) correctness
7. **Bias controls** — discretization bias vs statistical error separated;
   barrier monitoring bias (discrete vs continuous) addressed

Common blockers: drift under the wrong measure; barrier monitored at step dates
but term sheet says continuous; standard error not reported.

## 9. CDS / credit pricing

1. **Contract terms** — standard coupon (100/500bp), maturity on IMM 20th
   grid, ACT/360 premium accrual
2. **Curve inputs** — hazard/survival curve role vs risk-free discount curve
   role kept distinct
3. **Premium leg** — accrual on survival, plus accrual-on-default treatment
   (standard: half-period approximation or integral)
4. **Protection leg** — integral of (1−R) x dPD x DF; recovery convention
   (fixed 40% senior unsecured unless specified)
5. **Upfront/par spread conversion** — ISDA standard model conventions if
   claiming market-standard quotes
6. **Risk outputs** — CS01 from spread bump with re-bootstrapped hazard curve;
   units per 1bp

Common blockers: accrual-on-default dropped; discounting protection leg on the
hazard curve; non-IMM schedule for standard contracts.
