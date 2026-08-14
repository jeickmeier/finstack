//! Bond pricing methods, validation, and cashflow projection.

use crate::instruments::common_impl::validation;
use finstack_quant_core::dates::Date;
use finstack_quant_core::money::Money;
use finstack_quant_core::Result;
use rust_decimal::prelude::ToPrimitive;

use super::definitions::Bond;
use super::CashflowSpec;

impl Bond {
    /// Pricing-oriented dated cashflows: coupons, amortization, and positive
    /// notional (redemption). Negative notionals (initial draw) and pure PIK
    /// accretion are excluded because they are not discounted receipt flows.
    ///
    /// When the bond has an ex-coupon convention and `as_of` falls inside the
    /// ex-coupon window of a coupon, that coupon is excluded: a buyer settling
    /// in the ex-window does not receive the imminent coupon (market standard,
    /// e.g. UK gilts). Accrued interest is correspondingly negative in that
    /// window (see [`crate::cashflow::accrued_interest_amount`]).
    ///
    /// Internal pricing engines (discount, hazard, spread solvers) should use
    /// this instead of the public [`CashflowProvider::dated_cashflows`] which
    /// now returns the full signed canonical schedule.
    ///
    /// Cashflows dated exactly on `as_of` are **excluded** (strict
    /// `date > as_of`): a buyer settling on `as_of` does not receive that
    /// day's payment (settlement convention). This matches the tree and YTM
    /// engines, which always filtered `> as_of`.
    pub(crate) fn pricing_dated_cashflows(
        &self,
        curves: &finstack_quant_core::market_data::context::MarketContext,
        as_of: finstack_quant_core::dates::Date,
    ) -> Result<
        Vec<(
            finstack_quant_core::dates::Date,
            finstack_quant_core::money::Money,
        )>,
    > {
        self.pricing_dated_cashflows_at(curves, as_of, as_of)
    }

    /// [`Bond::pricing_dated_cashflows`] with an explicit coupon-entitlement date.
    ///
    /// Market convention (e.g. UK gilts/DMO) determines coupon entitlement by
    /// whether the **settlement** date falls on/after the ex-dividend date, not
    /// the trade/valuation date. Quote-derived metrics (YTM, YTW, Z-spread, DM,
    /// I-spread, ASW) therefore pass the quote/settlement date as
    /// `entitlement_date` so that a trade whose settlement lands inside the
    /// ex-window drops the imminent coupon, consistent with the negative
    /// accrued interest computed at the same date.
    ///
    /// # Arguments
    ///
    /// * `curves` - Market context used to build the full cashflow schedule
    ///   (floating-rate projection, amortization, custom schedules).
    /// * `as_of` - Valuation date; flows dated on or before it are excluded
    ///   (strict `date > as_of`).
    /// * `entitlement_date` - Date at which coupon entitlement is tested
    ///   against each coupon's ex-date; the quote/settlement date for market
    ///   quotes, `as_of` for model PV.
    pub(crate) fn pricing_dated_cashflows_at(
        &self,
        curves: &finstack_quant_core::market_data::context::MarketContext,
        as_of: finstack_quant_core::dates::Date,
        entitlement_date: finstack_quant_core::dates::Date,
    ) -> Result<
        Vec<(
            finstack_quant_core::dates::Date,
            finstack_quant_core::money::Money,
        )>,
    > {
        use finstack_quant_core::cashflow::CFKind;

        let ex_coupon = self.accrual_config().ex_coupon;
        let schedule = self.full_cashflow_schedule(curves)?;
        let mut flows = Vec::with_capacity(schedule.get_flows().len());
        for cf in schedule.get_flows() {
            let keep = cf.date > as_of
                && cf.kind != CFKind::Pik
                && !(cf.kind == CFKind::Notional && cf.amount.amount() < 0.0);
            if !keep {
                continue;
            }
            // Drop interest flows whose ex-date has passed: the buyer entitled
            // as of `entitlement_date` does not receive them.
            if cf.kind.is_interest_like() {
                if let Some(rule) = &ex_coupon {
                    let ex_date = rule.ex_date(cf.date)?;
                    if entitlement_date >= ex_date && entitlement_date < cf.date {
                        continue;
                    }
                }
            }
            flows.push((cf.date, cf.amount));
        }
        Ok(flows)
    }

