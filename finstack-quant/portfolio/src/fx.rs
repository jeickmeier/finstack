//! Portfolio-level FX conversion helpers.
//!
//! This module centralizes the "convert a `Money` amount into the portfolio
//! base currency using the FX matrix from a `MarketContext`" pattern that is
//! otherwise duplicated across valuation, metrics, attribution, margin, and
//! cashflow aggregation. Callers that need the same behaviour should call
//! [`convert_to_base`] rather than re-implementing the FxMatrix lookup and
//! error mapping.
//!
//! The implementation intentionally stays narrow:
//!
//! - Same-currency inputs short-circuit without consulting the FX matrix.
//! - Missing FX matrices surface as [`Error::MissingMarketData`].
//! - Missing rates for a specific pair surface as [`Error::FxConversionFailed`].
//!
//! [`convert_to_base`] is the spot path (NAV, metrics, attribution, factor
//! endpoints). [`convert_to_base_forward`] wraps that spot lookup and applies
//! covered-interest-parity forwards for cashflows with `payment_date > as_of`.
//! Do not re-implement matrix lookup in those wrappers.

use std::collections::HashMap;

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::fx::FxQuery;
use finstack_quant_core::money::Money;
use finstack_quant_core::types::CurveId;

use crate::error::{Error, Result};

/// Discount factors at or below this magnitude are treated as missing/zero.
const DF_NEAR_ZERO: f64 = 1e-14;

/// Convert a monetary amount into `base_currency` using the market FX matrix.
///
/// Returns the input unchanged when it is already denominated in `base_currency`.
///
/// # Arguments
///
/// * `amount` - Monetary amount to convert.
/// * `as_of` - Date used for the FX rate lookup.
/// * `market` - Market context supplying the FX matrix.
/// * `base_currency` - Target reporting currency.
///
/// # Errors
///
/// * [`Error::MissingMarketData`] - The market context has no FX matrix.
/// * [`Error::FxConversionFailed`] - The requested currency pair is not
///   available in the FX matrix.
pub fn convert_to_base(
    amount: Money,
    as_of: Date,
    market: &MarketContext,
    base_currency: Currency,
) -> Result<Money> {
    if amount.currency() == base_currency {
        return Ok(amount);
    }

    let fx_matrix = market
        .fx()
        .ok_or_else(|| Error::MissingMarketData("FX matrix not available".to_string()))?;

    let query = FxQuery::new(amount.currency(), base_currency, as_of);
    let rate_result = fx_matrix
        .rate(query)
        .map_err(|_| Error::FxConversionFailed {
            from: amount.currency(),
            to: base_currency,
        })?;

    Ok(Money::new(
        amount.amount() * rate_result.rate,
        base_currency,
    ))
}

/// Convert a dated cashflow into `base_currency` using spot or a CIP forward.
///
/// Same-currency amounts are returned unchanged. Payments on or before `as_of`
/// use [`convert_to_base`] at `as_of` (spot). Later payments use the covered
/// interest-rate parity forward
/// `F(T) = S × DF_from(T) / DF_base(T)`, matching
/// [`finstack_quant_valuations::instruments::fx::fx_forward::FxForward::market_forward_rate`].
/// `from` is the cashflow currency.
///
/// Discount curves are resolved from `discount_curves` when that map contains
/// the currency; otherwise from `market.get_discount(currency.to_string())`.
/// Missing curves or missing/zero discount factors fail closed.
///
/// # Arguments
///
/// * `amount` - Cashflow amount in its native currency.
/// * `as_of` - Valuation date: spot FX is observed here, and discount factors
///   are measured from this date to `payment_date`.
/// * `payment_date` - Cashflow payment date. On or before `as_of` selects
///   spot; after `as_of` selects the CIP forward.
/// * `market` - Market context supplying the FX matrix and discount curves.
/// * `base_currency` - Target reporting currency (`DF_base` in the CIP formula).
/// * `discount_curves` - Optional `Currency → CurveId` overrides. A missing
///   map or missing currency entry falls back to the ISO currency code as
///   the discount-curve identifier. There is no silent fallback to another
///   curve in the same currency (for example `USD-OIS`).
///
/// # Errors
///
/// * [`Error::MissingMarketData`] - FX matrix, discount curve, or a usable
///   discount factor is missing.
/// * [`Error::FxConversionFailed`] - The spot pair is not in the FX matrix.
/// * [`Error::Core`] - Discount-factor evaluation failed for a resolved curve.
pub fn convert_to_base_forward(
    amount: Money,
    as_of: Date,
    payment_date: Date,
    market: &MarketContext,
    base_currency: Currency,
    discount_curves: Option<&HashMap<Currency, CurveId>>,
) -> Result<Money> {
    if amount.currency() == base_currency {
        return Ok(amount);
    }

    if payment_date <= as_of {
        return convert_to_base(amount, as_of, market, base_currency);
    }

    let spot_converted = convert_to_base(amount, as_of, market, base_currency)?;
    let df_from = discount_factor(
        market,
        amount.currency(),
        as_of,
        payment_date,
        discount_curves,
    )?;
    let df_base = discount_factor(market, base_currency, as_of, payment_date, discount_curves)?;

    Ok(Money::new(
        spot_converted.amount() * (df_from / df_base),
        base_currency,
    ))
}

fn discount_curve_id(
    currency: Currency,
    discount_curves: Option<&HashMap<Currency, CurveId>>,
) -> String {
    discount_curves
        .and_then(|map| map.get(&currency))
        .map(ToString::to_string)
        .unwrap_or_else(|| currency.to_string())
}

fn discount_factor(
    market: &MarketContext,
    currency: Currency,
    as_of: Date,
    payment_date: Date,
    discount_curves: Option<&HashMap<Currency, CurveId>>,
) -> Result<f64> {
    let id = discount_curve_id(currency, discount_curves);
    let curve = market.get_discount(&id).map_err(|_| {
        Error::MissingMarketData(format!(
            "no discount curve for {currency} (looked up as '{id}')"
        ))
    })?;
    let df = curve
        .df_between_dates(as_of, payment_date)
        .map_err(Error::Core)?;
    if !df.is_finite() || df.abs() < DF_NEAR_ZERO {
        return Err(Error::MissingMarketData(format!(
            "discount factor for {currency} from {as_of} to {payment_date} is missing or zero (df={df})"
        )));
    }
    Ok(df)
}
