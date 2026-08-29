//! Unified MarketQuote enum.
//!
//! This module defines the top-level enum for all supported market quotes. The `MarketQuote`
//! enum provides a unified interface for working with quotes across all instrument types,
//! enabling generic calibration workflows and quote processing.

use super::bond::BondQuote;
use super::cds::CdsQuote;
use super::cds_tranche::CDSTrancheQuote;
use super::fx::FxQuote;
use super::inflation::InflationQuote;
use super::rates::RateQuote;
use super::vol::VolQuote;
use super::xccy::XccyQuote;
use finstack_quant_core::{Error, Result};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Polymorphic container for all supported market quote types.
///
/// This enum unifies all quote types into a single type, enabling generic quote processing,
/// serialization, and calibration workflows. Each variant wraps a specific quote type.
///
/// # Examples
///
/// Creating a rates quote:
/// ```rust
/// use finstack_quant_calibration::quotes::market_quote::MarketQuote;
/// use finstack_quant_calibration::quotes::rates::RateQuote;
/// use finstack_quant_calibration::quotes::ids::{Pillar, QuoteId};
/// use finstack_quant_core::types::IndexId;
///
/// # fn example() -> finstack_quant_core::Result<()> {
/// let rate_quote = RateQuote::Deposit {
///     id: QuoteId::new("USD-SOFR-DEP-1M"),
///     index: IndexId::new("USD-SOFR-1M"),
///     pillar: Pillar::Tenor("1M".parse()?),
///     rate: 0.0525,
/// };
///
/// let market_quote = MarketQuote::Rates(rate_quote);
/// # Ok(())
/// # }
/// ```
///
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Bond instruments
    Bond(BondQuote),
    /// Interest rate instruments
    Rates(RateQuote),
    /// Credit default swaps
    Cds(CdsQuote),
    /// CDS Tranches
    #[serde(rename = "cds_tranche")]
    CDSTranche(CDSTrancheQuote),
    /// FX instruments
    Fx(FxQuote),
    /// Inflation instruments
    Inflation(InflationQuote),
    /// Volatility instruments
    Vol(VolQuote),
    /// Cross-currency swap instruments
    Xccy(XccyQuote),
}

impl MarketQuote {
    /// Identifier shared by every quote class.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Bond(quote) => quote.id().as_str(),
            Self::Rates(quote) => quote.id().as_str(),
            Self::Cds(quote) => quote.id().as_str(),
            Self::CDSTranche(quote) => quote.id().as_str(),
            Self::Fx(quote) => quote.id().as_str(),
            Self::Inflation(quote) => quote.id().as_str(),
            Self::Vol(quote) => quote.id().as_str(),
            Self::Xccy(quote) => quote.id().as_str(),
        }
    }

    /// Validate quote-domain invariants before calibration.
    pub fn validate(&self) -> Result<()> {
        if self.id().trim().is_empty() {
            return Err(Error::Validation(
                crate::quotes::ids::EMPTY_QUOTE_ID.to_string(),
            ));
        }

        match self {
            Self::Bond(quote) => validate_bond_quote(quote),
            Self::Rates(quote) => validate_rate_quote(quote),
            Self::Cds(quote) => validate_cds_quote(quote),
            Self::CDSTranche(quote) => validate_cds_tranche_quote(quote),
            Self::Fx(quote) => validate_fx_quote(quote),
            Self::Inflation(quote) => validate_inflation_quote(quote),
            Self::Vol(quote) => quote.validate(),
            Self::Xccy(quote) => validate_xccy_quote(quote),
        }
    }
}

fn validate_finite(value: f64, field: &str) -> Result<()> {
    if !value.is_finite() {
        return Err(Error::Validation(format!(
            "{field} must be finite; got {value}"
        )));
    }
    Ok(())
}

fn validate_positive(value: f64, field: &str) -> Result<()> {
    validate_finite(value, field)?;
    if value <= 0.0 {
        return Err(Error::Validation(format!(
            "{field} must be positive; got {value}"
        )));
    }
    Ok(())
}

fn validate_unit_interval(value: f64, field: &str) -> Result<()> {
    validate_finite(value, field)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(Error::Validation(format!(
            "{field} must be in [0, 1]; got {value}"
        )));
    }
    Ok(())
}

fn validate_rate_quote(quote: &RateQuote) -> Result<()> {
    match quote {
        RateQuote::Deposit { rate, .. }
        | RateQuote::Fra { rate, .. }
        | RateQuote::Swap { rate, .. } => validate_finite(*rate, "rate"),
        RateQuote::Futures {
            price,
            convexity_adjustment,
            ..
        } => {
            validate_finite(*price, "price")?;
            validate_finite(*convexity_adjustment, "convexity_adjustment")
        }
    }
}