    /// Cashflow schedule enriched with discount factors, survival probabilities, and PVs.
    ///
    /// Builds the bond's full internal cashflow schedule
    /// and computes per-cashflow discount factors and (when a credit curve is configured)
    /// survival probabilities, returning a
    /// [`crate::cashflow::builder::PeriodDataFrame`] that is ready for tabular
    /// export or further analysis.
    ///
    /// # Arguments
    /// * `market` - Market context containing discount and optional hazard curves
    /// * `as_of` - Valuation date; defaults to the discount curve's base date when `None`
    ///
    /// # Returns
    /// A [`crate::cashflow::builder::PeriodDataFrame`] with `discount_factors`,
    /// optional `survival_probs`, and `pvs`.
    pub fn pricing_cashflows(
        &self,
        market: &finstack_quant_core::market_data::context::MarketContext,
        as_of: Option<Date>,
    ) -> Result<crate::cashflow::builder::PeriodDataFrame> {
        use crate::cashflow::builder::PeriodDataFrameOptions;
        use finstack_quant_core::dates::{Period, PeriodId};

        let schedule = self.full_cashflow_schedule(market)?;

        let periods: Vec<Period> = if let (Some(first), Some(last)) =
            (schedule.get_flows().first(), schedule.get_flows().last())
        {
            vec![Period {
                id: PeriodId::annual(first.date.year()),
                start: first.date,
                end: last.date,
                is_actual: true,
            }]
        } else {
            Vec::new()
        };

        let options = PeriodDataFrameOptions {
            credit_curve_id: self.credit_curve_id.as_ref().map(|id| id.as_str()),
            as_of,
            ..Default::default()
        };

        schedule.to_period_dataframe(&periods, market, self.discount_curve_id.as_str(), options)
    }

    /// Price bond using tree-based pricing for embedded options (calls/puts).
    ///
    /// This method is automatically called by `value()` when the bond has a non-empty
    /// call/put schedule. It uses a short-rate tree model to properly value the
    /// embedded optionality via backward induction.
    ///
    /// # Arguments
    /// * `market` - Market context with discount curve (and optionally hazard curve)
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// Option-adjusted present value of the bond
    pub(crate) fn value_with_tree(
        &self,
        market: &finstack_quant_core::market_data::context::MarketContext,
        as_of: finstack_quant_core::dates::Date,
    ) -> finstack_quant_core::Result<finstack_quant_core::money::Money> {
        use crate::instruments::fixed_income::bond::pricing::engine::tree::{
            bond_tree_config, TreePricer,
        };

        // Use the same dispatch as OAS/quote conversions so direct callable PV
        // honors BDT, Hull-White, hazard-tree, and tree-curve overrides.
        let config = bond_tree_config(self)?;
        let price_amount =
            TreePricer::with_config(config).price_at_oas(self, market, as_of, 0.0)?;

        Ok(Money::new(price_amount, self.notional.currency()))
    }

