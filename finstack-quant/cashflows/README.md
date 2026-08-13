# finstack-quant-cashflows

Cashflow schedule construction, accrual, and aggregation for bonds, loans,
swaps, and structured products.

The crate provides:

- `CashFlowSchedule::builder()` for principal, coupon, amortization, fee, and
  credit legs
- schedule-driven accrued interest
- currency-preserving aggregation and period present value
- serde-first JSON construction and validation

Add `finstack-quant-cashflows` as a direct dependency when using these APIs;
the Rust import path is `finstack_quant_cashflows`. Build the crate docs with
`cargo doc -p finstack-quant-cashflows --open`.

Amounts use `Money` with an explicit currency. Coupon rates are decimals
(`0.05` means 5%); fields named with `_bp` are basis points. `ScheduleParams`
owns frequency, day count, calendars, business-day adjustment, stubs, roll
rules, accrual-date adjustment, and payment lags.

## Example

This fixed coupon uses the semiannual 30/360 schedule preset:

```rust
use finstack_quant_cashflows::builder::{
    CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams,
};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::money::Money;
use rust_decimal_macros::dec;
use time::Month;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let issue = Date::from_calendar_date(2025, Month::January, 15)?;
    let maturity = Date::from_calendar_date(2026, Month::January, 15)?;

    let schedule = CashFlowSchedule::builder()
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .fixed_cf(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: dec!(0.05),
            schedule: ScheduleParams::semiannual_30360(),
        })
        .build(None)?;

    assert!(!schedule.get_flows().is_empty());
    Ok(())
}
```