fn validate_cds_quote(quote: &CdsQuote) -> Result<()> {
    match quote {
        CdsQuote::CdsParSpread {
            spread_bp,
            recovery_rate,
            ..
        } => {
            validate_positive(*spread_bp, "spread_bp")?;
            validate_unit_interval(*recovery_rate, "recovery_rate")
        }
        CdsQuote::CdsUpfront {
            running_spread_bp,
            upfront_pct,
            recovery_rate,
            ..
        } => {
            validate_positive(*running_spread_bp, "running_spread_bp")?;
            validate_finite(*upfront_pct, "upfront_pct")?;
            validate_unit_interval(*recovery_rate, "recovery_rate")
        }
    }
}

fn validate_cds_tranche_quote(quote: &CDSTrancheQuote) -> Result<()> {
    let CDSTrancheQuote::CDSTranche {
        attachment,
        detachment,
        upfront_pct,
        running_spread_bp,
        ..
    } = quote;
    validate_unit_interval(*attachment, "attachment")?;
    validate_unit_interval(*detachment, "detachment")?;
    if attachment >= detachment {
        return Err(Error::Validation(format!(
            "attachment must be less than detachment; got attachment={attachment}, detachment={detachment}"
        )));
    }
    validate_finite(*upfront_pct, "upfront_pct")?;
    validate_positive(*running_spread_bp, "running_spread_bp")
}

fn validate_fx_quote(quote: &FxQuote) -> Result<()> {
    match quote {
        FxQuote::ForwardOutright { forward_rate, .. } => {
            validate_positive(*forward_rate, "forward_rate")
        }
        FxQuote::SwapOutright {
            near_rate,
            far_rate,
            ..
        } => {
            validate_positive(*near_rate, "near_rate")?;
            validate_positive(*far_rate, "far_rate")
        }
        FxQuote::OptionVanilla { strike, .. } => validate_positive(*strike, "strike"),
    }
}

fn validate_inflation_quote(quote: &InflationQuote) -> Result<()> {
    match quote {
        InflationQuote::InflationSwap { rate, .. }
        | InflationQuote::YoYInflationSwap { rate, .. } => validate_finite(*rate, "rate"),
    }
}

fn validate_xccy_quote(quote: &XccyQuote) -> Result<()> {
    let XccyQuote::BasisSwap {
        basis_spread_bp,
        spot_fx,
        ..
    } = quote;
    validate_finite(*basis_spread_bp, "basis_spread_bp")?;
    if let Some(value) = spot_fx {
        validate_positive(*value, "spot_fx")?;
    }
    Ok(())
}

fn validate_bond_quote(quote: &BondQuote) -> Result<()> {
    match quote {
        BondQuote::FixedRateBulletCleanPrice {
            coupon_rate,
            clean_price_pct,
            ..
        } => {
            validate_finite(*coupon_rate, "coupon_rate")?;
            validate_positive(*clean_price_pct, "clean_price_pct")
        }
        BondQuote::FixedRateBulletZSpread {
            coupon_rate,
            z_spread,
            ..
        } => {
            validate_finite(*coupon_rate, "coupon_rate")?;
            validate_finite(*z_spread, "z_spread")
        }
        BondQuote::FixedRateBulletOas {
            coupon_rate, oas, ..
        } => {
            validate_finite(*coupon_rate, "coupon_rate")?;
            validate_finite(*oas, "oas")
        }
        BondQuote::FixedRateBulletYtm {
            coupon_rate, ytm, ..
        } => {
            validate_finite(*coupon_rate, "coupon_rate")?;
            validate_finite(*ytm, "ytm")
        }
    }
}

/// Trait for filtering quote collections into specific types (owned).
pub(crate) trait ExtractQuotes<T> {
    fn extract_quotes(&self) -> Vec<T>;
}

macro_rules! impl_extract_quotes {
    ($quote_type:ty, $variant:ident) => {
        impl ExtractQuotes<$quote_type> for [MarketQuote] {
            fn extract_quotes(&self) -> Vec<$quote_type> {
                self.iter()
                    .filter_map(|q| match q {
                        MarketQuote::$variant(inner) => Some(inner.clone()),
                        _ => None,
                    })
                    .collect()
            }
        }
    };
}

impl_extract_quotes!(RateQuote, Rates);
impl_extract_quotes!(CdsQuote, Cds);
impl_extract_quotes!(CDSTrancheQuote, CDSTranche);
impl_extract_quotes!(InflationQuote, Inflation);
impl_extract_quotes!(XccyQuote, Xccy);