    /// Validate all bond parameters.
    ///
    /// Performs comprehensive validation of the bond instrument:
    /// - Issue date must be before maturity date
    /// - Notional must be positive
    /// - Coupon rate must be non-negative (for fixed-rate bonds)
    /// - Call/put prices must be positive
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` with a descriptive message if any validation fails.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_quant_valuations::instruments::Bond;
    ///
    /// # fn main() -> finstack_quant_core::Result<()> {
    /// let bond = Bond::example()?;
    /// bond.validate()?; // Validates all parameters
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<()> {
        self.instrument_pricing_overrides.validate()?;
        self.metric_pricing_overrides.validate()?;
        self.scenario_pricing_overrides.validate()?;
        validation::validate_date_range_strict_with(
            self.issue_date,
            self.maturity,
            |start, end| {
                format!(
                    "Bond issue date ({}) must be before maturity date ({})",
                    start, end
                )
            },
        )?;

        validation::validate_money_finite(self.notional, "bond notional")?;
        validation::validate_money_gt_with(self.notional, 0.0, |amount| {
            format!("Bond notional must be positive, got {}", amount)
        })?;

        // Validate coupon rate for fixed-rate bonds (including amortizing with fixed base)
        Self::validate_coupon_rate(&self.cashflow_spec)?;

        // Validate call/put prices and exercise date ranges.
        if let Some(ref call_put) = self.call_put {
            call_put.validate_for_life(self.issue_date, self.maturity, "Bond")?;
        }

        Ok(())
    }

    /// Returns `true` when coupon cashflows depend on forward curve projection (floating FRNs).
    ///
    /// True for [`CashflowSpec::Floating`] and for [`CashflowSpec::Amortizing`] when the
    /// base specification is floating.
    pub fn has_floating_coupons(&self) -> bool {
        match &self.cashflow_spec {
            CashflowSpec::Floating(_) => true,
            CashflowSpec::Amortizing { base, .. } => {
                matches!(base.as_ref(), CashflowSpec::Floating(_))
            }
            _ => false,
        }
    }

    /// Recursively validate that fixed coupon rates are non-negative.
    ///
    /// Handles `Fixed`, `Floating` (no coupon rate to validate), and
    /// `Amortizing` (recurses into the base spec).
    fn validate_coupon_rate(spec: &CashflowSpec) -> Result<()> {
        match spec {
            CashflowSpec::Fixed(s) => {
                let rate = s.rate.to_f64().unwrap_or(0.0);
                if rate < 0.0 {
                    return Err(finstack_quant_core::Error::Validation(format!(
                        "Bond fixed coupon rate must be non-negative, got {}",
                        rate
                    )));
                }
            }
            CashflowSpec::StepUp(s) => {
                let rate = s.initial_rate.to_f64().unwrap_or(0.0);
                if rate < 0.0 {
                    return Err(finstack_quant_core::Error::Validation(format!(
                        "Bond step-up initial coupon rate must be non-negative, got {}",
                        rate
                    )));
                }
                for (_, step_rate) in &s.step_schedule {
                    let r = step_rate.to_f64().unwrap_or(0.0);
                    if r < 0.0 {
                        return Err(finstack_quant_core::Error::Validation(format!(
                            "Bond step-up coupon rate must be non-negative, got {}",
                            r
                        )));
                    }
                }
            }
            CashflowSpec::Amortizing { base, .. } => {
                Self::validate_coupon_rate(base)?;
            }
            CashflowSpec::Floating(_) => {
                // No fixed coupon rate to validate
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::{CallPut, CallPutSchedule, MakeWholeSpec};
    use finstack_quant_core::currency::Currency;
    use finstack_quant_core::market_data::context::MarketContext;
    use finstack_quant_core::types::CurveId;
    use time::macros::date;

    fn ex_coupon_bond() -> Bond {
        let mut bond = Bond::fixed(
            "EX-FLOWS",
            Money::new(100.0, Currency::USD),
            0.05,
            date!(2025 - 01 - 01),
            date!(2030 - 01 - 01),
            "USD-OIS",
        )
        .expect("valid bond");
        bond.settlement_convention = Some(super::super::BondSettlementConvention {
            ex_coupon_days: 7,
            ..Default::default()
        });
        bond
    }

    #[test]
    fn pricing_flows_exclude_coupon_inside_ex_window() {
        let bond = ex_coupon_bond();
        let coupon_date = date!(2025 - 07 - 01);
        let market = MarketContext::new();

        // Inside the ex-window (5 days before the coupon): the imminent coupon
        // is not a buyer flow.
        let as_of = coupon_date - time::Duration::days(5);
        let flows = bond.pricing_dated_cashflows(&market, as_of).expect("flows");
        assert!(
            !flows.iter().any(|(d, _)| *d == coupon_date),
            "ex-period coupon must be excluded from buyer flows"
        );

        // Outside the ex-window (8 days before): the coupon is still a buyer flow.
        let as_of = coupon_date - time::Duration::days(8);
        let flows = bond.pricing_dated_cashflows(&market, as_of).expect("flows");
        assert!(
            flows.iter().any(|(d, _)| *d == coupon_date),
            "cum-coupon flows must include the next coupon"
        );
    }

    /// Entitlement is a settlement-date rule: a trade whose settlement lands
    /// inside the ex-window must drop the coupon even when the trade date is
    /// still cum-coupon.
    #[test]
    fn entitlement_date_controls_ex_window_exclusion() {
        let bond = ex_coupon_bond();
        let coupon_date = date!(2025 - 07 - 01);
        let market = MarketContext::new();

        // Trade 8 days before the coupon (cum at trade date), settling 5 days
        // before (inside the 7-day ex-window): the coupon is not a buyer flow.
        let trade_date = coupon_date - time::Duration::days(8);
        let settle_date = coupon_date - time::Duration::days(5);
        let flows = bond
            .pricing_dated_cashflows_at(&market, trade_date, settle_date)
            .expect("flows");
        assert!(
            !flows.iter().any(|(d, _)| *d == coupon_date),
            "coupon must be excluded when the entitlement (settlement) date is in the ex-window"
        );

        // Same trade date with entitlement also at the trade date keeps the coupon.
        let flows = bond
            .pricing_dated_cashflows_at(&market, trade_date, trade_date)
            .expect("flows");
        assert!(
            flows.iter().any(|(d, _)| *d == coupon_date),
            "coupon must be included when the entitlement date is cum-coupon"
        );
    }

    #[test]
    fn validation_rejects_non_finite_call_and_make_whole_quotes() {
        let mut bond = ex_coupon_bond();
        bond.call_put = Some(CallPutSchedule {
            calls: vec![CallPut {
                start_date: date!(2027 - 01 - 01),
                end_date: date!(2027 - 01 - 01),
                price_pct_of_par: f64::NAN,
                make_whole: None,
            }],
            puts: Vec::new(),
        });
        assert!(bond
            .validate()
            .expect_err("NaN call price must fail")
            .to_string()
            .contains("finite"));

        let call = &mut bond.call_put.as_mut().expect("schedule").calls[0];
        call.price_pct_of_par = 101.0;
        call.make_whole = Some(MakeWholeSpec {
            reference_curve_id: CurveId::new("USD-TREASURY"),
            spread_bp: f64::INFINITY,
        });
        assert!(bond
            .validate()
            .expect_err("infinite make-whole spread must fail")
            .to_string()
            .contains("make-whole"));
    }
}
