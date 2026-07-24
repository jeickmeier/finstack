# Worked Example Audits

Two condensed examples showing the expected output shape and judgment level.
Real audits will have more steps; the point here is calibration — what a Pass,
Fail, and Uncertain look like, and how findings tie back to step IDs.

## Example 1 — Bond dirty price walkthrough (Fail found)

Subject code (illustrative):

```rust
fn dirty_price(bond: &Bond, curve: &Curve, trade_date: NaiveDate) -> f64 {
    let settle = trade_date; // settlement lag not applied
    let accrued = accrued_interest(bond, settle);
    let pv: f64 = bond
        .cashflows_after(settle)
        .iter()
        .map(|cf| cf.amount * curve.df(trade_date, cf.date))
        .sum();
    pv + accrued
}
```

Expected audit (abridged):

```markdown
## Subject
Walkthrough of `dirty_price` for a fixed-coupon bond: settlement, accrual,
cashflow selection, discounting, and price assembly. Fixed income, single
function boundary.

## Step Map
1. Settlement date determination
2. Accrued interest
3. Cashflow selection
4. Discounting
5. Dirty price assembly

## Step Audits

### Step 1 — Settlement date determination
- **Intent:** Derive settlement from trade date.
- **Standard:** Corporate/govvy bonds settle T+1 or T+2 on the relevant
  calendar (fixed-income-standards.md, settlement section).
- **Evidence:** `pricer.rs:2` — `let settle = trade_date;` uses trade date
  directly, no lag, no calendar.
- **Verdict:** Fail
- **Severity:** Major
- **Fix:** Compute `settle = calendar.advance(trade_date, lag)` with the lag
  from bond reference data; thread it through accrual and cashflow selection.

### Step 2 — Accrued interest
- **Intent:** Accrue coupon from last coupon date to settlement.
- **Standard:** Day count per bond terms from previous coupon to settlement.
- **Evidence:** `pricer.rs:3` — correct helper, but receives the wrong
  `settle` from Step 1.
- **Verdict:** Pass (conditional on Step 1 fix)
- **Severity:** n/a
- **Fix:** None beyond Step 1.

### Step 4 — Discounting
- **Intent:** Discount each cashflow to the pricing anchor.
- **Standard:** Bond PV is discounted to the settlement date; discounting to
  trade date misstates price by the lag's carry.
- **Evidence:** `pricer.rs:7` — `curve.df(trade_date, cf.date)` anchors on
  trade date.
- **Verdict:** Fail
- **Severity:** Major
- **Fix:** Anchor discounting at `settle`: `curve.df(settle, cf.date)`.

## Cross-Step Contracts
- Step 1 → Steps 2/3/4: a single settlement date must be computed once and
  reused; currently the wrong date propagates consistently, which will make
  the two Step-1/Step-4 fixes look like a price jump — change both together.

## Findings
1. [Major, Step 1, pricer.rs:2] No settlement lag; accrual, cashflow cutoff,
   and discount anchor all shift by 1–2 days. Fix: calendar-aware settlement.
2. [Major, Step 4, pricer.rs:7] Discounting anchored at trade date instead of
   settlement. Fix: anchor at settlement.

## Open Questions
- Which settlement lag and calendar apply (T+1 UST vs T+2 corporate)? Not in
  the bond struct — needs reference data confirmation.

## Brief Summary
Structure is standard (dirty = PV + accrued) but both date anchors are wrong.
Prices are biased by short-end carry over the settlement lag. Fix Steps 1 and
4 together and re-verify against a golden price.
```

Judgment notes:

- Step 2 shows a *conditional Pass*: the step's own logic is right, the input
  is wrong. Attribute the defect to the step that owns it, not every step it
  contaminates.
- The cross-step section warns that fixing one anchor without the other
  produces a confusing intermediate state — that is exactly the kind of
  between-steps insight this section exists for.

## Example 2 — DV01 bump path (Uncertain done right)

Situation: a `dv01` function bumps the zero curve +1bp and reprices, one-sided.

```markdown
### Step 3 — Bump specification
- **Intent:** Apply a 1bp parallel shift to produce rate risk.
- **Standard:** Two desk-standard approaches exist: bump raw quotes and
  re-bootstrap (matches hedge instruments) or bump the zero curve (faster,
  smoother). Both are used in practice; the choice must be documented because
  bucketed risk differs between them
  (algorithm-standards.md, risk section).
- **Evidence:** `position_risk.rs:88-95` — bumps zero rates directly; no doc
  comment stating the policy.
- **Verdict:** Uncertain
- **Severity:** Moderate
- **Fix:** Document the zero-bump policy in the rustdoc and metric key docs,
  or switch to quote-bump-and-rebootstrap if hedge-consistent bucketed risk
  is required. Confirm which one downstream consumers assume.
```

Judgment notes:

- Uncertain is the honest verdict when two legitimate desk practices exist and
  the code does not say which it implements. The fix is documentation plus a
  question to the owner — not an assumed rewrite.
- Severity is Moderate, not Blocker: numbers are internally consistent, but a
  consumer expecting quote-space risk would misuse them.
